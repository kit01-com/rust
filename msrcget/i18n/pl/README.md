# msrcget

Biblioteka do wieloźródłowego pobierania HTTP/HTTPS z podziałem na bloki, oparta na asynchronicznym modelu ukończeń [compio](https://crates.io/crates/compio), przeznaczona do rozwiązywania problemów z limitowaniem prędkości pojedynczego lustra, uszkodzeniem danych bloków oraz opóźniami wolnych węzłów.

## Test wydajności

Porównanie przepustowości pobierania [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) i [ureq](https://crates.io/crates/ureq) na publicznych źródłach lustrzanych HTTPS:

### Scenariusz 1: agregowane pobieranie z wielu publicznych lusterek

Pobranie pakietu wydawniczego Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB), lustra w kolejności: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (z przekierowaniem 302 między źródłami).

| Biblioteka / rozwiązanie | Przepustowość (MB/s) |
| --- | ---: |
| msrcget (5 źródeł, 8 współbieżności) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 źródeł) | 1.2 |
| cyper (1 źródło) | 14.4 |
| reqwest (1 źródło) | 14.4 |
| ureq (1 źródło) | 14.3 |

### Scenariusz 2: mieszane pobieranie z szybkich i wolnych źródeł

Pobranie pakietu nagłówków Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8,4 MB), lustra w kolejności: Alibaba Cloud (limit prędkości na połączenie), Tsinghua, Huawei Cloud, Tencent Cloud, oficjalne źródło Node.js (wolny węzeł transgraniczny).

| Biblioteka / rozwiązanie | Przepustowość (MB/s) |
| --- | ---: |
| msrcget (5 źródeł, 8 współbieżności) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 źródeł) | 0.2 |
| cyper (1 źródło) | 6.8 |
| reqwest (1 źródło) | 6.6 |
| ureq (1 źródło) | 6.7 |

### Metoda testowania

Kod benchmarku znajduje się w `examples/bench.rs` (wykonanie: `cargo run --profile bench --example bench`):

- Przygotowanie linii bazowej: przed mierzeniem czasu pobierz plik docelowy raz z góry, zweryfikuj długość bajtów i oblicz wartości bazowe CRC32 dla całego pliku i poszczególnych bloków; ten etap nie jest wliczany do czasu pomiaru.
- Granice pomiaru: każde rozwiązanie jest wykonywane niezależnie 3 razy; przed każdym przebiegiem usuwamy plik wyjściowy i tymczasowy plik `.part`; pomiar jednego przebiegu zaczyna się od wystosowania żądania sieciowego i kończy po pełnym zapisie pliku na dysk (`cyper`, `reqwest`, `ureq` w jawnym wykonaniu `sync_data` do buforowania; `msrcget` obejmuje pełny proces: wykrywanie zdalnej długości, tworzenie pliku sparse, współbieżne pobieranie bloków, buforowanie `sync_data`, skracanie metadanych na końcu oraz atomowa zmiana nazwy).
- Weryfikacja wyników: po zakończeniu pomiaru każdego przebiegu czytaj plik strumieniowo, weryfikując długość bajtów i CRC32 całego pliku; proces weryfikacji nie jest wliczany do czasu pobierania.
- Przeliczenie przepustowości: oblicz medianę czasu z 3 przebiegów, przepustowość = `liczba_bajtów_pliku / 1048576 / mediana_w_sekundach` (`1 MB = 1048576 bajtów`).
- Zasady przydziału źródeł: rozwiązania wieloźródłowe (`msrcget`, `down`) przekazują wszystkie 5 adresów lustrzanych ze scenariusza; rozwiązania jednoźródłowe (`cyper`, `reqwest`, `ureq`) nie obsługują wieloźródłowego podziału na bloki, więc używany jest tylko pierwszy adres lustra z listy żądań.

## Funkcje

- Weryfikacja blok po bloku i przełączanie awaryjne: wywołujący dostarcza interfejs `Verify` do weryfikacji każdego bloku; po niepowodzeniu weryfikacji lub błędzie żądania sieciowego automatycznie przełącza się na pozostałe nietestowane źródła lustrzane; po wyczerpaniu wszystkich źródeł zwraca `Error::Exhausted`.
- Pliki sparse i wznowienie pobierania: na podstawie całkowitej długości pliku przedpobiera się plik tymczasowy `.part` (sparse) i blokuje go (konflikt blokady zwraca `Error::Locked`); na końcu pliku zapisywana jest bitmapa CRC32 z podwójnymi slotami (ping-pong) utrwalająca ukończone bloki; po przerwaniu i ponownym uruchomieniu pobieranie automatycznie się wznowi, a po pełnym ukończeniu skracane są metadane na końcu i wykonuje się atomowa zmiana nazwy na plik docelowy.
- Dwubiegunowe kradzież zadań: źródło, które jako pierwsze ukończy swój interwał, dzieli największy pozostały nieukończony zakres na pół i przejmuje drugą połowę, wyrównując obciążenie między źródłami lustrzanymi.
- Wieloszlakowa rywalizacja w fazie końcowej: gdy liczba wolnych współbieżności przekracza liczbę pozostałych nieukończonych bloków, wysyłane są dodatkowe żądania do źródeł, które jeszcze nie próbowały danego bloku; pierwsze zweryfikowane dane zapisane na dysku natychmiast anuluje pozostałe żądania dla tego samego bloku.
- Automatyczne planowanie bloków: rozmiar bloków jest planowany na podstawie całkowitej długości pliku i liczby współbieżności; przy wznowianiu z istniejącego pliku `.part` automatycznie stosowany jest rozmiar bloków w nim zapisany.
- Ponowne użycie połączeń i obsługa protokołów: wbudowany pulap połączeń tego samego źródła ponownie wykorzystuje długotrwałe połączenia HTTP/1.1, obsługuje kodowanie transferu blokowego, HTTPS ([rustls](https://crates.io/crates/rustls)), przekierowania `301`/`302`/`303`/`307`/`308` między ścieżkami i źródłami oraz kontrolę czasu oczekiwania.

## Przykład użycia

Kod przykładowy znajduje się w `examples/basic.rs` (wykonanie: `cargo run --example basic`):

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

/// Weryfikator blokowy oparty na CRC32
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

  // Źródło 0: 1. blok zwraca uszkodzone dane, źródło 1: szybkie i poprawne, źródło 2: wolny węzeł o dużym opóźnieniu
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

  // Domyślny współbieżny pobót wieloźródłowy: automatyczne planowanie rozmiaru bloków, zapis pliku sparse .part, potem skrócenie i zmiana nazwy
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() zakończony: {size_a} bajtów, zalecany automatyczny rozmiar bloku {} bajtów",
    plan_chunk_size(size_a, 8)
  );

  // Wieloźródłowy pobót z weryfikacją CRC32 blok po bloku: uszkodzone bloki automatycznie przełączają źródło, szybkie źródła automatycznie kradną zadania i rywalizują z wolnymi w fazie końcowej
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
  println!("Get::verify() zweryfikowany i zakończony: {size_b} bajtów");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Dokumentacja interfejsu

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Pobiera `url_li` do `path` z domyślną współbieżnością `CONC` (`8`) i domyślnym czasem oczekiwania na pojedyncze żądanie `TIMEOUT` (`30 s`), zwracając całkowitą liczbę bajtów pliku.

### `Get`

Punkt wejścia budownika `Get::new(url_li, path)`, obsługujący konfigurację łańcuchową:

- `.size(size: u64)`: jawne podanie znanej całkowitej liczby bajtów pliku, pomijając zdalne wykrywanie długości.
- `.chunk_size(chunk_size: usize)`: podanie liczby bajtów na blok (przekazanie `0` zwraca `Error::ChunkSize`).
- `.conc(conc: usize)`: podanie liczby współbieżnych współbieżności do pobierania.
- `.timeout(timeout: Duration)`: podanie czasu oczekiwania na pojedyncze żądanie i odczyt.
- `.verify<V2: Verify>(verify: V2)`: wstrzyknięcie weryfikatora; zaimplementowany dla `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` i `Arc<T>`.
- `.run().await`: rozpoczęcie pobierania i zwrócenie `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` to 0-bazowy numer bloku, `data` to wycinek bajtów danego bloku; zwrócenie `true` oznacza pomyślną weryfikację.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Zwraca rozmiar bloku będący potęgą dwójki, mieszczący się między `MIN_CHUNK` (`64 KB`) a `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Dopisuje stałą sufiksu `PART_EXT` (".part"), zwraca ścieżkę pliku tymczasowego do wznowienia.

### `Error` i `Result`

Zjednolicony typ zwracany `Result<T> = std::result::Result<T, Error>`, warianty wyliczenia błędów obejmują: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
