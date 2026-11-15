# msrcget

Višestrukoizvorišna, po-dijelovima orijentirana HTTP/HTTPS biblioteka za preuzimanje, zasnovana na asinhrom modelu završavanja [compio](https://crates.io/crates/compio), namijenjena rješavanju ograničenja propusnosti pojedinačnih ogledala, oštećenja podataka u segmentima i usporavanja uslijed sporih čvorova.

## Procjena performansi

Usporedba [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) i [ureq](https://crates.io/crates/ureq) po propusnosti preuzimanja putem javnih HTTPS ogledala:

### Scenarij 1: Agregirano preuzimanje s više javnih ogledala

Preuzimanje Node.js v22.14.0 distribucijskog paketa (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), redoslijed ogledala: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (uključuje 302 međusedno preusmjeravanje).

| Biblioteka / rješenje | Propusnost (MB/s) |
| --- | ---: |
| msrcget (5 izvora, 8 paralelnih) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 izvora) | 1.2 |
| cyper (jedan izvor) | 14.4 |
| reqwest (jedan izvor) | 14.4 |
| ureq (jedan izvor) | 14.3 |

### Scenarij 2: Miješano preuzimanje brzi i spor izvori

Preuzimanje Node.js v22.14.0 paketa zaglavlja (`node-v22.14.0-headers.tar.gz`, 8.4 MB), redoslijed ogledala: Alibaba Cloud (ograničenje na jednu vezu), Tsinghua, Huawei Cloud, Tencent Cloud, službeni Node.js izvor (spor čvor preko granice).

| Biblioteka / rješenje | Propusnost (MB/s) |
| --- | ---: |
| msrcget (5 izvora, 8 paralelnih) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 izvora) | 0.2 |
| cyper (jedan izvor) | 6.8 |
| reqwest (jedan izvor) | 6.6 |
| ureq (jedan izvor) | 6.7 |

### Metodologija testiranja

Kod za procjenu nalazi se u `examples/bench.rs` (pokretanje: `cargo run --profile bench --example bench`):

- Priprema baze: Prije mjerenja vremena ciljna datoteka se unaprijed preuzme jednom, provjeri se broj bajtova i izračuna CRC32 za cijelu datoteku i po segmentima; ova faza se ne računa u vrijeme.
- Granice mjerenja: Svaki skup rješenja nezavisno se kontinuirano izvršava 3 puta; prije početka svakog prolaza brišu se izlazna datoteka i privremene `.part` datoteke; mjerenje počinje inicijalnim mrežnim zahtjevom i završava potpunim zapisivanjem datoteke na disk (`cyper`, `reqwest` i `ureq` eksplicitno izvršavaju `sync_data` flush; `msrcget` obuhvaća detekciju daljinske dužine, kreiranje rijetke datoteke, paralelno preuzimanje segmenata, `sync_data` flush, trunciranje repne metapodaci i atomsku promjenu naziva).
- Validacija proizvoda: Nakon završetka mjerenja svakog prolaza čita se zapisana datoteka u toku i provjerava se broj bajtova i CRC32 cijele datoteke; proces validacije se ne računa u vrijeme preuzimanja.
- Pretvorba propusnosti: Uzima se medijana vremena iz 3 prolaza, a propusnost se izračunava kao `broj bajtova datoteke / 1048576 / medijana sekundi` (`1 MB = 1048576 bajtova`).
- Pravila dodijele izvora: Višestrukoizvorišna rješenja (`msrcget`, `down`) primaju sva 5 adresa ogledala iz scenarija; jednoizvorišna rješenja (`cyper`, `reqwest`, `ureq`) ne podržavaju višestrukoizvorišno segmentiranje, pa se koristi prva adresa ogledala iz liste.

## Karakteristike

- Provjeravanje po segmentu i automatski prekid: Poziva se provjera kroz `Verify` interfejs za svaki segment; ako provjera ne prođe ili mrežni zahtjev ne uspije, automatski se prebacuje na preostala nepokrenuta ogledala za ponovni pokušaj; ako svi izvori ne uspiju, vraća se `Error::Exhausted`.
- Rijetke datoteke i nastavak preuzimanja: Napredno se stvara rijedaka `.part` privremena datoteka prema ukupnoj dužini datoteke i dodjeljuje se zaključak datoteke (sudar zaključaka vraća `Error::Locked`); na kraju datoteke upisuje se dupli CRC32 bitmap za trajno spremanje završenih segmenata, nakon prekida i ponovnog pokretanja automatski se nastavlja preuzimanje, a po završetku svega truncira se repna metapodaci i atomska promjena naziva na ciljanu datoteku.
- Binarna krađa zadataka: Izvor koji prvi završi preuzimanje svog raspona dijeli najveći preostali nezavršeni raspon na pola i preuzima drugi dio, čime se balansira opterećenje između ogledala.
- Višeputni trk za repne segmente: Kada je broj praznih ko-rutina veći od broja preostalih nezavršenih segmenata, šalju se redundanthi zahtjevi prema ogledalima koja još nisu pokušala taj segment; čim prvi podatak prođe provjeru i bude zapisan na disk, ostali zahtjevi za isti segment se odmah otkazuju.
- Automatsko planiranje segmenata: Veličina segmenata planira se prema ukupnoj dužini datoteke i broju paralelizma; kod nastavka postojeće `.part` datoteke automatski se koristi veličina segmenata zabilježena u njoj.
- Ponovna upotreba veza i podrška protokola: Ugrađen isti-izvor povezivač za ponovnu upotrebu HTTP/1.1 dugih veza, podržava transfer encoding po segmentima, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` preusmjeravanje između putanji i izvora te kontrolu isteka vremena.

## Demonstracija upotrebe

Primjer kod nalazi se u `examples/basic.rs` (pokretanje: `cargo run --example basic`):

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

/// Provjeravač po segmentima zasnovan na CRC32
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

  // Izvor 0 prvi segment vraća oštećene podatke, izvor 1 brzi normalni, izvor 2 visok-latencijski spor čvor
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

  // Zadano višestrukoizvorišno paralelno preuzimanje: automatsko planiranje veličine segmenata, pisanje rijetke .part zatim trunciranje i promjena naziva
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Višestrukoizvorišno preuzimanje s CRC32 provjerom po segmentu: oštećeni segmenti automatski prebacuju izvor, brzi izvori automatski kradu i trke repne segmente s sporim izvorom
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

## Dokumentacija interfejsa

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Koristi zadani broj paralelizma `CONC` (`8`) i zadani timeout po zahtjevu `TIMEOUT` (`30s`) za preuzimanje `url_li` na `path`, vraća ukupan broj bajtova datoteke.

### `Get`

Ulazni bod buildera `Get::new(url_li, path)`, podržava lance konfiguracija:

- `.size(size: u64)`: Eksplicitno zadano ukupan broj bajtova datoteke, preskače detekciju daljinske dužine.
- `.chunk_size(chunk_size: usize)`: Zada broj bajtova po segmentu (proširenje `0` vraća `Error::ChunkSize`).
- `.conc(conc: usize)`: Zada broj paralelnih ko-rutina za preuzimanje.
- `.timeout(timeout: Duration)`: Zada timeout za zahtjev i čitanje.
- `.verify<V2: Verify>(verify: V2)`: Ubacuje provjeravajući mehanizam; implementirano za `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` i `Arc<T>`.
- `.run().await`: Pokreće preuzimanje i vraća `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` je 0-bazni indeks segmenta, `data` je bajtovni isječak tog segmenta, vraćanje `true` označava uspješnu provjeru.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Vraća bajtovnu veličinu segmenta kao potenciju od 2 između `MIN_CHUNK` (`64 KB`) i `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Dodaje sufiks konstantu `PART_EXT` (`".part"`), vraća putanju privremene datoteke za nastavak preuzimanja.

### `Error` i `Result`

Jedinstveni povratni tip `Result<T> = std::result::Result<T, Error>`, varijante pogreške uključuju: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
