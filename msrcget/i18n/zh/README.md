# msrcget

基于 [compio](https://crates.io/crates/compio) 异步完成模型的多源分块 HTTP/HTTPS 下载库，用于解决单镜像限速、分块数据损坏及慢节点拖尾问题。

## 性能评测

对比 [down](https://crates.io/crates/down)、[cyper](https://crates.io/crates/cyper)、[reqwest](https://crates.io/crates/reqwest) 与 [ureq](https://crates.io/crates/ureq) 在公网 HTTPS 镜像源下的下载吞吐量：

### 场景 1：多公共镜像聚合下载

下载 Node.js v22.14.0 发行包（`node-v22.14.0-linux-x64.tar.xz`，28.5 MB），镜像按序为：清华、中科大、华为云、腾讯云、npmmirror（含 302 跨源重定向）。

| 库 / 方案             | 吞吐 (MB/s) |
| --------------------- | ----------: |
| msrcget (5 源,8 并发) |        13.9 |
| msrcget + CRC32       |        13.4 |
| down (5 源)           |         1.2 |
| cyper (单源)          |        14.4 |
| reqwest (单源)        |        14.4 |
| ureq (单源)           |        14.3 |

### 场景 2：快慢源混合下载

下载 Node.js v22.14.0 头文件包（`node-v22.14.0-headers.tar.gz`，8.4 MB），镜像按序为：阿里云（单连接限速）、清华、华为云、腾讯云、Node.js 官方源（跨境慢节点）。

| 库 / 方案             | 吞吐 (MB/s) |
| --------------------- | ----------: |
| msrcget (5 源,8 并发) |        13.5 |
| msrcget + CRC32       |        13.0 |
| down (5 源)           |         0.2 |
| cyper (单源)          |         6.8 |
| reqwest (单源)        |         6.6 |
| ureq (单源)           |         6.7 |

### 测试方法

评测代码位于 `examples/bench.rs`（执行 `cargo run --profile bench --example bench`）：

- 基线准备：计时前先预下载一次目标文件，核对字节长度并计算整包及各分块的 CRC32 基准值，该阶段不计入耗时。
- 计时边界：每套方案连续独立运行 3 轮，每轮开始前删除输出文件与 `.part` 临时文件；单轮从发起网络请求开始计时，至文件落盘完成停止计时（`cyper`、`reqwest`、`ureq` 均显式执行 `sync_data` 刷盘；`msrcget` 包含远端长度探测、创建稀疏文件、并发分块下载、截断尾部元数据、`sync_data` 刷盘及原子重命名全流程）。
- 产物校验：每轮停止计时后流式读取落盘文件，校验文件字节长度与整包 CRC32，校验过程不计入下载耗时。
- 吞吐换算：取 3 轮耗时的中位数，按 `文件字节数 / 1048576 / 中位秒数` 计算吞吐量（`1 MB = 1048576 字节`）。
- 源分配规则：多源方案（`msrcget`、`down`）传入场景内全部 5 个镜像地址；单源方案（`cyper`、`reqwest`、`ureq`）不支持多源分块，请求列表第 1 个镜像地址。

## 特性

- 逐块校验与故障切换：调用方传入 `Verify` 校验接口验证每个分块；校验未通过或网络请求失败时，自动切换至其余未尝试过的镜像源重试，遍历全部源仍失败时返回 `Error::Exhausted`。
- 稀疏文件与断点续传：按文件总长度预先创建稀疏 `.part` 临时文件并加文件锁（锁冲突返回 `Error::Locked`）；在文件末尾附加位图标记已完成分块，中断重启后自动续传，全部完成后截断尾部元数据并原子改名为目标文件。
- 二分任务窃取：率先完成自身区间的下载源从剩余未完成跨度最大的区间对半切分并接管后半段，平衡各镜像源负载。
- 尾盘多路竞速：当空闲协程数多于剩余未完成分块数时，向尚未尝试该分块的镜像源派发冗余请求；首个通过校验的数据落盘后立即取消同块其余请求。
- 自动分块规划：按文件总长度与并发数规划分块大小；续传已有 `.part` 文件时自动沿用其中记录的分块大小。
- 连接复用与协议支持：内置同源连接池复用 HTTP/1.1 长连接，支持分块传输编码、HTTPS（[rustls](https://crates.io/crates/rustls)）、`301`/`302`/`303`/`307`/`308` 跨路径与跨源重定向及超时控制。

## 使用演示

示例代码位于 `examples/basic.rs`（执行 `cargo run --example basic`）：

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

/// 基于 CRC32 的逐块校验器
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

  // 源 0 第 1 块返回损坏数据，源 1 高速正常，源 2 高延迟慢节点
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

  // 默认多源并发下载：自动规划分块大小，写稀疏 .part 后截断改名
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // 带逐块 CRC32 校验的多源下载：坏块自动切换源，快源自动窃取与尾盘竞速慢源
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

## 接口说明

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64> where I: IntoIterator<Item = S>, S: AsRef<str>`

使用默认并发数 `CONC`（`8`）与默认单次超时 `TIMEOUT`（`30 秒`）下载 `url_li` 至 `path`，返回文件总字节数。

### `Get`

构建器入口 `Get::new(url_li, path)`，支持链式配置：

- `.size(size: u64)`：显式指定已知文件总字节数，跳过远端长度探测（传 `0` 按空文件完成）。
- `.chunk_size(chunk_size: usize)`：指定单块字节数（传 `0` 返回 `Error::ChunkSize`）。
- `.conc(conc: usize)`：指定并发下载协程数（传 `0` 按 `1` 处理）。
- `.timeout(timeout: Duration)`：指定单次请求与读取超时。
- `.verify<V2: Verify>(verify: V2)`：注入校验器，已为 `()`、`Fn(usize, &[u8]) -> bool`、`Rc<T>` 与 `Arc<T>` 实现。
- `.run().await`：启动下载并返回 `Result<u64>`。

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` 为 0 基分块序号，`data` 为该块字节切片，返回 `true` 表示校验通过。

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

在 `MIN_CHUNK`（`64 KB`）与 `MAX_CHUNK`（`4 MB`）之间返回 2 的幂分块字节数。

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

拼接后缀常量 `PART_EXT`（`".part"`），返回续传临时文件路径。

### `Error` 与 `Result`

统一返回类型 `Result<T> = std::result::Result<T, Error>`，错误枚举变体包括：`EmptySrc`、`ChunkSize`、`Url`、`Scheme`、`Io`、`Tls`、`Timeout`、`Redirect`、`Status`、`Proto`、`Probe`、`Locked`、`Exhausted { idx }`。
