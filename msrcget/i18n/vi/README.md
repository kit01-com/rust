# msrcget

Thư viện tải HTTP/HTTPS phân mảnh đa nguồn dựa trên mô hình hoàn tất bất đồng bộ [compio](https://crates.io/crates/compio), dùng để giải quyết vấn đề giới hạn tốc độ của từng mirror, hư hỏng dữ liệu phân mảnh và nút chậm kéo dài thời gian tải.

## Đánh giá hiệu năng

So sánh thông lượng tải xuống trên các nguồn mirror HTTPS công khai của [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) và [ureq](https://crates.io/crates/ureq):

### Kịch bản 1: Tải tập hợp từ nhiều mirror công khai

Tải gói phát hành Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), thứ tự mirror là: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (bao gồm chuyển hướng 302 xuyên nguồn).

| Thư viện / Phương án | Thông lượng (MB/s) |
| --- | ---: |
| msrcget (5 nguồn, 8 song song) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 nguồn) | 1.2 |
| cyper (đơn nguồn) | 14.4 |
| reqwest (đơn nguồn) | 14.4 |
| ureq (đơn nguồn) | 14.3 |

### Kịch bản 2: Tải hỗn hợp nguồn nhanh và chậm

Tải gói file header Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8.4 MB), thứ tự mirror là: Alibaba Cloud (giới hạn tốc độ theo mỗi kết nối), Tsinghua, Huawei Cloud, Tencent Cloud, nguồn chính thức Node.js (nút chậm xuyên biên giới).

| Thư viện / Phương án | Thông lượng (MB/s) |
| --- | ---: |
| msrcget (5 nguồn, 8 song song) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 nguồn) | 0.2 |
| cyper (đơn nguồn) | 6.8 |
| reqwest (đơn nguồn) | 6.6 |
| ureq (đơn nguồn) | 6.7 |

### Phương pháp thử nghiệm

Mã đánh giá nằm trong `examples/bench.rs` (thực thi `cargo run --profile bench --example bench`):

- Chuẩn bị đường cơ sở: Trước khi đo thời gian, tải trước một lần tệp đích, kiểm tra độ dài byte và tính toán giá trị CRC32 tham chiếu cho toàn bộ gói và từng phân mảnh, giai đoạn này không tính vào thời gian.
- Ranh giới đo thời gian: Mỗi phương án chạy liên tiếp độc lập 3 vòng, trước mỗi vòng xóa tệp đầu ra và tệp tạm `.part`; mỗi vòng bắt đầu đo từ khi phát yêu cầu mạng, dừng khi tệp đã ghi xuống đĩa hoàn tất (cả `cyper`, `reqwest`, `ureq` đều thực thi `sync_data` để ghi ra đĩa; `msrcget` bao gồm toàn bộ quy trình: dò chiều dài từ xa, tạo tệp thưa, tải song song phân mảnh, `sync_data` ghi ra đĩa, cắt bỏ metadata cuối tệp và đổi tên nguyên tử).
- Kiểm tra sản phẩm: Sau mỗi vòng dừng đo, đọc dạng streaming tệp đã ghi xuống đĩa, kiểm tra độ dài byte và CRC32 toàn bộ gói, quá trình kiểm tra không tính vào thời gian tải.
- Đổi thông lượng: Lấy trung vị thời gian của 3 vòng, tính thông lượng theo `số byte tệp / 1048576 / số giây trung vị` (`1 MB = 1048576 byte`).
- Quy tắc gán nguồn: Phương án đa nguồn (`msrcget`, `down`) nhận vào tất cả 5 địa chỉ mirror trong kịch bản; phương án đơn nguồn (`cyper`, `reqwest`, `ureq`) không hỗ trợ phân mảnh đa nguồn, nhận địa chỉ mirror thứ 1 trong danh sách yêu cầu.

## Tính năng

- Xác thực từng mảnh và chuyển lỗi: Bên gọi truyền giao diện `Verify` để kiểm tra từng phân mảnh; khi xác thực không đạt hoặc yêu cầu mạng thất bại, tự động chuyển sang các mirror khác chưa thử để thử lại, khi duyệt hết tất cả nguồn vẫn thất bại thì trả về `Error::Exhausted`.
- Tệp thưa và tải tiếp: Tạo trước tệp tạm `.part` thưa theo tổng chiều dài tệp và khóa tệp (xung đột khóa trả về `Error::Locked`); ghi ở cuối tệp bản đồ CRC32 ping-pong hai khe để lưu trữ trạng thái các phân mảnh đã hoàn thành, sau khi ngắt và khởi động lại sẽ tự động tải tiếp, khi tất cả hoàn thành sẽ cắt bỏ metadata cuối và đổi tên nguyên tử thành tệp đích.
- Đánh cắp nhiệm vụ theo chia đôi: Nguồn tải hoàn thành khoảng của mình sớm nhất sẽ chia đôi khoảng chưa hoàn thành lớn nhất và tiếp quản nửa sau, cân bằng tải giữa các mirror.
- Đa đường đua cuối: Khi số coroutine rảnh lớn hơn số phân mảnh chưa hoàn thành, gửi yêu cầu thừa đến các mirror chưa thử phân mảnh đó; dữ liệu đầu tiên vượt qua xác thực sẽ ghi xuống đĩa ngay và hủy các yêu cầu còn lại của cùng phân mảnh.
- Lập kế hoạch phân mảnh tự động: Lập kế hoạch kích thước phân mảnh theo tổng chiều dài tệp và số song song; khi tải tiếp tệp `.part` hiện có sẽ tự động tiếp tục dùng kích thước phân mảnh đã ghi trong đó.
- Tái sử dụng kết nối và hỗ trợ giao thức: Tích hợp sẵn pool kết nối cùng nguồn để tái sử dụng kết nối dài HTTP/1.1, hỗ trợ mã hóa truyền phân mảnh, HTTPS ([rustls](https://crates.io/crates/rustls)), chuyển hướng `301`/`302`/`303`/`307`/`308` xuyên đường dẫn và xuyên nguồn cùng kiểm soát thời gian chờ.

## Hướng dẫn sử dụng

Mã ví dụ nằm trong `examples/basic.rs` (thực thi `cargo run --example basic`):

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

/// Bộ xác thực từng mảnh dựa trên CRC32
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

  // Nguồn 0 mảnh thứ 1 trả về dữ liệu hỏng, nguồn 1 tốc độ cao bình thường, nguồn 2 nút chậm độ trễ cao
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

  // Tải song song đa nguồn mặc định: tự động lập kế hoạch kích thước phân mảnh, ghi tệp thưa .part rồi cắt bỏ và đổi tên
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() hoàn thành: {size_a} byte, tự động đề xuất phân mảnh {} byte",
    plan_chunk_size(size_a, 8)
  );

  // Tải đa nguồn kèm xác thực CRC32 từng mảnh: mảnh hỏng tự động chuyển nguồn, nguồn nhanh tự động đánh cắp và đua với nguồn chậm ở giai đoạn cuối
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
  println!("Get::verify() xác thực và hoàn thành: {size_b} byte");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Tài liệu giao diện

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Sử dụng số song song mặc định `CONC` (`8`) và thời gian chờ mặc định cho mỗi lần `TIMEOUT` (`30s`) để tải `url_li` xuống `path`, trả về tổng số byte của tệp.

### `Get`

Điểm vào builder `Get::new(url_li, path)`, hỗ trợ cấu hình dạng chuỗi:

- `.size(size: u64)`: Chỉ định rõ tổng số byte tệp đã biết, bỏ qua dò chiều dài từ xa.
- `.chunk_size(chunk_size: usize)`: Chỉ định số byte mỗi mảnh (truyền `0` sẽ trả về `Error::ChunkSize`).
- `.conc(conc: usize)`: Chỉ định số coroutine tải song song.
- `.timeout(timeout: Duration)`: Chỉ định thời gian chờ cho mỗi yêu cầu và đọc.
- `.verify<V2: Verify>(verify: V2)`: Ghi nhận bộ xác thực, đã triển khai cho `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` và `Arc<T>`.
- `.run().await`: Khởi động tải và trả về `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` là số thứ tự phân mảnh gốc 0, `data` là lát cắt byte của mảnh đó, trả về `true` nghĩa là xác thực đạt.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Trả về kích thước phân mảnh là lũy thừa của 2 nằm giữa `MIN_CHUNK` (`64 KB`) và `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Nối hậu tố hằng `PART_EXT` (`".part"`), trả về đường dẫn tệp tạm để tải tiếp.

### `Error` và `Result`

Loại trả về thống nhất `Result<T> = std::result::Result<T, Error>`, các biến thể enum lỗi bao gồm: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
