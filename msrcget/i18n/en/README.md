# msrcget

A multi-source chunked HTTP/HTTPS download library built on the [compio](https://crates.io/crates/compio) async completion model, designed to address single-mirror rate limiting, chunk data corruption, and slow-node tail latency.

## Performance Benchmarks

Download throughput comparison of [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest), and [ureq](https://crates.io/crates/ureq) against public HTTPS mirrors:

### Scenario 1: Multi-public-mirror aggregate download

Download the Node.js v22.14.0 release package (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB). Mirror order: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (including 302 cross-origin redirects).

| Library / Solution             | Throughput (MB/s) |
| --------------------- | ----------: |
| msrcget (5 sources, 8 concurrent) |        13.9 |
| msrcget + CRC32       |        13.4 |
| down (5 sources)           |         1.2 |
| cyper (single source)          |        14.4 |
| reqwest (single source)        |        14.4 |
| ureq (single source)           |        14.3 |

### Scenario 2: Mixed fast/slow source download

Download the Node.js v22.14.0 headers package (`node-v22.14.0-headers.tar.gz`, 8.4 MB). Mirror order: Alibaba Cloud (single-connection rate limit), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js official source (cross-border slow node).

| Library / Solution             | Throughput (MB/s) |
| --------------------- | ----------: |
| msrcget (5 sources, 8 concurrent) |        13.5 |
| msrcget + CRC32       |        13.0 |
| down (5 sources)           |         0.2 |
| cyper (single source)          |         6.8 |
| reqwest (single source)        |         6.6 |
| ureq (single source)           |         6.7 |

### Test Methodology

Benchmark code is located at `examples/bench.rs` (run with `cargo run --profile bench --example bench`):

- Baseline preparation: Before timing, the target file is pre-downloaded once to verify byte length and compute baseline CRC32 values for the full file and each chunk. This phase is excluded from timing.
- Timing boundaries: Each solution is run independently for 3 consecutive rounds; before each round, the output file and `.part` temporary file are deleted. A single round is timed from the initiation of the network request to the completion of the on-disk write (`cyper`, `reqwest`, and `ureq` all explicitly perform `sync_data` to flush; `msrcget` covers the full pipeline of remote length probing, sparse file creation, concurrent chunk download, tail metadata truncation, `sync_data` flush, and atomic rename).
- Artifact verification: After each round's timing stops, the on-disk file is read in a streaming manner to verify byte length and full-file CRC32. Verification is excluded from download timing.
- Throughput calculation: Take the median of the 3 rounds' elapsed times and compute throughput as `file_bytes / 1048576 / median_seconds` (`1 MB = 1048576 bytes`).
- Source allocation: Multi-source approaches (`msrcget`, `down`) are passed all 5 mirror URLs in the scenario; single-source approaches (`cyper`, `reqwest`, `ureq`) do not support multi-source chunking, so the first mirror URL in the request list is used.

## Features

- Per-chunk verification and failover: The caller passes a `Verify` verification interface to validate each chunk; when verification fails or a network request errors, the download automatically switches to the next untried mirror source for retry. If all sources are exhausted, `Error::Exhausted` is returned.
- Sparse files and resumption: A sparse `.part` temporary file is pre-created based on the total file length and a file lock is applied (lock contention returns `Error::Locked`); a bitmap is appended at the file end to mark completed chunks, enabling automatic resumption after an interruption. Once all chunks are complete, the tail metadata is truncated and the file is atomically renamed to the target name.
- Binary work stealing: The source that first completes its own range splits the remaining uncompleted range with the largest span in half and takes over the latter half, balancing load across mirror sources.
- Tail multi-path racing: When the number of idle coroutines exceeds the remaining uncompleted chunks, redundant requests are dispatched to mirror sources that have not yet been tried for that chunk. The first verified data is written to disk, immediately cancelling all other requests for the same chunk.
- Automatic chunk planning: Chunk size is planned based on total file size and concurrency. When resuming an existing `.part` file, the chunk size recorded within is automatically reused.
- Connection reuse and protocol support: A built-in same-origin connection pool reuses HTTP/1.1 keep-alive connections. Supports chunked transfer encoding, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` cross-path and cross-origin redirects, and timeout control.

## Usage Example

Example code is located at `examples/basic.rs` (run with `cargo run --example basic`):

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

/// A CRC32-based per-chunk verifier
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

  // Source 0 returns corrupted data for chunk 1, source 1 is fast and normal, source 2 is a high-latency slow node
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

  // Default multi-source concurrent download: automatically plans chunk size, writes a sparse .part then truncates and renames
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Multi-source download with per-chunk CRC32 verification: corrupt chunks auto-switch sources, fast sources auto-steal and tail-race slow sources
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

## API Reference

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64> where I: IntoIterator<Item = S>, S: AsRef<str>`

Downloads `url_li` to `path` using the default concurrency `CONC` (`8`) and default per-request timeout `TIMEOUT` (`30 seconds`), returning the total file size in bytes.

### `Get`

Builder entry point `Get::new(url_li, path)`, supporting chained configuration:

- `.size(size: u64)`: Explicitly specifies the known total file size in bytes, skipping remote length probing (passing `0` treats it as an empty file).
- `.chunk_size(chunk_size: usize)`: Sets the chunk size in bytes (passing `0` returns `Error::ChunkSize`).
- `.conc(conc: usize)`: Specifies the number of concurrent download tasks (passing `0` is treated as `1`).
- `.timeout(timeout: Duration)`: Sets the per-request and read timeout.
- `.verify<V2: Verify>(verify: V2)`: Injects a verifier; implemented for `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>`, and `Arc<T>`.
- `.run().await`: Starts the download and returns `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` is the 0-based chunk index, `data` is the byte slice for that chunk; returning `true` indicates verification passed.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Returns a power-of-2 chunk size in bytes between `MIN_CHUNK` (`64 KB`) and `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Appends the suffix constant `PART_EXT` (`".part"`) and returns the resume temp file path.

### `Error` and `Result`

Unified return type `Result<T> = std::result::Result<T, Error>`; error enum variants include: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
