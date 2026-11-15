# msrcget

Knihovna pro stahování HTTP/HTTPS z více zdrojů s rozdělením do segmentů, založená na asynchronním completions modelu [compio](https://crates.io/crates/compio), sloužící k řešení limitování rychlosti u jednotlivých mirrorů, poškození dat v segmentech a zpomalování celkového stahování pomalými uzly.

## Hodnocení výkonu

Srovnání stahovacího throughputu knihoven [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) a [ureq](https://crates.io/crates/ureq) u HTTPS mirrorů na veřejné síti:

### Scénář 1: Agregované stahování z více veřejných mirrorů

Stahování distribučního balíčku Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB), mirror v pořadí: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (včetně přesměrování 302 napříč zdroji).

| Knihovna / řešení | Provoz (MB/s) |
| --- | ---: |
| msrcget (5 zdrojů, 8 paralelních) | 13,9 |
| msrcget + CRC32 | 13,4 |
| down (5 zdrojů) | 1,2 |
| cyper (jeden zdroj) | 14,4 |
| reqwest (jeden zdroj) | 14,4 |
| ureq (jeden zdroj) | 14,3 |

### Scénář 2: Smíšené stahování z rychlých a pomalých zdrojů

Stahování balíčku hlaviček Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8,4 MB), mirror v pořadí: Alibaba Cloud (limit na spojení), Tsinghua, Huawei Cloud, Tencent Cloud, oficiální zdroj Node.js (pomalý mezinárodní uzel).

| Knihovna / řešení | Provoz (MB/s) |
| --- | ---: |
| msrcget (5 zdrojů, 8 paralelních) | 13,5 |
| msrcget + CRC32 | 13,0 |
| down (5 zdrojů) | 0,2 |
| cyper (jeden zdroj) | 6,8 |
| reqwest (jeden zdroj) | 6,6 |
| ureq (jeden zdroj) | 6,7 |

### Metodika testování

Kód pro benchmarking se nachází v `examples/bench.rs` (spuštění: `cargo run --profile bench --example bench`):

- Příprava referenční hodnoty: Před měřením se stáhne cílový soubor jednou dopředu, ověří se délka v bajtech a vypočítají se referenční hodnoty CRC32 pro celý soubor i jednotlivé segmenty. Tato fáze se do měření nezapočítává.
- Hranice měření: Každé řešení se spustí nezávisle 3 kolí; před začátkem každého kola se smaže výstupní soubor i dočasné soubory `.part`. Měření jednoho kola začíná odesláním síťového požadavku a končí dokončením zápisu souboru na disk (u `cyper`, `reqwest` i `ureq` se explicitně vyvolá `sync_data` pro proplavení cache; u `msrcget` zahrnuje celý tok: detekci délky na vzdáleném konci, vytvoření řídkého souboru, paralelní stahování segmentů, `sync_data` pro proplavení cache, oříznutí koncových metadat a atomické přejmenování).
- Ověření výsledků: Po ukončení měření každého kola se soubor na disku přečte postupně (streamingově), ověří se délka v bajtech a CRC32 celého souboru. Vyhodnocování se do doby stahování nezapočítává.
- Přepočet throughputu: Berou se medián doby ze 3 kol, throughput se vypočítá jako `počet bajtů souboru / 1048576 / medián ve sekundách` (`1 MB = 1048576 bajtů`).
- Pravidlo přiřazování zdrojů: Více zdrojové řešení (`msrcget`, `down`) dostává všech 5 adres mirrorů ze scénáře; jedno-zdrovové řešení (`cyper`, `reqwest`, `ureq`) nepodporuje více zdrojové segmenty, používá 1. adresu mirroru ze seznamu.

## Funkce

- Ověření a přepojování při selhání segmentů: Volající předá rozhraní `Verify` pro ověření každého segmentu; při selhání ověření nebo chyby síťového požadavku se automaticky přepojí na zbytek nepoužitých mirrorů; pokud selže u všech zdrojů, vrátí se `Error::Exhausted`.
- Řídké soubory a pokračování přerušeného stahování: Podle celkové délky souboru se předem vytvoří řídký dočasné soubory `.part` a přidá se souborový zámoček (konflikt zámočku vrátí `Error::Locked`); na konci souboru se zapisuje dvouslotový ping-pong CRC32 bitmap pro perzistentní uchování dokončených segmentů. Po přerušení a restartu se stahování automaticky pokračuje; po dokončení všech segmentů se oříznou koncová metadata a soubor se atomicky přejmenuje na cílový soubor.
- Binární krádež úloh: Zdroj, který nejdřív dokončí stahování svého intervalu, rozdělí největší zbytek nedokončených intervalů na poloviny a převezme druhou polovinu, čímž se vyrovnává zatížení jednotlivých mirrorů.
- Závěrečný závod: Když je počet volných coroutin větší než počet zbývajících nedokončených segmentů, pošlou se nadbytečné požadavky na mirror, který daný segment ještě nezkusil; první data, která projdou ověřením a zapíší se na disk, okamžitě zruší zbytek požadavků pro tentýž segment.
- Automatické plánování segmentů: Velikost segmentu se plánuje podle celkové délky souboru a počtu paralelních operací; při pokračování stahování z existujícího souboru `.part` se automaticky použije velikost segmentu v něm uvedená.
- Opětelné použití spojů a podpora protokolů: Vestavěný pool spojů pro stejný zdroj opakovně využívá HTTP/1.1 long-lived spoje; podpora chunked transfer encoding, HTTPS ([rustls](https://crates.io/crates/rustls)), přesměrování `301`/`302`/`303`/`307`/`308` napřícestemi i mezi zdroji a řízení časových limitů.

## Ukázka použití

Ukázkový kód se nachází v `examples/basic.rs` (spuštění: `cargo run --example basic`):

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

/// Vyhodnocovač segmentů založený na CRC32
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

  // Zdroj 0 vrací poškozená data pro 1. segment, zdroj 1 je rychlý a v pořádku, zdroj 2 je pomalý s vysokou latencí
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

  // Výchozí paralelní stahování z více zdrojů: automatické plánování velikosti segmentů, zápis řídkého .part a poté oříznutí a přejmenování
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() dokončeno: {size_a} bajtů, automaticky navrhnutý segment {} bajtů",
    plan_chunk_size(size_a, 8)
  );

  // Stahování z více zdrojů s CRC32 ověřováním segmentů: poškozený segment automaticky přepojí na jiný zdroj, rychlý zdroj automaticky kráde úlohy a v závěrečné fázi závodí s pomalým zdrojem
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
  println!("Get::verify() ověřeno a dokončeno: {size_b} bajtů");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Popis rozhraní

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Stáhne `url_li` do `path` s výchozím počtem paralelních operací `CONC` (`8`) a výchozím časovým limitem na požadavek `TIMEOUT` (`30 s`), vrátí celkový počet bajtů souboru.

### `Get`

Vstupní bod builderu `Get::new(url_li, path)`, podporuje řetězcové konfigurování:

- `.size(size: u64)`: explicitně zadat známou celkovou délku souboru v bajtech, přeskočí detekci délky na vzdáleném konci.
- `.chunk_size(chunk_size: usize)`: zadat počet bajtů na segment (předání `0` vrátí `Error::ChunkSize`).
- `.conc(conc: usize)`: zadat počet paralelních coroutin pro stahování.
- `.timeout(timeout: Duration)`: zadat časový limit pro jednotlivé požadavky a čtení.
- `.verify<V2: Verify>(verify: V2)`: injektovat vyhodnocovač; implementováno pro `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` i `Arc<T>`.
- `.run().await`: spustí stahování a vrátí `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` je 0-položné pořadové číslo segmentu, `data` je bajtový řez daného segmentu; návrat `true` znamená, že ověření proběhlo úspěšně.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Vrátí velikost segmentu jako mocninu 2 mezi `MIN_CHUNK` (`64 KB`) a `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Připojí konstantu přípony `PART_EXT` (`".part"`), vrátí cestu dočasného souboru pro pokračování stahování.

### `Error` a `Result`

Jednotný návratový typ `Result<T> = std::result::Result<T, Error>`, varianty chybového výčtu zahrnují: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
