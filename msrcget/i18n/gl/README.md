# msrcget

Biblioteca de descarga HTTP/HTTPS por bloques de múltiples orixes baseada no modelo de completamento asíncrono de [compio](https://crates.io/crates/compio), deseñada para resolver a limitación de velocidade dunha única orixe, a corrupción de datos en bloques e o efecto arrastre dos nodos lentos.

## Evaluación do rendemento

Comparación do débito de descarga en orixes de espello HTTPS de internet pública entre [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) e [ureq](https://crates.io/crates/ureq):

### Escenario 1: descarga agregada de múltiples espellos públicos

Descarga do paquete de distribución de Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB), coas orixes de espello na orde: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (incluído redirecionamento 302 entre orixes).

| Biblioteca / Solución | Débito (MB/s) |
| --- | ---: |
| msrcget (5 orixes, 8 en paralelo) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 orixes) | 1.2 |
| cyper (unha orixe) | 14.4 |
| reqwest (unha orixe) | 14.4 |
| ureq (unha orixe) | 14.3 |

### Escenario 2: descarga mesturada de orixes rápidas e lentas

Descarga do paquete de cabeceiras de Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8,4 MB), coas orixes de espello na orde: Alibaba Cloud (limitación de velocidade por conexión), Tsinghua, Huawei Cloud, Tencent Cloud, orixe oficial de Node.js (nodo lento transfronteirizo).

| Biblioteca / Solución | Débito (MB/s) |
| --- | ---: |
| msrcget (5 orixes, 8 en paralelo) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 orixes) | 0.2 |
| cyper (unha orixe) | 6.8 |
| reqwest (unha orixe) | 6.6 |
| ureq (unha orixe) | 6.7 |

### Método de proba

O código de avaliación está en `examples/bench.rs` (executar `cargo run --profile bench --example bench`):

- Preparación da liña base: antes de cronometrar, descárgase unha vez o ficheiro obxectivo, verifícase a lonxitude en bytes e calcúlase o valor de referencia CRC32 do paquete completo e de cada bloque; esta fase non se computa no tempo.
- Límites de cronometaxe: cada solución execútase de forma independente durante 3 roldas seguidas; antes de cada rolda elimínanse o ficheiro de saída e o ficheiro temporal `.part`; a cronometaxe comézanse cando se emite a solicitude de rede e párase cando o ficheiro está completamente escrito no disco (`cyper`, `reqwest` e `ureq` executan explicitamente `sync_data` para escribir no disco; `msrcget` inclúe a detección da lonxitude remota, a creación do ficheiro escaso, a descarga de bloques en paralelo, `sync_data` para escribir no disco, a truncación dos metadatos da cola e a renomación atómica en todo o fluxo).
- Verificación dos produtos: despois de detener a cronometaxe en cada rolda, léese o ficheiro escrito en fluxo para verificar a lonxitude en bytes e o CRC32 do paquete completo; o proceso de verificación non se computa no tempo de descarga.
- Conversión do débito: tómase a mediana dos tempos das 3 roldas e o débito calcúlase como `bytes do ficheiro / 1048576 / segundos da mediana` (`1 MB = 1048576 bytes`).
- Regras de asignación de orixes: as solucións de múltiplas orixes (`msrcget`, `down`) reciben os 5 enderezos de espello do escenario; as solucións de unha única orixe (`cyper`, `reqwest`, `ureq`) non admiten descente en bloques multi-orixe, e pídese ao primeiro enderezo de espello da lista.

## Características

- Verificación por bloque e conmutación ante fallos: a parte que chama inxecta unha interface `Verify` para validar cada bloque; se a validación non supera ou a solicitude de rede falla, conmuta automaticamente á seguinte orixe de espello non intentada; se falla ao percorrer todas as orixes, devólvese `Error::Exhausted`.
- Ficheiros escasos e reanudación de transferencia: créase previamente un ficheiro temporal `.part` escaso segundo a lonxitude total do ficheiro e aplícase un bloque de ficheiro (se hai conflito de bloqueo, devólvese `Error::Locked`); na cola do ficheiro escríbese un mapa de bits de ping-pong de CRC32 de dobre ranura para persistir os bloques completados; tras unha interrupción e reinicio, reanúdase automaticamente; ao completarse todo, trúncaense os metadatos da cola e renómase atomicamente ao ficheiro obxectivo.
- Ladroego de tarefas por bisección: a orixe de descarga que remata primeiro o seu intervalo divide ao medio o intervalo restante con maior extensión non completada e asume a metade posterior, igualando a carga entre as orixes de espello.
- Competición multi-vía na fase final: cando o número de corutinas en reposo supera o número de bloques restantes non completados, envíanse solicitudes redundantes ás orixes de espello que aínda non intentaron ese bloque; o primeiro dato que supera a validación escríbese no disco e a continuación cancelanse as demais solicitudes do mesmo bloque.
- Planificación automática de bloques: o tamaño do bloque planifícase segundo a lonxitude total do ficheiro e o número de conexións en paralelo; ao reanudar un ficheiro `.part` existente, úsase automaticamente o tamaño de bloque rexistrado nel.
- Reutilización de conexións e soporte de protocolos: inclúe unha piscina de conexións por orixe para reutilizar conexións de mantemento HTTP/1.1; admite codificación de transferencia por bloques, HTTPS ([rustls](https://crates.io/crates/rustls)), redireccionamentos `301`/`302`/`303`/`307`/`308` entre camiños e entre orixes, así como control de tempo de espera.

## Demostración de uso

O código de exemplo está en `examples/basic.rs` (executar `cargo run --example basic`):

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

/// Validador por bloque baseado en CRC32
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

  // A orixe 0 devolve datos corrompidos no bloque 1, a orixe 1 é rápida e normal, a orixe 2 é un nodo lento con alta latencia
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

  // Descarga multi-orixe en paralelo por defecto: planificación automática do tamaño de bloques, escritura do .part escaso e truncación con renomación
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() completado: {size_a} bytes, tamaño de bloque suxerido automaticamente {} bytes",
    plan_chunk_size(size_a, 8)
  );

  // Descarga multi-orixe con verificación CRC32 por bloque: os bloques defectuosos conmutan automaticamente de orixe, as orixes rápidas fan ladroego automático e compiten na fase final contra as orixes lentas
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
  println!("Get::verify() validado e completado: {size_b} bytes");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Documentación da interface

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Descarga de `url_li` a `path` usando o número de conexións en paralelo por defecto `CONC` (`8`) e o tempo de espera por defecto `TIMEOUT` (`30 s`); devolve o número total de bytes do ficheiro.

### `Get`

Punto de entrada do construtor `Get::new(url_li, path)`, admite configuración encadeada:

- `.size(size: u64)`: especifica explicitamente o número total de bytes do ficheiro coñecido, omitindo a detección da lonxitude remota.
- `.chunk_size(chunk_size: usize)`: especifica o número de bytes por bloque (transmitir `0` devolve `Error::ChunkSize`).
- `.conc(conc: usize)`: especifica o número de corutinas de descarga en paralelo.
- `.timeout(timeout: Duration)`: especifica o tempo de espera para a solicitude e a lectura.
- `.verify<V2: Verify>(verify: V2)`: inxecta o validador; xa se implementou para `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` e `Arc<T>`.
- `.run().await`: inicia a descarga e devolve `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` é o índice de bloque base 0; `data` é a rebanada de bytes dese bloque; devolver `true` indica que a validación superou.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Devolve un tamaño de bloque en bytes que é potencia de 2 entre `MIN_CHUNK` (`64 KB`) e `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Engade a constante de sufixo `PART_EXT` (".part") e devolve a ruta do ficheiro temporal de reanudación.

### `Error` e `Result`

Tipo de retorno unificado `Result<T> = std::result::Result<T, Error>`; as variantes do enum de erros inclúen: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
