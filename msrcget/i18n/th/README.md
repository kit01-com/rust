# msrcget

ไลบรารีดาวน์โหลด HTTP/HTTPS แบบแบ่งบล็อกจากหลายแหล่ง โดยอาศัยโมเดลการเสร็จสิ้นแบบไม่ซิงโครนัสของ [compio](https://crates.io/crates/compio) เพื่อแก้ปัญหาการจำกัดอัตราของมิร์ลอร์เดียว ข้อมูลบล็อกเสียหาย และปัญหาโหนดช้าดึงรั้ง.

## การทดสอบประสิทธิภาพ

เปรียบเทียบ throughput การดาวน์โหลดของ [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) และ [ureq](https://crates.io/crates/ureq) บนมิร์ลอร์ HTTPS ในเครือข่ายสาธารณะ:

### สถานการณ์ 1: การดาวน์โหลดรวมจากมิร์ลอร์สาธารณะหลายแห่ง

ดาวน์โหลดแพ็กเกจ Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB) มิร์ลอร์เรียงตามลำดับ: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (รวมการ redirect ข้ามแหล่ง 302).

| ไลบรารี / แนวทาง | Throughput (MB/s) |
| --- | ---: |
| msrcget (5 แหล่ง, 8 แบบขนาน) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 แหล่ง) | 1.2 |
| cyper (แหล่งเดียว) | 14.4 |
| reqwest (แหล่งเดียว) | 14.4 |
| ureq (แหล่งเดียว) | 14.3 |

### สถานการณ์ 2: การดาวน์โหลดผสมแหล่งเร็ว-ช้า

ดาวน์โหลดแพ็กเกจ header ของ Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8.4 MB) มิร์ลอร์เรียงตามลำดับ: Alibaba Cloud (จำกัดอัตราต่อ connection เดียว), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js ทางการ (โหนดช้าข้ามประเทศ).

| ไลบรารี / แนวทาง | Throughput (MB/s) |
| --- | ---: |
| msrcget (5 แหล่ง, 8 แบบขนาน) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 แหล่ง) | 0.2 |
| cyper (แหล่งเดียว) | 6.8 |
| reqwest (แหล่งเดียว) | 6.6 |
| ureq (แหล่งเดียว) | 6.7 |

### วิธีการทดสอบ

โค้ดทดสอบอยู่ที่ `examples/bench.rs` (รัน `cargo run --profile bench --example bench`):

- เตรียม baseline: ก่อนจับเวลาให้ดาวน์โหลดไฟล์เป้าหมายล่วงหน้าหนึ่งครั้ง ตรวจสอบความยาวไบต์ และคำนวณค่า CRC32 อ้างอิงทั้งไฟล์และแต่ละบล็อก ระยะนี้ไม่คิดเป็นเวลา.
- ขอบเขตการจับเวลา: แต่ละแนวทางรันแบบอิสระ 3 รอบติดต่อกัน ก่อนเริ่มแต่ละรอบให้ลบไฟล์ผลลัพธ์และไฟล์ชั่วคราว `.part`; รอบเดียวเริ่มจับเวลาตั้งแต่ส่งคำขอเครือข่าย จนไฟล์เขียนลงดิสก์เสร็จ (ทั้ง `cyper`, `reqwest`, `ureq` ล้วนรัน `sync_data` อย่างชัดแจ้ง; `msrcget` ครอบคลุมตั้งแต่ตรวจความยาวระยะไกล สร้างไฟล์ sparse ดาวน์โหลดบล็อกขนานกัน `sync_data` ตัด metadata ท้ายไฟล์ และ rename แบบ atomic ทั้งหมด).
- ตรวจสอบผลลัพธ์: หลังหยุดจับเวลาแต่ละรอบ อ่านไฟล์ที่เขียนลงดิสก์แบบสตรีม ตรวจสอบความยาวไบต์และ CRC32 ของทั้งไฟล์ กระบวนการตรวจสอบไม่คิดเป็นเวลาดาวน์โหลด.
- คำนวณ throughput: นำค่ามัธยฐานของเวลา 3 รอบ มาคำนวณ throughput ด้วยสูตร `จำนวนไบต์ของไฟล์ / 1048576 / ค่ามัธยฐานเป็นวินาที` (`1 MB = 1048576 ไบต์`).
- กฎการกระจายแหล่ง: แนวทางหลายแหล่ง (`msrcget`, `down`) ส่งที่อยู่มิร์ลอร์ทั้ง 5 แห่งในสถานการณ์; แนวทางแหล่งเดียว (`cyper`, `reqwest`, `ureq`) ไม่รองรับการแบ่งบล็อกหลายแหล่ง ใช้เฉพาะที่อยู่มิร์ลอร์ตัวที่ 1 ในรายการ.

## คุณลักษณะ

- ตรวจสอบแต่ละบล็อกและสลับแหล่งอัตโนมัติ: ผู้เรียกใช้ส่งอินเทอร์เฟส `Verify` เพื่อตรวจสอบแต่ละบล็อก; เมื่อการตรวจสอบไม่ผ่านหรือคำขอเครือข่ายล้มเหลว ระบบจะสลับไปมิร์ลอร์ที่ยังไม่ได้ลองโดยอัตโนมัติ หากลองทุกแหล่งแล้วยังล้มเหลวจะคืนค่า `Error::Exhausted`.
- ไฟล์ sparse และ resume เมื่อหยุด: สร้างไฟล์ชั่วคราว `.part` แบบ sparse ล่วงหน้าตามความยาวรวมของไฟล์ และล็อกไฟล์ (หากชนกันคืนค่า `Error::Locked`); เขียน bitmap CRC32 แบบ ping-pong สองสล็อตที่ท้ายไฟล์เพื่อเก็บสถานะบล็อกที่เสร็จแล้ว เมื่อรีสตาร์ทจะ resume อัตโนมัติ เมื่อเสร็จทั้งหมดจะตัด metadata ท้ายไฟล์และ rename แบบ atomic เป็นไฟล์เป้าหมาย.
- Task stealing แบบแบ่งครึ่ง: แหล่งที่ดาวน์โหลดช่วงของตัวเองเสร็จก่อนจะแบ่งช่วงที่ยังไม่เสร็จซึ่งมีช่วงกว้างที่สุดเป็นสองส่วน แล้วรับช่วงหลังไปทำ เพื่อสมดุลโหลดของแต่ละมิร์ลอร์.
- Race หลายทางช่วงท้าย: เมื่อจำนวน coroutine ว่างเกินบล็อกที่เหลือที่ยังไม่เสร็จ ส่งคำขอซ้ำไปยังมิร์ลอร์ที่ยังไม่ได้ลองบล็อกนั้น; เมื่อข้อมูลบล็อกแรกที่ผ่านตรวจสอบเขียนลงดิสก์แล้ว ยกเลิกคำขอบล็อกเดียวกันที่เหลือทันที.
- วางแผนขนาดบล็อกอัตโนมัติ: กำหนดขนาดบล็อกตามความยาวรวมของไฟล์และจำนวน concurrency; เมื่อ resume จากไฟล์ `.part` ที่มีอยู่ จะใช้ขนาดบล็อกที่บันทึกไว้โดยอัตโนมัติ.
- การ reuse connection และรองรับโพรโทคอล: มี connection pool สำหรับ reuse HTTP/1.1 keep-alive ของแหล่งเดียวกัน รองรับ chunked transfer encoding, HTTPS ([rustls](https://crates.io/crates/rustls)), redirect ข้าม path และข้ามแหล่ง `301`/`302`/`303`/`307`/`308` และการควบคุม timeout.

## การทดลองใช้งาน

โค้ดตัวอย่างอยู่ที่ `examples/basic.rs` (รัน `cargo run --example basic`):

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

/// ตัวตรวจสอบบล็อกละบล็อกแบบ CRC32
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

  // แหล่ง 0 บล็อกแรกคืนข้อมูลเสียหาย แหล่ง 1 เร็วปกติ แหล่ง 2 โหนดช้า latency สูง
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

  // ดาวน์โหลดขนานหลายแหล่งโดยค่าเริ่มต้น: วางแผนขนาดบล็อกอัตโนมัติ เขียน .part แบบ sparse แล้วตัดและ rename
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // ดาวน์โหลดหลายแหล่งพร้อมตรวจสอบ CRC32 บล็อกละบล็อก: บล็อกเสียสลับแหล่งอัตโนมัติ แหล่งเร็ว steal งานและ race กับแหล่งช้าช่วงท้าย
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

## คำอธิบายอินเทอร์เฟส

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

ดาวน์โหลด `url_li` ไปยัง `path` โดยใช้จำนวน concurrency ค่าเริ่มต้น `CONC` (`8`) และ timeout ต่อคำขอค่าเริ่มต้น `TIMEOUT` (`30s`) คืนค่าจำนวนไบต์รวมของไฟล์.

### `Get`

จุดเริ่มต้น builder `Get::new(url_li, path)` รองรับ chain configuration:

- `.size(size: u64)`: ระบุจำนวนไบต์รวมของไฟล์ที่ทราบไว้ล่วงหน้า ข้ามการตรวจความยาวระยะไกล.
- `.chunk_size(chunk_size: usize)`: ระบุจำนวนไบต์ต่อบล็อก (ส่ง `0` จะคืนค่า `Error::ChunkSize`).
- `.conc(conc: usize)`: ระบุจำนวน coroutine ดาวน์โหลดแบบขนาน.
- `.timeout(timeout: Duration)`: ระบุ timeout สำหรับคำขอและการอ่านแต่ละครั้ง.
- `.verify<V2: Verify>(verify: V2)`: เสิร์ฟตัวตรวจสอบ มี implement สำหรับ `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` และ `Arc<T>`.
- `.run().await`: เริ่มดาวน์โหลดและคืนค่า `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` คือลำดับบล็อกเริ่มจาก 0, `data` คือ slice ไบต์ของบล็อกนั้น คืนค่า `true` หมายถึงผ่านการตรวจสอบ.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

คืนค่าขนาดบล็อกที่เป็นกำลังสองของ 2 ระหว่าง `MIN_CHUNK` (`64 KB`) ถึง `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

ผูก suffix คงที่ `PART_EXT` (`.part`) คืนค่า path ของไฟล์ชั่วคราวสำหรับ resume.

### `Error` และ `Result`

ชนิดการคืนค่ารวม `Result<T> = std::result::Result<T, Error>` ตัวแปรของ enum ความผิดพลาด ได้แก่: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
