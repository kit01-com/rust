# msrcget

Bibliotecă de descărcare HTTP/HTTPS cu mai multe surse și segmentare, bazată pe modelul de completitudine asincronă al [compio](https://crates.io/crates/compio), concepută pentru a rezolva problemele de limitare a vitezei pe o singură oglindă, coruperea datelor segmentate și întârzierile cauzate de nodurile lente.

## Evaluarea performanței

Compararea randamentului de descărcare al [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) și [ureq](https://crates.io/crates/ureq) pe surse de oglinzi HTTPS din rețeaua publică:

### Scenariul 1: Agregare de descărcare de pe mai multe oglinzi publice

Descărcarea pachetului de distribuție Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB), oglinzile în ordinea: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (incluzând redirecționare 302 între surse).

| Bibliotecă / soluție | Randament (MB/s) |
| --- | ---: |
| msrcget (5 surse, 8 concurențe) | 13,9 |
| msrcget + CRC32 | 13,4 |
| down (5 surse) | 1,2 |
| cyper (sursă unică) | 14,4 |
| reqwest (sursă unică) | 14,4 |
| ureq (sursă unică) | 14,3 |

### Scenariul 2: Descărcare mixtă cu surse rapide și lente

Descărcarea pachetului de fișiere de antet Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8,4 MB), oglinzile în ordinea: Alibaba Cloud (limitare de viteză pe conexiune unică), Tsinghua, Huawei Cloud, Tencent Cloud, sursa oficială Node.js (nod lent transfrontalier).

| Bibliotecă / soluție | Randament (MB/s) |
| --- | ---: |
| msrcget (5 surse, 8 concurențe) | 13,5 |
| msrcget + CRC32 | 13,0 |
| down (5 surse) | 0,2 |
| cyper (sursă unică) | 6,8 |
| reqwest (sursă unică) | 6,6 |
| ureq (sursă unică) | 6,7 |

### Metoda de testare

Codul de evaluare se află în `examples/bench.rs` (execuare `cargo run --profile bench --example bench`):

- Pregătirea bazei: înainte de cronometraj, fișierul țintă se pre-descarcă o dată, se verifică lungimea în octeți și se calculează valorile de referință CRC32 pentru pachetul complet și pentru fiecare segment; această etapă nu se include în timpul măsurat.
- Marginile cronometrajului: fiecare soluție rulează independent de 3 ori consecutiv; înainte de fiecare rundă se șterge fișierul de ieșire și fișierul temporar `.part`; cronometrarea începe la emiterea cererii de rețea și se oprește la finalizarea scrierii fișierului pe disc (`cyper`, `reqwest`, `ureq` execută explicit `sync_data` pentru a scrie pe disc; `msrcget` include probele de lungime remote, crearea fișierului spartan, descărcarea concurentă a segmentelor, `sync_data` pentru scrierea pe disc, trunchierea metadatelor de coadă și redenumirea atomică a fișierului).
- Validarea rezultatelor: după oprirea cronometrajului la fiecare rundă, fișierul scris pe disc se citește în flux, se verifică lungimea în octeți a fișierului și CRC32-ul pachetului complet; procesul de validare nu se include în timpul de descărcare.
- Calculul randamentului: se ia mediana duratălor celor 3 runde, iar randamentul se calculează cu `numărul de octeți ai fișierului / 1048576 / mediiana în secunde` (`1 MB = 1048576 octeți`).
- Regula de alocare a surselor: soluțiile cu mai multe surse (`msrcget`, `down`) primesc toate cele 5 adrese de oglinde din scenariu; soluțiile cu sursă unică (`cyper`, `reqwest`, `ureq`) nu suportă descărcare cu mai multe surse segmentate, deci se folosește prima adresă de oglindă din lista de cereri.

## Funcționalități

- Validare segment cu segment și comutare la defecțiune: apelatorul trece o interfață de validare `Verify` pentru a verifica fiecare segment; dacă validarea eșuează sau cererea de rețea eșuează, se comută automat la alte surse de oglinde neîncercate pentru reîncercare; dacă toate sursele sunt epuizate, se returnează `Error::Exhausted`.
- Fișiere spartane și reluare de la punctul de întrerupere: se creează în prealibil un fișier temporar spartan `.part` de lungimea totală a fișierului, cu blocare de fișier (conflict de blocare returnează `Error::Locked`); la coada fișierului se scrie o hartă de biți CRC32 cu doi sloturi de ping-pong pentru a persista segmentele finalizate; la repornirea după o întrerupere, reluarea este automată; după finalizarea completă, metadatele de coadă sunt trunchiate și fișierul este redenumit atomic în fișierul țintă.
- Furt de taskuri prin diviziune binară: sursa care termină mai întâi intervalul propriu împarte la mijloc cel mai mare interval nefinalizat rămas și preia jumătatea din spate, echilibrând sarcina pe sursele de oglinde.
- Competiție cu mai multe căi la final: când numărul de co-rutini libere depășește numărul de segmente nefinalizate rămase, se trimit cereri redundante către sursele de oglinde care nu au încercat acel segment; primul dat care trece validarea este scris pe disc, iar celelalte cereri pentru acel segment sunt anulate imediat.
- Planificarea automată a segmentelor: mărimea segmentelor este planificată în funcție de lungimea totală a fișierului și numărul de concurențe; la reluarea unui fișier `.part` existent, se reia automat mărimea segmentelor înregistrată în acesta.
- Reutilizarea conexiunilor și suportul de protocol: include intern un bazin de conexiuni pentru reutilizarea conexiunilor lungi HTTP/1.1 de pe aceeași sursă, cu suport pentru codificarea transmisiei cu segmente, HTTPS ([rustls](https://crates.io/crates/rustls)), redirecționări `301`/`302`/`303`/`307`/`308` între căi și între surse, și controlul timeout-ului.

## Demo de utilizare

Codul de exemplu se află în `examples/basic.rs` (execuare `cargo run --example basic`):

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

/// Validator pe segment bazat pe CRC32
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

  // Sursa 0 returnează date corupte pentru primul segment, sursa 1 este rapidă și normală, sursa 2 este un nod lent cu latență mare
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

  // Descărcare concurentă implicită de pe mai multe surse: planificarea automată a mărimii segmentelor, scriere pe .part spartan, apoi trunchiere și redenumire
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() finalizat: {size_a} octeți, mărime segment sugerate automat {} octeți",
    plan_chunk_size(size_a, 8)
  );

  // Descărcare de pe mai multe surse cu validare CRC32 pe segment: segmentele defecte comută automat sursa, sursele rapide fură automat taskuri și concurează cu sursele lente la final
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
  println!("Get::verify() validat și finalizat: {size_b} octeți");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Descrierea interfeței

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Descarcă `url_li` în `path` folosind numărul implicit de concurențe `CONC` (`8`) și timeout-ul implicit per cerere `TIMEOUT` (`30s`), returnând numărul total de octeți ai fișierului.

### `Get`

Intrare builder `Get::new(url_li, path)`, cu suport pentru configurare în lanț:

- `.size(size: u64)`: specifică explicit numărul cunoscut de octeți ai fișierului, omite probele de lungime remote.
- `.chunk_size(chunk_size: usize)`: specifică numărul de octeți per segment (trecerea `0` returnează `Error::ChunkSize`).
- `.conc(conc: usize)`: specifică numărul de co-rutini de descărcare concurentă.
- `.timeout(timeout: Duration)`: specifică timeout-ul per cerere și citire.
- `.verify<V2: Verify>(verify: V2)`: injectează un validator, implementat deja pentru `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` și `Arc<T>`.
- `.run().await`: pornește descărcarea și returnează `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` este indicele zero-bazat al segmentului, `data` este felia de octeți a acelui segment, `true` indică validarea reușită.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Returnează mărimea segmentului în octeți, putere a lui 2, între `MIN_CHUNK` (`64 KB`) și `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Adaugă sufixul constant `PART_EXT` (`".part"`), returnând calea fișierului temporar de reluare.

### `Error` și `Result`

Tipul de returnare uniform `Result<T> = std::result::Result<T, Error>`, variantele enumului de eroare includ: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
