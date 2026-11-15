# msrcget

Mehrsquellenbasierte, chunkbasierte HTTP/HTTPS-Download-Bibliothek auf dem asynchronen Fertigstellungsmodell von [compio](https://crates.io/crates/compio), zur Behebung von Ratenbegrenzung einzelner Mirrors, Chunk-Datenkorruption und Engpass durch langsame Knoten

## Leistungsbewertung

Vergleich des Download-Durchsatzes von [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) und [ureq](https://crates.io/crates/ureq) über öffentliche HTTPS-Mirrors:

### Szenario 1: Aggregierter Download über mehrere öffentliche Mirrors

Download des Node.js v22.14.0 Release-Pakets (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB), Mirrors in der Reihenfolge: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (inkl. 302-quellübergreifender Weiterleitungen)

| Bibliothek / Ansatz | Durchsatz (MB/s) |
| --------------------- | ----------: |
| msrcget (5 Quellen, 8 parallel) |        13.9 |
| msrcget + CRC32       |        13.4 |
| down (5 Quellen)      |         1.2 |
| cyper (1 Quelle)      |        14.4 |
| reqwest (1 Quelle)    |        14.4 |
| ureq (1 Quelle)       |        14.3 |

### Szenario 2: Gemischter Download über schnelle und langsame Quellen

Download des Node.js v22.14.0 Header-Pakets (`node-v22.14.0-headers.tar.gz`, 8,4 MB), Mirrors in der Reihenfolge: Alibaba Cloud (ratenbegrenzt pro Verbindung), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js-Official Source (langsamer grenzüberschreitender Knoten)

| Bibliothek / Ansatz | Durchsatz (MB/s) |
| --------------------- | ----------: |
| msrcget (5 Quellen, 8 parallel) |        13.5 |
| msrcget + CRC32       |        13.0 |
| down (5 Quellen)      |         0.2 |
| cyper (1 Quelle)      |         6.8 |
| reqwest (1 Quelle)    |         6.6 |
| ureq (1 Quelle)       |         6.7 |

### Testmethodik

Der Benchmark-Code befindet sich in `examples/bench.rs` (Ausführung: `cargo run --profile bench --example bench`):

- Baseline-Vorbereitung: Vor der Zeitmessung wird die Zielt-datei einmal vorab heruntergeladen, die Byte-Länge überprüft und die CRC32-Referenzwerte für das gesamte Paket sowie die einzelnen Chunks berechnet; diese Phase wird nicht in die Laufzeit einberechnet
- Abgrenzung der Zeitmessung: Jeder Ansatz wird in 3 aufeinanderfolgenden, unabhängigen Runden ausgeführt; vor jeder Runde werden die Ausgabedatei und die `.part`-Temporärdatei gelöscht. Die Zeitnahme einer einzelnen Runde beginnt mit dem Senden der Netzwerkanfrage und endet mit dem Abschluss des Schreibens der Datei auf Platte (`cyper`, `reqwest` und `ureq` führen jeweils explizit `sync_data` aus, um die Daten auf die Platte zu flushen; `msrcget` umfasst den vollständigen Ablauf: Ermittlung der Fernlänge, Anlegen einer Sparse-Datei, paralleler Chunks-Download, Abschneiden der Metadaten am Dateiende, `sync_data`-Flush und atomares Umbenennen).
- Ergebnisvalidierung: Nach Beendigung der Zeitmessung wird die Platte-Datei stromweise gelesen und die Byte-Länge sowie der CRC32-Wert des gesamten Pakets geprüft; die Validierung wird nicht in die Download-Laufzeit einberechnet
- Durchsatzberechnung: Der Median aus den Laufzeiten der 3 Runden wird verwendet, um den Durchsatz mit `Dateigröße_in_Byte / 1048576 / Median_s` zu berechnen (`1 MB = 1048576 Bytes`)
- Quellen-Zuweisung: Mehrquellen-Lösungen (`msrcget`, `down`) erhalten alle 5 Mirror-Adressen des Szenarios; Einzelfquellen-Lösungen (`cyper`, `reqwest`, `ureq`) unterstützen keinen chunkbasierten Mehrquellen-Download und verwenden nur die 1. Mirror-Adresse in der Liste

## Funktionen

- Chunkweise Validierung und Failover: Der Aufrufer übergibt eine `Verify`-Schnittstelle zur Überprüfung jedes Chunks; bei Validierungsfehler oder Netzwerkausfall wird automatisch auf eine der noch nicht versuchten Mirror-Quellen umgeschaltet; wenn alle Quellen versagt haben, wird `Error::Exhausted` zurückgegeben
- Sparse-Dateien und Fortsetzung nach Unterbrechung: Basierend auf der Gesamtlänge wird vorab eine Sparse-`.part`-Temporärdatei angelegt und eine Datei-Sperre gesetzt (bei Konflikt wird `Error::Locked` zurückgegeben). Am Dateiende wird eine Bitmap angehängt, die die bereits abgeschlossenen Blöcke markiert; nach einer Unterbrechung wird automatisch an der letzten Stelle fortgesetzt, nach Abschluss aller Blöcke werden die Metadaten am Dateiende entfernt und die Datei atomar in die Zieldatei umbenannt.
- Binärer Task-Stealing: Die erste Quelle, die ihr eigenes Intervall vollständig heruntergeladen hat, teilt den größten verbleibenden unvollendeten Bereich halbiert und übernimmt die zweite Hälfte, um die Last zwischen den Mirror-Quellen auszugleichen
- Mehrpfad-Rennen im Endspurt: Übersteigt die Anzahl freier Coroutines die Anzahl der verbleibenden unvollendeten Chunks, werden redundante Anfragen an Mirror-Quellen gesendet, die diesen Chunk noch nicht versucht haben; sobald der erste validierte Chunk auf der Platte liegt, werden die restlichen Anfragen für denselben Chunk sofort abgebrochen
- Automatische Chunk-Planung: Die Chunk-Größe wird basierend auf der Gesamtdateigröße und der Parallelitätszahl geplant; beim Resume einer vorhandenen `.part`-Datei wird die darin gespeicherte Chunk-Größe automatisch wiederverwendet
- Verbindungsreuse und Protokollunterstützung: Ein eingebetteter Mirror-spezifischer Verbindungspool wiederverwendet HTTP/1.1-Long-Verbindungen; unterstützt Chunked Transfer Encoding, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308`-Weiterleitungen über Pfade und Quellen sowie Timeout-Steuerung

## Verwendungsbeispiel

Der Beispielcode befindet sich in `examples/basic.rs` (Ausführung: `cargo run --example basic`):

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

/// CRC32-basierter Chunk-Validierer
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

  // Mirror 0: Chunk 1 liefert beschädigte Daten, Mirror 1: hoch performant, Mirror 2: langsamer Knoten mit hoher Latenz
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

  // Standard-Mehrsquellen-Konkurrenzdownload: automatische Chunk-Planung, schreibt sparse .part, danach Truncieren und Umbenennen
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Mehrquellen-Download mit CRC32-Chunkvalidierung: fehlerhafte Chunks werden automatisch zu anderen Mirrors umgeleitet; schnelle Mirrors übernehmen per Task-Stealing und konkurrieren im Endspurt mit langsamen
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

## API-Beschreibung

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64> where I: IntoIterator<Item = S>, S: AsRef<str>`

Lädt `url_li` unter Verwendung der Standard-Konkurrenz `CONC` (`8`) und des Standard-Timeout `TIMEOUT` (`30 Sekunden`) nach `path` herunter und gibt die Gesamtbyteanzahl der Datei zurück.

### `Get`

Builder-Einstiegspunkt `Get::new(url_li, path)`, unterstützt Ketteneinstellungen:

- `.size(size: u64)`: Gibt die bekannte Gesamtgröße der Datei in Bytes explizit an und überspringt die Ermittlung der Fernlänge (übergeben von `0` wird als leere Datei abgeschlossen).
- `.chunk_size(chunk_size: usize)`: Angabe der Chunk-Größe in Bytes (`0` ergibt `Error::ChunkSize`)
- `.conc(conc: usize)`: Gibt die Anzahl der parallelen Download-Coroutinen an (übergeben von `0` wird als `1` behandelt).
- `.timeout(timeout: Duration)`: Angabe des Timeouts für einzelne Anfragen und Lesevorgänge
- `.verify<V2: Verify>(verify: V2)`: Injizierung des Validierers; implementiert für `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` und `Arc<T>`
- `.run().await`: Startet den Download und liefert `Result<u64>` zurück

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` ist die 0-basierte Chunk-Nummer, `data` der Bytes-Slice dieses Chunks; `true` bedeutet, dass die Validierung bestanden wurde

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Liefert eine auf eine Zweierpotenz gerundete Chunk-Größe in Bytes im Bereich von `MIN_CHUNK` (`64 KB`) bis `MAX_CHUNK` (`4 MB`)

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Verknüpft die Suffix-Konstante `PART_EXT` (`".part"`) und liefert den Pfad der Resume-Zwischendatei

### `Error` und `Result`

Einheitlicher Rückgabotyp `Result<T> = std::result::Result<T, Error>`; die Fehler-Enum-Varianten umfassen: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`
