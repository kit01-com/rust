# msrcget

Llibreria de descàrrega HTTP/HTTPS fragmentada amb múltiples fonts, basada en el model de completament asíncric de [compio](https://crates.io/crates/compio), dissenyada per resoldre la limitació de velocitat d'un sol mirror, la corrupció de dades de fragments i l'arrossegament de nodes lents.

## Avaluació de rendiment

Comparació del rendiment de descàrrega de [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) i [ureq](https://crates.io/crates/ureq) mitjançant fonts de mirror HTTPS públiques:

### Escenari 1: Descàrrega agregada de múltiples mirrors públics

Descàrrega del paquet de distribució de Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB), amb els mirrors en l'ordre següent: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (inclou redireccions 302 entre dominis).

| Llibreria / Mètode | Rendiment (MB/s) |
| --- | ---: |
| msrcget (5 fonts, 8 en paral·lel) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 fonts) | 1.2 |
| cyper (font única) | 14.4 |
| reqwest (font única) | 14.4 |
| ureq (font única) | 14.3 |

### Escenari 2: Descàrrega amb fonts ràpides i lentes barrejades

Descàrrega del paquet de capçaleres de Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8,4 MB), amb els mirrors en l'ordre següent: Alibaba Cloud (limitació de velocitat per connexió), Tsinghua, Huawei Cloud, Tencent Cloud, font oficial de Node.js (node lent transfronterer).

| Llibreria / Mètode | Rendiment (MB/s) |
| --- | ---: |
| msrcget (5 fonts, 8 en paral·lel) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 fonts) | 0.2 |
| cyper (font única) | 6.8 |
| reqwest (font única) | 6.6 |
| ureq (font única) | 6.7 |

### Mètode de prova

El codi de l'avaluació es troba a `examples/bench.rs` (executar `cargo run --profile bench --example bench`):

- Preparació de la línia base: abans de cronometrar, es pre-descarrega el fitxer objectiu una vegada, es verifica la longitud en bytes i es calculen els valors de referència CRC32 del paquet complet i de cada fragment; aquesta fase no compta en el temps total.
- Límits de cronometratge: cada mètode s'executa de forma independent i consecutiva durant 3 rondes; abans de començar cada ronda, s'elimina el fitxer de sortida i el fitxer temporal `.part`; la cronometració de cada ronda comença amb la petició de xarxa i finalitza quan el fitxer s'ha escrit completament al disc (tant `cyper`, `reqwest` com `ureq` executen explícitament `sync_data` per escriure les dades al disc; `msrcget` inclou la detecció de longitud remota, la creació d'un fitxer dispers, la descàrrega fragmentada en paral·lel, `sync_data` per escriure les dades al disc, la truncació dels metadades de la cua i el canvi atòmic de nom del fitxer complet).
- Validació del resultat: un cop aturat el cronòmetre en cada ronda, es llegeix el fitxer escrit en flux, es verifica la longitud en bytes del fitxer i el CRC32 del paquet complet; el procés de validació no compta en el temps de descàrrega.
- Conversió de rendiment: es pren la mediana del temps de les 3 rondes i es calcula el rendiment com a `bytes del fitxer / 1048576 / segons medians` (`1 MB = 1048576 bytes`).
- Regles d'assignació de fonts: els mètodes de múltiples fonts (`msrcget`, `down`) reben les 5 adreces de mirror de l'escenari; els mètodes de font única (`cyper`, `reqwest`, `ureq`) no admeten fragmentació multiframe, s'utilitza la primera adreça de mirror de la llista de peticions.

## Característiques

- Validació fragment a fragment i commutació en cas de fallada: el cridant proporciona la interfície `Verify` per validar cada fragment; si la validació no s'aprèpia o la petició de xarxa falla, es commuta automàticament a les fonts de mirror restants no intentades per a reintentar; si es recorren totes les fonts i encara falla, es retorna `Error::Exhausted`.
- Fitxers dispersos i reüll des de l'últim punt d'interrupció: es crea prèviament un fitxer temporal `.part` dispers segons la longitud total del fitxer i s'aplica un bloqueig de fitxer (un conflicte de bloqueig retorna `Error::Locked`); a la cua del fitxer s'escriu un mapa de bits CRC32 amb doble ranura ping-pong que persisteix els fragments completats; després d'una interrupció i reinici, la descàrrega es retoma automàticament; un cop complet, es trunca la cua de metadades i es renomea atòmicament al fitxer objectiu.
- Robatori de tasques per bisecció: la font que primer completa el seu interval propi divideix per la meitat l'interval incomplet de major amplitud disponible i assumeix la segona meitat, equilibrant la càrrega entre les fonts de mirror.
- Curses multivies a la cua: quan el nombre de corrutines inactives supera el nombre de fragments pendents, es despeten peticions redundants cap a les fonts de mirror que encara no han intentat aquest fragment; el primer conjunt de dades que passa la validació s'escriu al disc i immediatament es cancel·len les restants peticions del mateix fragment.
- Planificació automàtica de fragments: es planifica la mida de fragment segons la longitud total del fitxer i el grau de paral·lelisme; en reomplir un fitxer `.part` existent, es reutilitza automàticament la mida de fragment registrada a dins.
- Reutilització de connexions i suport de protocol: incorpora una piscina de connexions per a reutilitzar connexions llargues HTTP/1.1 de la mateixa font, admet l'codificació de transferència per fragments, HTTPS ([rustls](https://crates.io/crates/rustls)), redireccions entre camins i entre dominis amb codis `301`/`302`/`303`/`307`/`308` i control de temps d'espera.

## Demostració d'ús

El codi d'exemple es troba a `examples/basic.rs` (executar `cargo run --example basic`):

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

///// Validador fragment a fragment basat en CRC32
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

  // La font 0 retorna dades corruptes al 1r fragment, la font 1 és ràpida i correcta, la font 2 és un node lent amb alta latència
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

  // Descàrrega multiframe concurrent per defecte: planificació automàtica de la mida de fragment, escrit d'un .part dispers i truncament amb canvi de nom
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Descàrrega multiframe amb validació CRC32 fragment a fragment: els fragments corruptes conmuten automàticament de font, les fonts ràpides roben tasques i cursen a la cua contra les fonts lentes
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

## Documentació de la interfície

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Utilitza el grau de paral·lelisme per defecte `CONC` (`8`) i el temps d'espera individual per defecte `TIMEOUT` (`30s`) per descarregar `url_li` a `path`, i retorna el nombre total de bytes del fitxer.

### `Get`

Punt d'entrada del constructor `Get::new(url_li, path)`, admet configuració en cadena:

- `.size(size: u64)`: especifica explícitament el nombre total de bytes coneguts del fitxer, ometent la detecció de longitud remota.
- `.chunk_size(chunk_size: usize)`: especifica el nombre de bytes per fragment (transmetre `0` retorna `Error::ChunkSize`).
- `.conc(conc: usize)`: especifica el nombre de corrutines de descàrrega en paral·lel.
- `.timeout(timeout: Duration)`: especifica el temps d'espera per a cada petició i lectura.
- `.verify<V2: Verify>(verify: V2)`: injecta un validador; ja està implementat per a `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` i `Arc<T>`.
- `.run().await`: inicia la descàrrega i retorna `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` és l'índex del fragment amb base 0, `data` és la retallada de bytes d'aquest fragment, retornar `true` indica que la validació ha superat.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Retorna un nombre de bytes de fragment que sigui potència de 2 entre `MIN_CHUNK` (`64 KB`) i `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Concatena la constant de sufix `PART_EXT` (`".part"`) i retorna el camí del fitxer temporal de reüll.

### `Error` i `Result`

Tipus de retorn uniforme `Result<T> = std::result::Result<T, Error>`, amb variants d'error que inclouen: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
