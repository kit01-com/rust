# msrcget

Knjižnica za večvirni segmentni prenos po HTTP/HTTPS, ki temelji na asinhrnem modelu dokončanja [compio](https://crates.io/crates/compio), namenjena reševanju omejitev hitrosti enojnega zrcala, okvare podatkovnih segmentov in problema z počasnim repom.

## Ocena zmogljivosti

Primerjava prenosne hitrosti knjižnic [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) in [ureq](https://crates.io/crates/ureq) na javnih HTTPS zrcalih:

### Scenarij 1: Agregirani prenos s številnih javnih zrcel

Prenos izdane datoteke Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB), zaporedje zrcel: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (vključno s preklopi prek 302 med zrcali).

| Knjižnica / rešitev | Prenosna hitrost (MB/s) |
| --- | ---: |
| msrcget (5 virov, 8 sovrstnih) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 virov) | 1.2 |
| cyper (eden vir) | 14.4 |
| reqwest (eden vir) | 14.4 |
| ureq (eden vir) | 14.3 |

### Scenarij 2: Mešan prenos hitrih in počasnih virov

Prenos paketa glavih datotek Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8,4 MB), zaporedje zrcel: Alibaba Cloud (omejena hitrost na povezavo), Tsinghua, Huawei Cloud, Tencent Cloud, uradno skladišče Node.js (mejnoprečno počasno vozlišče).

| Knjižnica / rešitev | Prenosna hitrost (MB/s) |
| --- | ---: |
| msrcget (5 virov, 8 sovrstnih) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 virov) | 0.2 |
| cyper (eden vir) | 6.8 |
| reqwest (eden vir) | 6.6 |
| ureq (eden vir) | 6.7 |

### Metoda testiranja

Koda za oceno je v `examples/bench.rs` (izvedba `cargo run --profile bench --example bench`):

- Priprava osnove: Pred meritvijo časa se ciljno datoteko enkrat prednese, preveri se dolžina v bajtih in izračunajo osnovne vrednosti CRC32 za celotno datoteko ter posamezne segmente; ta faza se ne šteje v čas.
- Meje meritve: Vsaka rešitev se neodvisno izvede 3 zaporedne kroge; pred začetkom vsakega kroga se izbrišejo izhodna datoteka in začasna `.part` datoteka; en krog se meri od izdaje omrežnega zahtevka do dokončanja pisanja datoteke na disk (`cyper`, `reqwest`, `ureq` izrecno izvedejo `sync_data` za izpust na disk; `msrcget` vključuje preiskanje oddaljene dolžine, ustvarjanje razredčene datoteke, sovrstni segmentni prenos, `sync_data` izpust na disk, odrezanje končnih metapodatkov ter atomsko preimenovanje).
- Preverjanje rezultata: Po koncu meritve vsakega kroga se zapisana datoteka prebere v strmem načinu, preveri se številko bajtov datoteke in CRC32 celotnega paketa; postopek preverjanja se ne šteje v čas prenosa.
- Preračun pasovne širine: Vzemete mediano iz treh meritev časa in izračunate prenosno hitrost po formuli `število bajtov datoteke / 1048576 / medianska sekunda` (`1 MB = 1048576 bajtov`).
- Pravila dodelitve virov: Večvirne rešitve (`msrcget`, `down`) prejmejo vsa 5 naslovov zrcel v scenariju; enovirske rešitve (`cyper`, `reqwest`, `ureq`) ne podpirajo večvirnih segmentov in uporabijo prvi naslov zrcela na seznamu zahtevkov.

## Značilnosti

- Preverjanje po segmentih in preklapljanje ob napaki: Poklicatelj preda vmesnik `Verify` za preverjanje vsakega segmenta; če preverjanje ni uspelo ali omrežni zahtevek ni uspel, se samodejno preklaplja na druga netestirana zrcala; če po preizkusu vseh virov ne uspe, se vrne `Error::Exhausted`.
- Razredčene datoteke in nadaljevanje prenosa: Po skupni dolžini datoteke se vnaprej ustvari razredčena začasna `.part` datoteka in doda zaklep datoteke (konflikt zaklepa vrne `Error::Locked`); na koncu datoteke se zapiše CRC32 bitno mapo s podvojenimi preti za ohranitev dokončanih segmentov, po prekinitvi in ponovnem zagonu se samodejno nadaljuje, po dokončanju vsega se odrežejo končni metapodatki in se atomsko preimenuje v ciljno datoteko.
- Deljenje nalog: Vir, ki prvi dokonča svoj interval, največje preostale nedokončane razdalje prereže na polovici in prevzame drugi del, da izravnava breme med zrcali.
- Končna tekma več poti: Ko je število prostih sovrstnikov večje od števila preostalih nedokončanih segmentov, se za segmente, ki jih zrcalo še ni poskusilo, pošljejo dodatni zahtevki; ko prvi preverjeni podatki padejo na disk, se takoj prekinejo ostali zahtevki za isti segment.
- Samodejno načrtovanje segmentov: Velikost segmentov se načrtuje glede na skupno dolžino datoteke in število sovrstnikov; pri nadaljevanju obstoječe `.part` datoteke se samodejno uporabi velikost segmenta, zabeležena v njej.
- Ponovna uporaba povezav in podpora protokolom: Vgrajeni povod povezav za isti vir ponovno uporablja HTTP/1.1 dolge povezave; podpira segmentno prenosno kodiranje, HTTPS ([rustls](https://crates.io/crates/rustls)), preklope po potih in med zrcali (`301`/`302`/`303`/`307`/`308`) ter upravljanje časa do izteka.

## Demo uporabe

Primer koda je v `examples/basic.rs` (izvedba `cargo run --example basic`):

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

/// Preverjalnik po segmentih na podlagi CRC32
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

  // Vir 0 vrne poškodovane podatke v prvem segmentu, vir 1 hitro in pravilno, vir 2 počasno vozlišče z visoko zakasnitvijo
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

  // Privzeti večvirni sovrstni prenos: samodejno načrtovanje velikosti segmentov, pisanje razredčene .part datoteke, odrezanje in preimenovanje
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Večvirni prenos s CRC32 preverjanjem po segmentih: pokvarjeni segmenti samodejno preklopijo vir, hitri viri samodejno prevzamejo del in tekmujejo s počasnim na koncu
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

## Opis vmesnikov

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Prenese `url_li` v `path` s privzetim številom sovrstnikov `CONC` (`8`) in privzetim enkratnim časom do izteka `TIMEOUT` (`30 s`), vrne skupno število bajtov datoteke.

### `Get`

Vhodno mesto gradbenika `Get::new(url_li, path)`, podpira veržno konfiguracijo:

- `.size(size: u64)`: Izrecno določi znano skupno število bajtov datoteke in preskoči preiskanje oddaljene dolžine.
- `.chunk_size(chunk_size: usize)`: Določi število bajtov posameznega segmenta (posled `0` vrne `Error::ChunkSize`).
- `.conc(conc: usize)`: Določi število sovrstnikov za prenos.
- `.timeout(timeout: Duration)`: Določi čas do izteka za enkratni zahtevek in branje.
- `.verify<V2: Verify>(verify: V2)`: Vstavi preverjalnik; implementirano za `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` in `Arc<T>`.
- `.run().await`: Zažene prenos in vrne `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` je vrstna številka segmenta od 0, `data` je bačni rezin tega segmenta; `true` pomeni uspešno preverjanje.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Vrne velikost segmenta kot potenca števila 2 med `MIN_CHUNK` (`64 KB`) in `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Pripere konstanto za naziv `PART_EXT` (`".part"`) in vrne pot do začasne datoteke za nadaljevanje prenosa.

### `Error` in `Result`

Edinsten povrnjen tip `Result<T> = std::result::Result<T, Error>`; variante napačnega sešteva vključujejo: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
