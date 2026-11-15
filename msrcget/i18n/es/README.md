# msrcget

Biblioteca de descarga HTTP/HTTPS multifuente por bloques basada en el modelo de completación asíncrona de [compio](https://crates.io/crates/compio), diseñada para resolver la limitación de velocidad de una réplica única, la corrupción de datos por bloque y el efecto de arrastre de nodos lentos.

## Evaluación de rendimiento

Comparación del throughput de descarga de [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) y [ureq](https://crates.io/crates/ureq) en fuentes de réplicas HTTPS de red pública:

### Escenario 1: Descarga agregada de múltiples réplicas públicas

Descarga del paquete de distribución de Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), réplicas en orden: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (con redirección 302 entre fuentes).

| Biblioteca / Método | Ancho de banda (MB/s) |
| --------------------- | ----------: |
| msrcget (5 fuentes, 8 concurrencias) |        13.9 |
| msrcget + CRC32       |        13.4 |
| down (5 fuentes)      |         1.2 |
| cyper (fuente única)  |        14.4 |
| reqwest (fuente única)|        14.4 |
| ureq (fuente única)   |        14.3 |

### Escenario 2: Descarga mixta de fuentes rápidas y lentas

Descarga del paquete de cabeceras de Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8.4 MB), réplicas en orden: Alibaba Cloud (limitación por conexión), Tsinghua, Huawei Cloud, Tencent Cloud, fuente oficial de Node.js (nodo lento transfronterizo).

| Biblioteca / Método | Ancho de banda (MB/s) |
| --------------------- | ----------: |
| msrcget (5 fuentes, 8 concurrencias) |        13.5 |
| msrcget + CRC32       |        13.0 |
| down (5 fuentes)      |         0.2 |
| cyper (fuente única)  |         6.8 |
| reqwest (fuente única)|         6.6 |
| ureq (fuente única)   |         6.7 |

### Método de prueba

El código de evaluación se encuentra en `examples/bench.rs` (ejecutar `cargo run --profile bench --example bench`):

- Preparación de la línea base: antes de cronometrar, se realiza una precarga del archivo objetivo, se verifica la longitud en bytes y se calculan los valores de referencia CRC32 del paquete completo y de cada bloque; esta fase no se cuenta en el tiempo total.
- Límites de temporización: cada método se ejecuta de forma independiente durante 3 rondas consecutivas; antes de cada ronda se elimina el archivo de salida y el archivo temporal `.part`; la medición de una ronda comienza al emitir la solicitud de red y finaliza cuando el archivo se escribe por completo en disco (`cyper`, `reqwest` y `ureq` ejecutan explícitamente `sync_data` para volcar a disco; `msrcget` abarca el flujo completo: detección de longitud remota, creación de archivo disperso, descarga concurrente por bloques, truncado de metadatos finales, `sync_data` para volcar a disco y renombrado atómico).
- Validación del producto: tras detener el cronómetro en cada ronda, se lee el archivo en disco de forma estructurada para verificar la longitud en bytes y el CRC32 del paquete completo; el proceso de validación no se cuenta en el tiempo de descarga.
- Conversión de throughput: se toma la mediana de los tiempos de las 3 rondas y se calcula el throughput como `bytes_del_archivo / 1048576 / segundos_mediano` (`1 MB = 1048576 bytes`).
- Regla de asignación de fuentes: las soluciones multifuente (`msrcget`, `down`) reciben las 5 direcciones de réplicas del escenario; las soluciones de fuente única (`cyper`, `reqwest`, `ureq`) no admiten partición multifuente y solicitan la primera dirección de réplica en la lista.

## Características

- Verificación por bloque y conmutación de fallo: el llamador transmite la interfaz de verificación `Verify` para validar cada bloque; si la verificación falla o la solicitud de red es incorrecta, se conmuta automáticamente a la siguiente réplica no probada; si se agotan todas las fuentes, se devuelve `Error::Exhausted`.
- Archivos dispersos y reanudación desde interrupción: se crea con antelación un archivo temporal `.part` disperso según la longitud total del archivo y se aplica un bloqueo de archivo (en caso de conflicto de bloqueo se devuelve `Error::Locked`); se adjunta un mapa de bits al final del archivo para marcar los bloques ya completados, tras una interrupción y reinicio la descarga se reanuda automáticamente, y al completarse se truncan los metadatos finales y se realiza un renombrado atómico al archivo destino.
- Robo de tareas por bisección: la fuente que primero completa su intervalo propio divide por mitad el intervalo con mayor extensión incompleta restante y asume la segunda mitad, equilibrando la carga entre las réplicas.
- Carrera multipista en el tramo final: cuando el número de coroutines inactivas supera el de bloques incompletos restantes, se envían solicitudes redundantes a las réplicas que aún no han probado dicho bloque; en cuanto el primer dato que supera la verificación se escribe en disco, se cancelan de inmediato las demás solicitudes del mismo bloque.
- Planificación automática de bloques: el tamaño de los bloques se planifica según la longitud total del archivo y el número de concurrencia; al reanudar con un archivo `.part` existente, se reutiliza automáticamente el tamaño de bloque registrado en él.
- Reutilización de conexiones y soporte de protocolo: pool de conexiones integrado por fuente para reutilizar conexiones persistentes HTTP/1.1; admite codificación de transferencia por bloques, HTTPS ([rustls](https://crates.io/crates/rustls)), redirecciones intermedias y entre fuentes `301`/`302`/`303`/`307`/`308` y control de tiempo de espera.

## Demostración de uso

El código de ejemplo se encuentra en `examples/basic.rs` (ejecutar `cargo run --example basic`):

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

/// Validador por bloques basado en CRC32
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

  // La fuente 0 devuelve datos corruptos en el bloque 1; la fuente 1 es rápida y normal; la fuente 2 es un nodo lento de alta latencia
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

  // Descarga concurrente multifuente por defecto: planifica automáticamente el tamaño de bloques, escribe un .part disperso y luego lo trunca y renombra
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Descarga multifuente con verificación CRC32 por bloque: los bloques defectuosos se conmutan automáticamente de fuente; las fuentes rápidas roban bloques y compiten en el tramo final contra las lentas
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

## Descripción de la interfaz

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64> where I: IntoIterator<Item = S>, S: AsRef<str>`

Descarga `url_li` a `path` utilizando el número de concurrencia predeterminado `CONC` (`8`) y el tiempo de espera predeterminado `TIMEOUT` (`30 s`), devolviendo el total de bytes del archivo.

### `Get`

Punto de entrada del constructor `Get::new(url_li, path)`, admite configuración encadenada:

- `.size(size: u64)`: especifica explícitamente el total de bytes del archivo conocido y omite la detección de longitud remota (pasar `0` lo trata como archivo vacío).
- `.chunk_size(chunk_size: usize)`: especifica el número de bytes por bloque (pasar `0` devuelve `Error::ChunkSize`).
- `.conc(conc: usize)`: especifica el número de coroutinas de descarga concurrentes (pasar `0` se trata como `1`).
- `.timeout(timeout: Duration)`: especifica el tiempo de espera de la solicitud y la lectura individual.
- `.verify<V2: Verify>(verify: V2)`: inyecta un validador; ya implementado para `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` y `Arc<T>`.
- `.run().await`: inicia la descarga y devuelve `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` es el índice del bloque con base 0, `data` es la porción de bytes de dicho bloque; retornar `true` indica verificación superada.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Devuelve un tamaño de bloque en potencia de 2 bytes entre `MIN_CHUNK` (`64 KB`) y `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Concatena la constante de sufijo `PART_EXT` (`".part"`) y devuelve la ruta del archivo temporal de reanudación.

### `Error` y `Result`

Tipo de retorno unificado `Result<T> = std::result::Result<T, Error>`; las variantes del enum de error incluyen: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
