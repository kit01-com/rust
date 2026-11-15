# msrcget

Biblioteka do pobieraniōa HTTP/HTTPS po kawałkach ôd wielu zêrkiôd, oparta na asynchronicznym modelu ukończyniōa [compio](https://crates.io/crates/compio); służy do ôbécyowaniōa ôgraniczeniōa prędkości jyngo zêrkiôda, uszkodzeniōa danych w kawałkach ôraz spōlniowaniōa wolnykůch węzłów.

## Pomiary wydolności

Porôwnanie przepuściwności pobieraniōa ôd publicznykůch zêrkiôd HTTPS miyndzy [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) ôraz [ureq](https://crates.io/crates/ureq):

### Scenôrz 1: Agregowane pobieraniy ôd wielu publicznykůch zêrkiôd

Pobieraniy pakietu wydaniowoego Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), zêrkiôdy w kolejności: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (z przekierowaniym 302 miyndzy zêrkiôdama).

| Biblioteka / Metoda | Przepuściwność (MB/s) |
| --- | ---: |
| msrcget (5 zêrkiôd, 8 równoległych) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 zêrkiôd) | 1.2 |
| cyper (1 zêrkiôd) | 14.4 |
| reqwest (1 zêrkiôd) | 14.4 |
| ureq (1 zêrkiôd) | 14.3 |

### Scenôrz 2: Pobieraniy ze zmiesanych szybkich ôraz wolnykůch zêrkiôd

Pobieraniy pakietu nagłówkowykůch Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8.4 MB), zêrkiôdy w kolejności: Alibaba Cloud (ôgraniczono prędkość ôd jynego połączeniōa), Tsinghua, Huawei Cloud, Tencent Cloud, oficjalny zêrkiôd Node.js (wolny węzeł transgraniczny).

| Biblioteka / Metoda | Przepuściwność (MB/s) |
| --- | ---: |
| msrcget (5 zêrkiôd, 8 równoległych) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 zêrkiôd) | 0.2 |
| cyper (1 zêrkiôd) | 6.8 |
| reqwest (1 zêrkiôd) | 6.6 |
| ureq (1 zêrkiôd) | 6.7 |

### Metoda testowaniōa

Kod pomiarowy sie ôbyje w `examples/bench.rs` (uruchomienie: `cargo run --profile bench --example bench`):

- Prziôgotowaniy bazowy: przed pomiaram czasu niy piersy pobierze sie celowy plik, zweryfikuje dłużstość bajtowo ôraz obliczy bôzowy CRC32 dlo całości ôraz kōżdgo kawałka; to etapy sie niy wliczo do czasu.
- Granice czasowaniōa: kōżda metoda sie sprowadzo kōntynuacyjnie 3 razy ôd osôbny; przed kōżdym rundym ôdymô ôbiegowy plik ôraz tymczasowy plik `.part`; czas ôd pierszego ôdêsłaniōa ôdzytu sieciowo do zôpisaniōa pliku na dysku (`cyper`, `reqwest`, `ureq` — wsiystki wycônyjajům `sync_data`; `msrcget` — cały proces: ôdpytaniyô dalowy dłużstości, ôdmiarka rzadki plik, równoległy pobier po kawałkach, `sync_data`, ciyniyniyniyni metadanych ôd kōńca ôraz ôtomatyczne ôbmieniyaniy nazwy).
- Weryfikacyjo wynikuw: ôd ôstawieniōa ôd kōżdym rundym plik na dysku sie czyta strumieniowo, weryfikuje sie dłużstość bajtowo ôraz CRC32 całości; weryfikacyjo sie niy wliczo do czasu pobieraniōa.
- Ôbliczeniye przepuściwności: bierze sie medyanną czasuw 3 rund, przepuściwność sie ôbliczo ôd `liczba bajtów pliku / 1048576 / medyana w sekundach` (`1 MB = 1048576 bajtów`).
- Zasady ôd przydziału zêrkiôd: warianty wieluzêrkiôdowe (`msrcget`, `down`) bierům wsiystkie 5 adresuw zêrkiôdów ze scenôrza; warianty jednozêrkiôdowe (`cyper`, `reqwest`, `ureq`) niy wskałujům wieluzêrkiôdowego pobieraniōa po kawałkach — proszo ôd pierszego adresu ze listy.

## Właściwości

- Weryfikacyjo kawałek po kawałku ôraz przëcygniyniyniyni na ôwyjęki: ôdwołujoncy wstawia interfejs `Verify` do weryfikowaniōa kōżdgo kawałka; jak weryfikacyjo sie niy ôdli czy prosziy siec ôdbyw sie niy dale, automatycznie sie przëcygniyniyniyni do jynego, niyprzëdôłgowego zêrkiôda; jak wsiystkie zêrkiôdy niy dadzům rady, wrôca `Error::Exhausted`.
- Rzadki pliki ôraz wznowińiyni pobieraniōa: ôd dłużstości pliku z ôd gôrdy tworzizy sie rzadki tymczasowy plik `.part` ôraz ôdmiarka plik (kolizyjô klucza wrôca `Error::Locked`); ôd kōńca pliku zapisizy sie podwôjny pinge-pongowy bitmapa CRC32 do ôdchowaniōa ukończonykůch kawałków; ôd przerwanio ôd startu sie automatycznie wznowi, ôd ukończeniu ciynie sie metadane ôd kōńca ôraz atomowo sie obmiyniyo nazwa na celowy plik.
- Dwudzielnje kradzyńiyniyni zadań: zêrkiôd, kiery piersy dokończy swoj ôbszar, cieŏnie niyukończony ôd naibielszy dłuży ôbszar na dwa ôdzym ôraz przejmuje ôdym; to wypoziomowuje obciążeniy miyndzy zêrkiôdama.
- Wielotorowy gonit na kōńcu: jak wiyncy wolnykůch koreptyniyjów ôstoło niyukończonykůch kawałków, sie ôdêsylajům ôddatne prosze do zêrkiôdów, kierych niy prôbowali tego kawałka; piersze dane, kiery ôdli przez weryfikacyjo, sie ôd razu zapisuje na dysk, a ôdym prosze do tego samego kawałka sie ôdwołuje.
- Automatyczne planowaniy kawałków: rozmiar kawałka sie ôd kła ôd dłużstości pliku ôraz liczy równoległych; wznowiôjąc istniejący plik `.part` sie automatycznie trzimie rozmiar kawałków ôd tam zapisany.
- Wôrszywaniy połączeniom ôraz pōdszkadowaniy prôtokołów: wstrzōnym poolu połączeniom ôd tego samego zêrkiôda wôrszuje sie długie ôdzytë HTTP/1.1; wskałano ôd kodowaniy ôd kawałkach, HTTPS ([rustls](https://crates.io/crates/rustls)), przekierowaniom `301`/`302`/`303`/`307`/`308` miyndzy ôbszarami ôraz zêrkiôdama ôraz sterowaniy ôd czasy.

## Prziyklad użyciōa

Przykładowy kod sie ôbyje w `examples/basic.rs` (uruchomienie: `cargo run --example basic`):

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

/// Weryfikatôr kawałek po kawałku ôd CRC32
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

  // Zêrkiôd 0 ôd pierszego kawałka ôd ôdzywajyma uszkodzony dane, zêrkiôd 1 szybki ôraz normalny, zêrkiôd 2 z ôdym ôdzytim wolny węzeł
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

  // Domyślne równoległe pobieraniy ôd wielu zêrkiôdów: automatyczne planowaniy rozmiaru kawałków, ôd zapisaniyô rzadki .part sie ciynie ôraz ôdmiyniyo nazwa
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() ukończone: {size_a} bajtów, automatycznie zaproponowano kawałek {} bajtów",
    plan_chunk_size(size_a, 8)
  );

  // Równoległe pobieraniy ôd wielu zêrkiôdów z weryfikacyjōm CRC32 kawałek po kawałku: uszkodzony kawałki automatycznie sie ôdmiyniowôm zêrkiôda, szybki zêrkiôdy automatycznie krôdům ôd wolnykůch ôraz gonit na kōńcu
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
  println!("Get::verify() zweryfikowane ôraz ukończone: {size_b} bajtów");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Ôpisy interfejsów

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Pobierze `url_li` do `path` ôd domyślnym liczbym równoległych `CONC` (`8`) ôraz domyślnym ôdzym ôd jednym proszym `TIMEOUT` (`30s`); wrôca ôdym bajtów pliku.

### `Get`

Wejsce ôd budownicym `Get::new(url_li, path)`, wskałano łańcuchowe konfigurowaniye:

- `.size(size: u64)`: wjawne ôdmiarka znany dłużstości bajtowo pliku, pominiy ôdpytywaniyô dłużstości ôd daleka.
- `.chunk_size(chunk_size: usize)`: ôdmiarka dłużstości bajtowo jynego kawałka (jak se ôd `0`, wrôca `Error::ChunkSize`).
- `.conc(conc: usize)`: ôdmiarka liczy równoległych koreptyniyjów do pobieraniōa.
- `.timeout(timeout: Duration)`: ôdmiarka ôdzymu proszeniōa ôraz czytaniōa jednym razym.
- `.verify<V2: Verify>(verify: V2)`: ôd ôdwołaniyô weryfikatōra; ôdmiarka już istnieje dlo `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` ôraz `Arc<T>`.
- `.run().await`: ôdôbiy pobieraniyōa ôraz wrôca `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` to numer kawałka ôd zôczątku (0-ôdym), `data` to bajtowy ôdym kawałka, ôd ôdwrôcenia `true` to, że weryfikacyjo sie ôdli.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Ôdwrôca rozmiar kawałka w potędze 2 miyndzy `MIN_CHUNK` (`64 KB`) ôraz `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Ôd dodaniyô ôd stałom `PART_EXT` (`".part"`), ôdwrôca ścieżka tymczasowego pliku do wznowińiyni pobieraniōa.

### `Error` ôraz `Result`

Jednolity typ ôd ôdwracaniyōa `Result<T> = std::result::Result<T, Error>`, wôrianty enumu ôd ôdmyłki: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
