# msrcget

Knižnica na sťahovanie HTTP/HTTPS v blokoch z viacerých zdrojov založená na asynchronom dokončovacom modeli [compio](https://crates.io/crates/compio), určená na riešenie rýchlostných obmedzení jednotlivých zrkadiel, poškodenia blokových dát a oneskorenia spôsobeného pomalými uzlami.

## Výkonové benchmarky

Porovnanie prenosovej rýchlosti sťahovania [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) a [ureq](https://crates.io/crates/ureq) pri verejných HTTPS zrkadlách:

### Scénar 1: Agregované sťahovanie z viacerých verejných zrkadiel

Sťahovanie distribučného balíka Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB), zrkadlá v poradí: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (vrátane presmerovania 302 na iný origin).

| Knižnica / riešenie | Prenosová rýchlosť (MB/s) |
| --- | ---: |
| msrcget (5 zrkadiel, 8 súbežne) | 13,9 |
| msrcget + CRC32 | 13,4 |
| down (5 zrkadiel) | 1,2 |
| cyper (1 zdroj) | 14,4 |
| reqwest (1 zdroj) | 14,4 |
| ureq (1 zdroj) | 14,3 |

### Scénar 2: Súbežné sťahovanie s rýchlymi a pomalými zdrojmi

Sťahovanie balíka hlavičiek Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8,4 MB), zrkadlá v poradí: Aliyun (rýchlostné obmedzenie na jedno spojenie), Tsinghua, Huawei Cloud, Tencent Cloud, oficiálny zdroj Node.js (medzinárodný pomalý uzol).

| Knižnica / riešenie | Prenosová rýchlosť (MB/s) |
| --- | ---: |
| msrcget (5 zrkadiel, 8 súbežne) | 13,5 |
| msrcget + CRC32 | 13,0 |
| down (5 zrkadiel) | 0,2 |
| cyper (1 zdroj) | 6,8 |
| reqwest (1 zdroj) | 6,6 |
| ureq (1 zdroj) | 6,7 |

### Metóda testovania

Vyhodnovací kód sa nachádza v `examples/bench.rs` (spusťte `cargo run --profile bench --example bench`):

- Príprava bázy: pred meraním času sa najprv sťahuje cieľový súbor raz vopred, overí sa dĺžka v bajtoch a vypočíta sa CRC32 pre celý súbor aj jednotlivé bloky; táto fása sa do merania nezahŕňa.
- Hranica merania: každý postup sa nezávisle spustí 3 razy, pred začiatkom každého kola sa odstránia výstupný súbor a dočasný súbor `.part`; meranie začína od odoslania sieťovej požiadavky a končí dokončením zapísania súboru na disk (`cyper`, `reqwest`, `ureq` všetky explicitne vykonajú `sync_data`; `msrcget` zahŕňa detekciu vzdialenej dĺžky, vytvorenie riedkeho súboru, súbežné sťahovanie blokov, `sync_data`, odstrihnutie koncových metadát a atomické premenovanie).
- Overenie výstupu: po ukončení merania sa prečíta súbor na disku postupne (streaming), overí sa dĺžka v bajtoch a CRC32 celého súboru; overovacia fása sa do času sťahovania nezahŕňa.
- Prevod na prenosovú rýchlosť: vezme sa medián z 3 meraní a prenosová rýchlosť sa vypočíta podľa vzorca `počet bajtov súboru / 1048576 / medián v sekundách` (`1 MB = 1048576 bajtov`).
- Pravidlo pridelovania zdrojov: viaczdrové riešenia (`msrcget`, `down`) dostanú všetky 5 adries zrkadiel zo scénara; jednozdrové riešenia (`cyper`, `reqwest`, `ureq`) nepodporujú viaczdrové sťahovanie v blokoch, požiadava smeruje na 1. adresu zrkadla zo zoznamu.

## Vlastnosti

- Blok-po-bloku overovanie a prepínanie pri poruche: volajúca strana prepojí rozhranie `Verify` na overenie každého bloku; pri neúspešnom overovaní alebo chybe sieťovej požiadavky sa automaticky prepnú na ostatné nezskúšané zrkadlá, ak zlyhajú všetky zdroje, vráti sa `Error::Exhausted`.
- Riedke súbory a sťahovanie s pokračovaním: podľa celkovej dĺžky súboru sa vopred vytvorí riedky dočasný súbor `.part` a aplikuje sa zámok súboru (konflikt zámku vráti `Error::Locked`); na konci súboru sa zapisuje bitmapa CRC32 s dvojitým slotom ping-pong na perzistenciu dokončených blokov, po prerušovaní sa sťahovanie automaticky pokračuje, po dokončení všetkého sa odstrihnú koncové metadáta a súbor sa atomicky premenuje na cieľový súbor.
- Krádež úloh metódou binárneho delenia: zdroj, ktorý najskôr dokončí svoj interval, rozdelí najväčší zostávajúci nedokončený interval na pol a prevzieme druhú polovicu, čím sa vyváža záťaž jednotlivých zrkadiel.
- Záverečný súbežný záber: keď počet voľných kórutín prevyšuje počet zostávajúcich nedokončených blokov, zašlú sa redundantné požiadavky na zrkadlá, ktoré daný blok ešte neskúšali; keď prvé dáta, ktoré prejdú overením, sa zapíšu na disk, ostatné požiadavky pre rovnaký blok sa okamžite zrušia.
- Automatické plánovanie blokov: veľkosť blokov sa plánuje podľa celkovej dĺžky súboru a počtu súbežnosti; pri pokračovaní s existujúcim súborom `.part` sa automaticky použije z neho zaznamenaná veľkosť blokov.
- Opätovné použitie spojení a podpora protokolu: zabudovaný pool spojení na rovnakom zrkadle pre opakované použitie HTTP/1.1 dlhých spojení, podpora blokovaného transferového kódovania, HTTPS ([rustls](https://crates.io/crates/rustls)), presmerovania `301`/`302`/`303`/`307`/`308` medzi rôznymi cestami a originmi a riadenie časového limitu.

## Ukážka použitia

Ukážkový kód sa nachádza v `examples/basic.rs` (spusťte `cargo run --example basic`):

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

/// Overovateľ blok po bloku na báze CRC32
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

  // Zdroj 0 vráti poškodené dáta v 1. bloku, zdroj 1 je rýchly a funkčný, zdroj 2 je pomalý uzol s vysokou latenciou
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

  // Východzie súbežné sťahovanie z viacerých zdrojov: automatické plánovanie veľkosti blokov, zapísanie riedkeho .part a následné odstrihnutie a premenovanie
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() hotové: {size_a} bajtov, automaticky odporúčaný blok {} bajtov",
    plan_chunk_size(size_a, 8)
  );

  // Sťahovanie z viacerých zdrojov s blok-po-bloku overovaním CRC32: poškodené bloky sa automaticky prepínajú na iný zdroj, rýchle zdroje automaticky krádú úlohy a v závere súbežne prekonávajú pomalé zdroje
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
  println!("Get::verify() overené a dokončené: {size_b} bajtov");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Popis rozhrania

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Sťahovanie `url_li` do `path` s výchoďovou súbežnosťou `CONC` (`8`) a výchoďovým časovým limitom na jednu požiadavku `TIMEOUT` (`30 s`), vráti celkový počet bajtov súboru.

### `Get`

Vstupný bod stavára `Get::new(url_li, path)`, podporuje reťazcovú konfiguráciu:

- `.size(size: u64)`: explicitné určenie známych celkových bajtov súboru, preskočí detekciu vzdialenej dĺžky.
- `.chunk_size(chunk_size: usize)`: určenie počtu bajtov na blok (pri `0` vráti `Error::ChunkSize`).
- `.conc(conc: usize)`: určenie počtu súbežných stahovacích kórutín.
- `.timeout(timeout: Duration)`: určenie časového limitu na jednu požiadavku a čítanie.
- `.verify<V2: Verify>(verify: V2)`: inject overovacieho prístroja, implementované pre `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` a `Arc<T>`.
- `.run().await`: spustí sťahovanie a vráti `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` je poradové číslo bloku od 0, `data` je rezná úsečka bajtov daného bloku, návrat `true` znamená úspešné overenie.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Vracia mocninu 2 v bajtoch v rozmedzí `MIN_CHUNK` (`64 kB`) a `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Prípojil prieponový konštant `PART_EXT` (`".part"`), vráti cestu dočasného súboru pre pokračovanie sťahovania.

### `Error` a `Result`

Jednotný návratový typ `Result<T> = std::result::Result<T, Error>`, varianty chýbového enumu zahŕňajú: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
