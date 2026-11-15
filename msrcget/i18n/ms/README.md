# msrcget

Perpustakaan muat turun HTTP/HTTPS berbilang sumber dan bersegmen berasaskan model penyelesaian asinkron [compio](https://crates.io/crates/compio), untuk menyelesaikan isu had laju pantulan tunggal, kerosakan data bersegmen, dan kesan susulan node perlahan.

## Penilaian Prestasi

Perbandingan [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) dan [ureq](https://crates.io/crates/ureq) dari segi laju muat turun pada sumber pantulan HTTPS rangkaian awam：

### Senario 1: Muat turun gabungan berbilang pantulan awam

Muat turun pakej edaran Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), susunan pantulan: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (termasuk pengalihan 302 rentas sumber).

| Perpustakaan / Pelan | Laju (MB/s) |
| --- | ---: |
| msrcget (5 sumber, 8 serentak) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 sumber) | 1.2 |
| cyper (sumber tunggal) | 14.4 |
| reqwest (sumber tunggal) | 14.4 |
| ureq (sumber tunggal) | 14.3 |

### Senario 2: Muat turun campuran sumber pantas dan perlahan

Muat turun pakej kepala fail Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8.4 MB), susunan pantulan: Alibaba Cloud (had laju sambungan tunggal), Tsinghua, Huawei Cloud, Tencent Cloud, sumber rasmi Node.js (node perlahan rentas sempadan).

| Perpustakaan / Pelan | Laju (MB/s) |
| --- | ---: |
| msrcget (5 sumber, 8 serentak) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 sumber) | 0.2 |
| cyper (sumber tunggal) | 6.8 |
| reqwest (sumber tunggal) | 6.6 |
| ureq (sumber tunggal) | 6.7 |

### Kaedah Ujian

Kod penilaian terletak di `examples/bench.rs` (jalankan `cargo run --profile bench --example bench`)：

- Persediaan garisan asas: Sebelum permulaan kiraan masa, muat turun fail sasaran terlebih dahulu, sahkan panjang bait dan kira nilai asas CRC32 bagi pakej penuh serta setiap segmen; fasa ini tidak dikira dalam masa.
- Sempatan masa: Setiap pelan dijalankan secara berasingan selama 3 pusingan; sebelum setiap pusingan, buang fail keluaran dan fail sementara `.part`; pusingan tunggal bermula kiraan masa daripada permulaan permintaan rangkaian hingga fail selesai ditulis ke cakera (semua `cyper`, `reqwest`, `ureq` melaksanakan `sync_data` secara eksplisit untuk menyegerakkan ke cakera; `msrcget` merangkumi penjodohan panjang jarak jauh, penciptaan fail jarang, muat turun serentak bersegmen, `sync_data` penyegerakan ke cakera, pemangkasan metadata hujung dan penama semula atomik).
- Pengesahan hasil: Selepas setiap pusingan berhenti mengira masa, baca fail yang ditulis secara penstriman, sahkan panjang bait fail dan CRC32 pakej penuh; proses pengesahan tidak dikira dalam masa muat turun.
- Pengiraan laju: Ambil nilai median masa 3 pusingan, kira laju mengikut `bilangan bait fail / 1048576 / saat median` (`1 MB = 1048576 bait`).
- Peraturan peruntukan sumber: Pelan berbilang sumber (`msrcget`, `down`) memasukkan semua 5 alamat pantulan dalam senario; pelan sumber tunggal (`cyper`, `reqwest`, `ureq`) tidak menyokong pembahagian berbilang sumber, guna alamat pantulan ke-1 dalam senarai permintaan.

## Ciri-ciri

- Pengesahan bersegmen dan pembetulan kecacatan: Pemanggil memasukkan antara muka pengesahan `Verify` untuk mengesahkan setiap segmen; apabila pengesahan gagal atau permintaan rangkaian rosak, secara automatik bertukar ke pantulan lain yang belum dicuba untuk cuba lagi, jika semua sumber gagal kembalikan `Error::Exhausted`.
- Fail jarang dan sambungan semula: Buat awal cipta fail sementara `.part` jarang mengikut panjang keseluruhan fail dan kunci fail (pertembungan kunci kembalikan `Error::Locked`); tulis peta CRC32 dwi-lotak ping-pong pada hujung fail untuk mendabarkan segmen yang siap, selepas gangguan dan mula semula sambung secara automatik, setelah semua siap pangkas metadata hujung dan namakan semula secara atomik ke fail sasaran.
- Pencuri tugang pembahagi dua: Pantulan yang selesai julat sendiri terlebih dahulu bahagikan dua julat tidak selesai yang paling besar dan ambil alih separuh belakang, mengimbangi beban antara setiap pantulan.
- Lumba berbilang laluan penghujung: Apabila bilangan koren idle melebihi bilangan segmen tidak selesai, kirim permintaan lebihi kepada pantulan yang belum mencuba segmen tersebut; data pertama yang lulus pengesahan ditulis ke cakera kemudian batalkan permintaan lain bagi segmen yang sama.
- Perancangan pemutaran automatik: Rancang saiz segmen mengikut panjang keseluruhan fail dan bilangan serentak; semasa sambung fail `.part` sedia ada, secara automatik gunakan saiz segmen yang direkodkan.
- Penggunaan semula sambungan dan sokongan protokol: Kolam sambungan terbina dalam untuk penggunaan semula sambungan panjang HTTP/1.1 pantulan yang sama, menyokong kod penghantaran bersegmen, HTTPS ([rustls](https://crates.io/crates/rustls)), pengalihan `301`/`302`/`303`/`307`/`308` rentas laluan dan rentas sumber serta kawalan tamat masa.

## Demo Penggunaan

Kod contoh terletak di `examples/basic.rs` (jalankan `cargo run --example basic`)：

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

/// Pengesah bersegmen berasaskan CRC32
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

  // Pantulan 0 segmen 1 mengembalikan data rosak, pantulan 1 pantas normal, pantulan 2 node perlahan berkelewatan tinggi
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

  // Muat turun berbilang sumber serentak lalai: rancang saiz segmen secara automatik, tulis fail jarang .part kemudian pangkas dan namakan semula
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Muat turun berbilang sumber dengan pengesahan CRC32 bersegmen: segmen rosak automatik tukar pantulan, pantulan pantas automatik curi tugang dan lumba penghujung dengan pantulan perlahan
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

## Penerangan Antara Muka

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Muat turun `url_li` ke `path` menggunakan bilangan serentak lalai `CONC` (`8`) dan tamat masa per permintaan lalai `TIMEOUT` (`30s`), kembalikan jumlah bait fail.

### `Get`

Masuk pembina `Get::new(url_li, path)`, menyokong konfigurasi berantai：

- `.size(size: u64)`: Nyatakan secara eksplisit jumlah bait fail yang diketahui, skip penjodohan panjang jarak jauh.
- `.chunk_size(chunk_size: usize)`: Nyatakan bilangan bait per segmen (melembarkan `0` kembalikan `Error::ChunkSize`).
- `.conc(conc: usize)`: Nyatakan bilangan koren muat turun serentak.
- `.timeout(timeout: Duration)`: Nyatakan tamat masa permintaan dan baca per kali.
- `.verify<V2: Verify>(verify: V2)`: Suntik pengesah, telah dilaksanakan untuk `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` dan `Arc<T>`.
- `.run().await`: Mula muat turun dan kembalikan `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` ialah nombor segmen asas 0, `data` ialah hirisan bait segmen tersebut, kembalikan `true` bermaksud pengesahan lulus.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Kembalikan bilangan bait segmen kuasa 2 antara `MIN_CHUNK` (`64 KB`) dan `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Sambung pemalar akhir `PART_EXT` (`".part"`), kembalikan laluan fail sementara sambungan.

### `Error` dan `Result`

Jenis kembalikan seragam `Result<T> = std::result::Result<T, Error>`, variasi enum ralat termasuk: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
