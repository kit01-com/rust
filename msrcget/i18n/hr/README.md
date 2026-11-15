# msrcget

Knjižnica za višezorno blokno preuzimanje HTTP/HTTPS temeljena na asinkronom modelu dovršetka [compio](https://crates.io/crates/compio), namijenjena rješavanju ograničenja brzine na pojedinim ogledalima, oštećenja podataka u blokovima i odgađanja zbog sporih čvorova.

## Procjena performansi

Usporedba propusnosti [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) i [ureq](https://crates.io/crates/ureq) na javnim HTTPS ogledalima:

### Scenarij 1: Agregirano preuzimanje s više javnih ogledala

Preuzimanje Node.js v22.14.0 distribucijskog paketa (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), ogledala u nizu: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (uključuje 302 prekretanje između izvora).

| Knjižnica / rješenje | Propusnost (MB/s) |
| --- | ---: |
| msrcget (5 izvora, 8 paralelnih) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 izvora) | 1.2 |
| cyper (jedan izvor) | 14.4 |
| reqwest (jedan izvor) | 14.4 |
| ureq (jedan izvor) | 14.3 |

### Scenarij 2: Miješano preuzimanje s brzim i sporim izvorima

Preuzimanje Node.js v22.14.0 paketa zaglavlja (`node-v22.14.0-headers.tar.gz`, 8.4 MB), ogledala u nizu: Alibaba Cloud (ograničenje po vezi), Tsinghua, Huawei Cloud, Tencent Cloud, zvanični Node.js izvor (spori međunarodni čvor).

| Knjižnica / rješenje | Propusnost (MB/s) |
| --- | ---: |
| msrcget (5 izvora, 8 paralelnih) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 izvora) | 0.2 |
| cyper (jedan izvor) | 6.8 |
| reqwest (jedan izvor) | 6.6 |
| ureq (jedan izvor) | 6.7 |

### Metoda testiranja

Kôd procjene nalazi se u `examples/bench.rs` (pokrenuti s `cargo run --profile bench --example bench`):

- Priprema bazne linije: prije mjerenja vremena ciljani datoteka se preuzme unaprijed, provjeri se dužina u bajtovima i izračunaju CRC32 bazne vrijednosti za cijeli paket i pojedinačne blokove; ta faza se ne uračunava u vrijeme.
- Granice mjerenja vremena: svako rješenje kontinuirano i neovisno pokreće 3 kruga; prije početka svakog kruga brišu se izlazna datoteka i privremena `.part` datoteka. Mjerenje u jednom krugu počinje s pokretanjem mrežnog zahtjeva i završava se dovršetkom pisanja datoteke na disk (`cyper`, `reqwest` i `ureq` eksplicitno izvršavaju `sync_data` za ispis na disk; `msrcget` obuhvaća cijeli proces: detekciju udaljene duljine, kreiranje retkovite datoteke, paralelno blokno preuzimanje, `sync_data` ispis na disk, orezivanje meta podataka na kraju i atomsko preimenovanje).
- Provjera rezultata: nakon završka mjerenja svakog kruga zapisana datoteka se čita u toku, provjerava se dužina datoteke u bajtovima i CRC32 cijelog paketa; proces provjere se ne uračunava u vrijeme preuzimanja.
- Pretvorba propusnosti: uzima se medijan vremena iz 3 kruga, a propusnost se izračunava kao `broj_bajtova_datoteke / 1048576 / medijan_u_sekundama` (`1 MB = 1048576 bajtova`).
- Pravila dodjele izvora: višezorna rješenja (`msrcget`, `down`) primaju svih 5 adresi ogledala iz scenarija; jednozorna rješenja (`cyper`, `reqwest`, `ureq`) ne podržavaju višezorno blokno preuzimanje, pa se zatražuje prva adresa ogledala iz popisa.

## Značajke

- Provjeravanje blok po blok s automatskim prebacivanjem: pozivnik prenosi `Verify` sučelje za provjeru svakog bloka; ako provjera nije uspješna ili mrežni zahtjev nije uspio, automatski se preklopi na preostale neiskorištene izvore ogledala za ponovni pokušaj, a ako ne uspije ni nakon obila svih izvora, vraća se `Error::Exhausted`.
- Retkovite datoteke i prekidanje s nastavkom: prema ukupnoj duljini datoteke unaprijed se kreira retkovita privremena `.part` datoteka s zaključavanjem datoteke (sudar zaključanja vraća `Error::Locked`); na kraju datoteke upisuje se dvoslotna ping-pong CRC32 mapa bitova za trajno spremanje dovršenih blokova, pri prekidu i ponovnom pokretanju automatski se nastavlja, nakon dovršetka svih blokova oreže se meta podatak na kraju i atomski se preimenuje u ciljanu datoteku.
- Binarna krađa zadataka: izvor koji prvi dovrši vlastiti interval dijeli preostali interval s najvećim nedovršenim rasponom na polovinu i preuzima drugu polovinu, čime se balansira opterećenje svih izvora ogledala.
- Višestruko trkanje na kraju: kada broj praznih koregona premašuje broj preostalih nedovršenih blokova, posalju se rezervni zahtjevi izvorima ogledala koji još nisu probali taj blok; čim prvi podatak koji je prošao provjeru bude upisan na disk, odmah se otkažu ostali zahtjevi za isti blok.
- Automatsko planiranje blokova: veličina bloka planira se prema ukupnoj duljini datoteke i broju paralelnih; kod nastavka preuzimanja postojeće `.part` datoteke automatski se koristi veličina bloka zabilježena u njoj.
- Ponovna upotreba veza i podrška protokolima: ugrađeni bazen veza za isti izvor ponovno koristi HTTP/1.1 dugačke veze, podržava enkodiranje u blokovima za prijenos, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` prekretanje kroz putanje i izvore te upravljanje s istekom vremena.

## Demo upotrebe

Primjer kôda nalazi se u `examples/basic.rs` (pokrenuti s `cargo run --example basic`):

```rust
mod mock;

use std::{
  env::temp_dir,
  fs::{metadata, remove_file},
  sync::Arc,
  time::Duration,
};

use crc32fast::Hasher;
use mock::start_mirror;
use msrcget::{Get, Result, Verify, get, part_path, plan_chunk_size};

/// Provjeritelj blok po blok temeljen na CRC32
struct CrcVerify(Vec<u32>);

impl Verify for CrcVerify {
  fn verify(&self, idx: usize, data: &[u8]) -> bool {
    let mut hasher = Hasher::new();
    hasher.update(data);
    self.0.get(idx).copied() == Some(hasher.finalize())
  }
}

#[compio::main]
async fn main() -> Result<()> {
  let chunk_size = 64 * 1024;
  let data: Arc<[u8]> = (0usize..chunk_size * 8 + 512)
    .map(|i| (i.wrapping_mul(31) & 0xff) as u8)
    .collect::<Vec<_>>()
    .into();

  // Izvor 0 u prvom bloku vraća oštećene podatke, izvor 1 radi brzo i ispravno, izvor 2 je spori čvor s visokom kašnjenjem
  let u0 = start_mirror(Arc::clone(&data), chunk_size, Some(1), Duration::ZERO).await;
  let u1 = start_mirror(Arc::clone(&data), chunk_size, None, Duration::ZERO).await;
  let u2 = start_mirror(
    Arc::clone(&data),
    chunk_size,
    None,
    Duration::from_millis(200),
  )
  .await;

  let out_a = temp_dir().join("msrcget_demo_get.bin");
  let out_b = temp_dir().join("msrcget_demo_verify.bin");
  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  let _ = remove_file(part_path(&out_a));
  let _ = remove_file(part_path(&out_b));

  // Zadano višezorno paralelno preuzimanje: automatsko planiranje veličine blokova, pisanje retkovite .part datoteke, orezivanje i preimenovanje
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Višezorno preuzimanje s CRC32 provjerom blok po blok: oštećeni bloci automatski prebacuju na drugi izvor, brzi izvori automatski krade zadatke i trka na kraju protiv sporog izvora
  let crc_li: Vec<u32> = data
    .chunks(chunk_size)
    .map(|chunk| {
      let mut h = Hasher::new();
      h.update(chunk);
      h.finalize()
    })
    .collect();

  let size_b = Get::new([&u0, &u1, &u2], &out_b)
    .chunk_size(chunk_size)
    .conc(4)
    .verify(CrcVerify(crc_li))
    .run()
    .await?;

  assert_eq!(metadata(&out_b)?.len(), size_b);
  println!("Get::verify() 校验并完成: {size_b} 字节");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Dokumentacija sučelja

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Korištenjem zadane razine paralelnosti `CONC` (`8`) i zadanoog isteka vremena po zahtjevu `TIMEOUT` (`30 s`) preuzima `url_li` u `path` i vraća ukupan broj bajtova datoteke.

### `Get`

Ulazni konstrukt `Get::new(url_li, path)`, podržava lancano podešavanje:

- `.size(size: u64)`: eksplicitno odredi ukupan broj bajtova datoteke, preskaši detekciju udaljene duljine.
- `.chunk_size(chunk_size: usize)`: odredi broj bajtova po bloku (prosljeđivanje `0` vraća `Error::ChunkSize`).
- `.conc(conc: usize)`: odredi broj paralelnih koregona za preuzimanje.
- `.timeout(timeout: Duration)`: odredi istek vremena po zahtjevu i čitanju.
- `.verify<V2: Verify>(verify: V2)`: ugradi provjeritelj, implementiran za `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` i `Arc<T>`.
- `.run().await`: pokrene preuzimanje i vrati `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` je 0-bazni redni broj bloka, `data` je rez bajtova tog bloka, vraćanje `true` označava uspješnu provjeru.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Vraća veličinu bloka u bajtovima kao potenciju broja 2 između `MIN_CHUNK` (`64 KB`) i `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Pridruži konstantu sufixa `PART_EXT` (`".part"`), vrati putanju privremene datoteke za nastavak preuzimanja.

### `Error` i `Result`

Jedinstveni tip povratka `Result<T> = std::result::Result<T, Error>`, varijante pogrešaka uključuju: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
