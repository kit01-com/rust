# msrcget

基於 [compio](https://crates.io/crates/compio) 非同步完成模型的多源分塊 HTTP/HTTPS 下載函式庫，用於解決單鏡像限速、分塊資料損毀及慢節點拖尾問題。

## 效能評測

對照 [down](https://crates.io/crates/down)、[cyper](https://crates.io/crates/cyper)、[reqwest](https://crates.io/crates/reqwest) 與 [ureq](https://crates.io/crates/ureq) 在公網 HTTPS 鏡像來源下的下載吞吐量：

### 情境 1：多公共鏡像聚合下載

下載 Node.js v22.14.0 發行包（`node-v22.14.0-linux-x64.tar.xz`，28.5 MB），鏡像順序為：清華、中科大、華為雲、騰訊雲、npmmirror（含 302 跨來源重新導向）。

| 函式庫 / 方案 | 吞吐量 (MB/s) |
| --- | ---: |
| msrcget (5 來源,8 並行) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 來源) | 1.2 |
| cyper (單來源) | 14.4 |
| reqwest (單來源) | 14.4 |
| ureq (單來源) | 14.3 |

### 情境 2：快慢來源混合下載

下載 Node.js v22.14.0 頭文件包（`node-v22.14.0-headers.tar.gz`，8.4 MB），鏡像順序為：阿里雲（單連線限速）、清華、華為雲、騰訊雲、Node.js 官方來源（跨國慢節點）。

| 函式庫 / 方案 | 吞吐量 (MB/s) |
| --- | ---: |
| msrcget (5 來源,8 並行) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 來源) | 0.2 |
| cyper (單來源) | 6.8 |
| reqwest (單來源) | 6.6 |
| ureq (單來源) | 6.7 |

### 測試方法

評測程式碼位於 `examples/bench.rs`（執行 `cargo run --profile bench --example bench`）：

- 基準準備：計時前先預下載目標檔案一次，核對位元組長度並計算整包及各分塊的 CRC32 基準值，該階段不計入耗時。
- 計時邊界：每套方案連續獨立執行 3 輪，每輪開始前刪除輸出檔案與 `.part` 臨時檔案；單輪從發起網路請求開始計時，至檔案寫入磁碟完成停止計時（`cyper`、`reqwest`、`ureq` 均顯式執行 `sync_data` 刷寫磁碟；`msrcget` 包含遠端長度探測、建立稀疏檔案、並行分塊下載、`sync_data` 刷寫磁碟、截斷尾部中繼資料及原子重新命名全流程）。
- 產物校驗：每輪停止計時後串流讀取已寫入磁碟的檔案，校驗檔案位元組長度與整包 CRC32，校驗過程不計入下載耗時。
- 吞吐量換算：取 3 輪耗時的中位數，依 `檔案位元組數 / 1048576 / 中位秒數` 計算吞吐量（`1 MB = 1048576 位元組`）。
- 來源分配規則：多來源方案（`msrcget`、`down`）傳入情境內全部 5 個鏡像位址；單來源方案（`cyper`、`reqwest`、`ureq`）不支援多來源分塊，僅請求清單第 1 個鏡像位址。

## 特性

- 逐塊校驗與故障切換：呼叫方傳入 `Verify` 校驗介面驗證每個分塊；校驗未通過或網路請求失敗時，自動切換至其餘未嘗試過的鏡像來源重試，遍歷全部來源仍失敗時回傳 `Error::Exhausted`。
- 稀疏檔案與中斷續傳：依檔案總長度預先建立稀疏 `.part` 臨時檔案並加上檔案鎖（鎖衝突時回傳 `Error::Locked`）；在檔案末尾寫入雙槽乒乓 CRC32 位圖以持久化已完成分塊，中斷重啟後自動續傳，全部完成後截斷尾部中繼資料並原子重新命名為目標檔案。
- 二分任務竊取：率先完成自身區間下載的來源從剩餘未完成跨距最大的區間對半切分並接管後半段，平衡各鏡像來源負載。
- 尾盤多路競速：當閒置協程數多於剩餘未完成分塊數時，向尚未嘗試該分塊的鏡像來源派发冗餘請求；首個通過校驗的資料寫入磁碟後立即取消同塊其餘請求。
- 自動分塊規劃：依檔案總長度與並行數規劃分塊大小；續傳已有 `.part` 檔案時自動沿用其中記錄的分塊大小。
- 連線重用與通訊協定支援：內建同源連線池以重用 HTTP/1.1 長連線，支援分塊傳輸編碼、HTTPS（[rustls](https://crates.io/crates/rustls)）、`301`/`302`/`303`/`307`/`308` 跨路徑與跨來源重新導向及逾時控制。

## 使用演示

範例程式碼位於 `examples/basic.rs`（執行 `cargo run --example basic`）：

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

/// 基於 CRC32 的逐塊校驗器
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

  // 來源 0 第 1 塊回傳損毀資料，來源 1 高速正常，來源 2 高延遲慢節點
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

  // 預設多來源並行下載：自動規劃分塊大小，寫入稀疏 .part 後截斷重新命名
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 位元組，自動建議分塊 {} 位元組",
    plan_chunk_size(size_a, 8)
  );

  // 附逐塊 CRC32 校驗的多來源下載：壞塊自動切換來源，快來源自動竊取與尾盤競速慢來源
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
  println!("Get::verify() 校驗並完成: {size_b} 位元組");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## 介面說明

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

使用預設並行數 `CONC`（`8`）與預設單次逾時 `TIMEOUT`（`30s`）將 `url_li` 下載至 `path`，回傳檔案總位元組數。

### `Get`

建構器入口 `Get::new(url_li, path)`，支援鏈式配置：

- `.size(size: u64)`：顯式指定已知檔案總位元組數，跳過遠端長度探測。
- `.chunk_size(chunk_size: usize)`：指定單塊位元組數（傳入 `0` 回傳 `Error::ChunkSize`）。
- `.conc(conc: usize)`：指定並行下載協程數。
- `.timeout(timeout: Duration)`：指定單次請求與讀取逾時。
- `.verify<V2: Verify>(verify: V2)`：注入校驗器，已為 `()`、`Fn(usize, &[u8]) -> bool`、`Rc<T>` 與 `Arc<T>` 實作。
- `.run().await`：啟動下載並回傳 `Result<u64>`。

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` 為 0 基分塊序號，`data` 為該塊位元組切片，回傳 `true` 表示校驗通過。

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

在 `MIN_CHUNK`（`64 KB`）與 `MAX_CHUNK`（`4 MB`）之間回傳 2 的次幂分塊位元組數。

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

拼接後綴常數 `PART_EXT`（`".part"`），回傳續傳臨時檔案路徑。

### `Error` 與 `Result`

統一回傳型別 `Result<T> = std::result::Result<T, Error>`，錯誤列舉變體包括：`EmptySrc`、`ChunkSize`、`Url`、`Scheme`、`Io`、`Tls`、`Timeout`、`Redirect`、`Status`、`Proto`、`Probe`、`Locked`、`Exhausted { idx }`。
