# msrcget

Multi-bron, geblokkeerde HTTP/HTTPS-downloadbibliotheek op basis van het asynchrone voltoeiingsmodel van [compio](https://crates.io/crates/compio), ontworpen om snelheidsbeperking bij een enkele mirror, beschadigde blokdata en trage knooppunten die de doorvoer beperken, op te lossen.

## Prestatietests

Vergelijking van de download-doorvoer van [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) en [ureq](https://crates.io/crates/ureq) bij publieke HTTPS-mirrorbronnen:

### Scenario 1: geaggregeerde download via meerdere publieke mirrors

Download van het Node.js v22.14.0-releasepakket (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB). Mirrors in volgorde: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (incl. 302-cross-origin redirect).

| Bibliotheek / Oplossing | Doorvoer (MB/s) |
| --------------------- | ----------: |
| msrcget (5 bronnen, 8 concurrenties) |        13.9 |
| msrcget + CRC32       |        13.4 |
| down (5 bronnen)      |         1.2 |
| cyper (énkele bron)   |        14.4 |
| reqwest (énkele bron) |        14.4 |
| ureq (énkele bron)    |        14.3 |

### Scenario 2: download met gemengde snelle en trage bronnen

Download van het Node.js v22.14.0-headerpakket (`node-v22.14.0-headers.tar.gz`, 8,4 MB). Mirrors in volgorde: Alibaba Cloud (snelheidsbeperking per connectie), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js-officialbron (langzame grensoverschrijdende knooppunten).

| Bibliotheek / Oplossing | Doorvoer (MB/s) |
| --------------------- | ----------: |
| msrcget (5 bronnen, 8 concurrenties) |        13.5 |
| msrcget + CRC32       |        13.0 |
| down (5 bronnen)      |         0.2 |
| cyper (énkele bron)   |         6.8 |
| reqwest (énkele bron) |         6.6 |
| ureq (énkele bron)    |         6.7 |

### Testmethode

Evaluatiecode bevindt zich in `examples/bench.rs` (voer `cargo run --profile bench --example bench` uit):

- Basislinevoorbereiding: vóór het meten wordt het doelbestand eenmaal preventief gedownload, wordt de byte-lengte geverifieerd en worden de CRC32-referentiewaarden voor het volledige pakket en de individuele blokken berekend. Deze fase wordt niet meegenomen in de meettijd.
- Meetgrenzen: elke oplossing wordt opeenvolgend en onafhankelijk 3 runden uitgevoerd; vóór elke ronde worden het outputbestand en de `.part`-tijdelijke bestanden verwijderd. Per ronde begint de timing bij het afzetten van het netwerkverzoek en stopt deze wanneer het bestand volledig op schijf is geschreven (`cyper`, `reqwest` en `ureq` voeren expliciet `sync_data` uit om de schijf op te schuiven; `msrcget` omvat de volledige workflow: opvragen van de brongrootte op afstand, aanmaken van een spaarbestand, geconcurrenteerd bloksgewijs downloaden, afkappen van de metadata aan het einde, op schijf schrijven met `sync_data` en atomaar hernoemen).
- Outputverificatie: na het stoppen van de meet per run wordt het geschreven bestand stromend gelezen en worden de byte-lengte en de CRC32 van het volledige pakket geverifieerd. Het verificatieproces wordt niet meegenomen in de downloadtijd.
- Doorvoeromrekening: de mediaan van de meettijden over 3 runs wordt gebruikt om de doorvoer te berekenen als `bestandsbytes / 1048576 / mediaanseconden` (`1 MB = 1048576 bytes`).
- Bronallocatieregels: multi-bron-oplossingen (`msrcget`, `down`) krijgen alle 5 mirroradressen uit het scenario mee; single-bron-oplossingen (`cyper`, `reqwest`, `ureq`) ondersteunen geen multi-bron-blokkade en gebruiken uitsluitend het 1e mirroradres in de lijst.

## Functies

- Per-blokverificatie en failover: de aanroeper geeft een `Verify`-verificatieinterface door om elk blok te valideren; bij mislukte verificatie of een mislukt netwerkverzoek wordt automatisch overgeschakeld naar de overige nog niet geprobeerde mirrorbronnen. Als alle bronnen zijn geprobeerd en het is nog steeds mislukt, wordt `Error::Exhausted` teruggegeven.
- Spaarbestanden en hervatting bij onderbreking: op basis van de totale bestandsgrootte wordt vooraf een spaar-`.part`-tijdelijk bestand aangemaakt en een bestandsslot geplaatst (bij slotconflict wordt `Error::Locked` teruggegeven). Aan het einde van het bestand wordt een bitgrafiek toegevoegd die de voltooide blokken markeert; na een onderbreking wordt automatisch hervat en zodra alles is voltooid, worden de metadata aan het einde afgekapt en het bestand atomaar hernoemd naar het doelfbestand.
- Binaire taakdiefstal: de downloadbron die als eerste haar interval voltooit, deelt het resterende onvoltooide interval met de grootste span in tweeën en neemt de tweede helft over, waardoor de belasting tussen de mirrorbronnen wordt geëquilibreerd.
- Multi-weg race aan het einde: wanneer het aantal idle coroutines groter is dan het aantal resterende onvoltooide blokken, worden redundante verzoeken gestuurd naar mirrorbronnen die dit blok nog niet hebben geprobeerd. Zodra de eerste data die de verificatie heeft doorstaan is geschreven naar schijf, worden de overige verzoeken voor hetzelfde blok onmiddellijk geannuleerd.
- Automatische blokplanning: de blokmaten worden geplaneerd op basis van de totale bestandslengte en het aantal concurrenties. Bij resume van een bestaand `.part`-bestand wordt automatisch de daarin geregistreerde blokmaat overgenomen.
- Verbindingshergebruik en protocolondersteuning: een ingebouwde per-bronverbindingspool hergebruikt HTTP/1.1-keep-alive-verbindingen; ondersteunt chunked transfer encoding, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` cross-path- en cross-origin-redirects, en time-outbeheersing.

## Gebruiksdemonstratie

Voorbeelcode bevindt zich in `examples/basic.rs` (voer `cargo run --example basic` uit):

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

/// Per-blokverificator op basis van CRC32
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

  // Bron 0: blok 1 geeft beschadigde data, bron 1: snel en normaal, bron 2: trage knooppunt met hoge latentie
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

  // Standaard multi-bron concurrente download: automatische blokplanning, sparse .part schrijven en afkappen + hernoemen
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() voltooid: {size_a} bytes, automatisch voorgestelde blok {} bytes",
    plan_chunk_size(size_a, 8)
  );

  // Multi-bron download met per-blok CRC32-verificatie: beschadigde blokken schakelen automatisch van bron, snelle bronnen stelen automatisch en racen de trage bron aan het einde
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
  println!("Get::verify() geverifieerd en voltooid: {size_b} bytes");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## API-beschrijving

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64> where I: IntoIterator<Item = S>, S: AsRef<str>`

Downloadt `url_li` naar `path` met het standaard concurrentieniveau `CONC` (`8`) en de standaard time-out per verzoek `TIMEOUT` (`30 seconden`) en geeft het totale aantal bytes van het bestand terug.

### `Get`

Builder-ingang `Get::new(url_li, path)`, ondersteunt kettingconfiguratie:

- `.size(size: u64)`: geeft het bekende totale aantal bytes van het bestand expliciet aan en slaat de detectie van de brongrootte op afstand over (bij `0` wordt het bestand als leeg beschouwd en als voltooid gemarkeerd).
- `.chunk_size(chunk_size: usize)`: aantal bytes per blok opgeven (het doorgeven van `0` geeft `Error::ChunkSize` terug).
- `.conc(conc: usize)`: stelt het aantal geconcurrenteerde downloadcoroutines in (bij `0` wordt `1` gebruikt).
- `.timeout(timeout: Duration)`: time-out voor een enkel verzoek en lezen opgeven.
- `.verify<V2: Verify>(verify: V2)`: een verificator injecteren; geïmplementeerd voor `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` en `Arc<T>`.
- `.run().await`: start de download en geeft `Result<u64>` terug.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` is de 0-gebaseerde blokindex, `data` is het byteslice van dat blok; het teruggeven van `true` betekent dat de verificatie geslaagd is.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Geeft een machts-van-2-blokbyteaantal terug tussen `MIN_CHUNK` (`64 KB`) en `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Koppelt de suffixconstante `PART_EXT` (`".part"`) en geeft het pad van het tijdelijke resume-bestand terug.

### `Error` en `Result`

Gestandaardiseerde teruggeeftype `Result<T> = std::result::Result<T, Error>`; de foutenumvarianten omvatten: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
