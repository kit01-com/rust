# msrcget

[compio](https://crates.io/crates/compio) aszinkron befejezési modelljén alapuló, többforrású, blokkos HTTP/HTTPS letöltőkönyvtár, amely a egyes tükör sávszélességkorlátozásának, a blokkadat-sérülésnek és a lassú csomópontok által okozott lelassulásnak a megoldására szolgál.

## Teljesítménymérés

Az [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) és [ureq](https://crates.io/crates/ureq) letöltési sávszélességének összehasonlítása nyilvános HTTPS tükörforrásokon:

### 1. jelenszint: Több nyilvános tükör aggregált letöltése

A Node.js v22.14.0 kiadói csomag (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB) letöltése; a tükörök sorrendje: Tsinghua, USZT, Huawei Cloud, Tencent Cloud, npmmirror (302-es forráson átívelő átirányítással).

| Könyvtár / Megoldás | Sávszélesség (MB/s) |
| --- | ---: |
| msrcget (5 forrás, 8 párhuzamosság) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 forrás) | 1.2 |
| cyper (egyes forrás) | 14.4 |
| reqwest (egyes forrás) | 14.4 |
| ureq (egyes forrás) | 14.3 |

### 2. jelenszint: Gyors és lassú források kevert letöltése

A Node.js v22.14.0 fejléc-csomag (`node-v22.14.0-headers.tar.gz`, 8,4 MB) letöltése; a tükörök sorrendje: Alibaba Cloud (egyes kapcsolat sávszélességkorlátozással), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js hivatalos forrás (határon átívelő lassú csomópont).

| Könyvtár / Megoldás | Sávszélesség (MB/s) |
| --- | ---: |
| msrcget (5 forrás, 8 párhuzamosság) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 forrás) | 0.2 |
| cyper (egyes forrás) | 6.8 |
| reqwest (egyes forrás) | 6.6 |
| ureq (egyes forrás) | 6.7 |

### Vizsgálati módszer

A mérési kód a `examples/bench.rs` fájlban található (végrehajtás: `cargo run --profile bench --example bench`):

- Baszispont előkészítése: Az időmérés előtt előtölti a célfájlt, ellenőrzi a bájt hosszát és kiszámítja a teljes csomag valamint az egyes blokkok CRC32 referenciaértékét; ez a szakasz nem számít bele az időbe.
- Időmérés határai: Minden megoldás 3 egymás utáni, független futást végez; minden futás előtt törli a kimeneti fájlt és a `.part` ideiglenes fájlt. Egy futás a hálózati kérés indításával kezdődik, a fájl lemezre írásának befejeztekor áll le (a `cyper`, `reqwest` és `ureq` mindegyike explicit módon végrehajtja a `sync_data` lemezeztetést; a `msrcget` a teljes folyamatot magában foglalja: távoli hossz felderítése, sparse fájl létrehozása, párhuzamos blokkos letöltés, `sync_data` lemezeztetés, a végi metaadatok levágása és atomi átnevezés).
- Kimenet ellenőrzése: Minden futás leállítása után folyamatosan beolvasja a lemezre írt fájlt, ellenőrzi a bájt hosszát és a teljes csomag CRC32 értékét; az ellenőrzés nem számít a letöltési időbe.
- Sávszélesség átszámítása: A 3 futás időzítésének mediánja alapján a `fájl_bájtok / 1048576 / medián_másodperc` képlettel számítódik a sávszélesség (`1 MB = 1048576 bájt`).
- Forrás kiosztása: A többforrású megoldások (`msrcget`, `down`) a jelenszint összes 5 tükör címét adják meg; az egyes forrásos megoldások (`cyper`, `reqwest`, `ureq`) nem támogatják a többforrású blokkosítást, a kéréslista 1. tükör címét használják.

## Funkciók

- Blokkonkénti ellenőrzés és hibaleváltás: A hívó fél `Verify` interfészben adja meg az ellenőrzést minden blokkhoz; ha az ellenőrzés elbukik vagy a hálózati kérés meghiúsul, automatikusan átvált a többi, meg nem próbált tükörforrásra. Ha minden forrás kimerülése után továbbra is hiba történik, `Error::Exhausted`-t ad vissza.
- Sparse fájlok és megszakítás utáni folytatás: A fájl teljes hossza alapján előre létrehoz egy sparse `.part` ideiglenes fájlt és fájlt zárol (zárütközés esetén `Error::Locked`-t ad vissza). A fájl végére dual slot ping-pong CRC32 bitteret ír a kész blokkok perzisztens tárolásához; megszakítás és újraindítás után automatikusan folytatja, teljes befejezéskor levágja a végi metaadatokat és atomi módon átnevezi a célfájllá.
- Kettéosztásos feladatlopás: Az a forrás, amelyik először befejezi saját intervallumának letöltését, a legnagyobb, be nem fejezett intervallumot felezve átveszi annak második felét, így kiegyenlíti a tükörforrások terhelését.
- Záró szakasz többútvonalas versenye: Amikor a szabad coroutineok száma meghaladja a be nem fejezett blokkok számát, redundáns kéréseket ad azokat a tükörforrásokat, amelyek még nem próbálták meg az adott blokkot. Az első, ellenőrzést átmúló adat lemezeztetése után azonnal megszakítja az adott blokk többi kérését.
- Automatikus blokkméret-tervezés: A fájl teljes hossza és a párhuzamosságszám alapján határozza meg a blokkméretet; meglévő `.part` fájl folytatásakor automatikusan használja a benne rögzített blokkméretet.
- Kapcsolat-újrahasznosítás és protokoltámogatás: Beépített, ugyanahhoz a forráshoz tartozó kapcsolatpool újrahasznosítja az HTTP/1.1 tartós kapcsolatokat; támogatja a darabos adatátviteli kódolást, HTTPS-t ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` útvonal- és forráson átívelő átirányítást, valamint időtúllépés-kezelést.

## Használati példa

A példakód a `examples/basic.rs` fájlban található (végrehajtás: `cargo run --example basic`):

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

/// CRC32-alapú blokkonkénti validáló
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

  // A 0. forrás az 1. blokkot sérült adatokkal adja vissza, az 1. forrás nagy sebességgel működik hibamentesen, a 2. forrás magas késésű lassú csomópont
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

  // Alapértelmezett többforrású párhuzamos letöltés: automatikus blokkméret-tervezés, sparse .part fájl írása utáni levágás és átnevezés
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Többforrású letöltés blokkonkénti CRC32 ellenőrzéssel: a sérült blokkoknál automatikus forrásváltás, a gyors forrás automatikus lopása és a lezárásnál a lassú forrással való verseny
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

## Interfész-leírás

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Az alapértelmezett párhuzamosságszám `CONC` (`8`) és az alapértelmezett egyszeri időtúllépés `TIMEOUT` (`30s`) segítségével letölti a `url_li` forrást a `path` célúthoz; visszatéríti a fájl teljes bájt számát.

### `Get`

Builder belépési pont: `Get::new(url_li, path)`, láncolt konfigurációval:

- `.size(size: u64)`: Explícit módon megadja a ismert fájl teljes bájt számát, kihagyva a távoli hossz felderítését.
- `.chunk_size(chunk_size: usize)`: Megadja a blokk bájt számát (0 átadása esetén `Error::ChunkSize`-t ad vissza).
- `.conc(conc: usize)`: Megadja a párhuzamos letöltő coroutineok számát.
- `.timeout(timeout: Duration)`: Megadja az egyszeri kérés és olvasás időtúllépését.
- `.verify<V2: Verify>(verify: V2)`: Beágyaz egy ellenőrzőt; megvalósítva a `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` és `Arc<T>` típusokhoz.
- `.run().await`: Indítja a letöltést és `Result<u64>`-t ad vissza.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

Az `idx` a 0-alapú blokk sorszáma, a `data` az adott blokk bájt-szeletje; a `true` visszatérési érték az ellenőrzés sikeres átadását jelzi.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

2 hatványának megfelelő blokk-bajt-számot ad vissza a `MIN_CHUNK` (`64 KB`) és `MAX_CHUNK` (`4 MB`) között.

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

A `PART_EXT` (`".part"`) végződési konstans hozzáillesztésével visszaadja a folytatási ideiglenes fájl útvonalát.

### `Error` és `Result`

Egységes visszatérítési típus: `Result<T> = std::result::Result<T, Error>`; a hibanév variánsai: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
