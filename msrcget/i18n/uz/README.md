# msrcget

[compio](https://crates.io/crates/compio) asinxikomiya modeliga asoslangan koʻp manbali blokli HTTP/HTTPS yuklab olish kutubxonasi, yagona oyna manbasi tezlik cheklovi, blokli maʼlumot buzilishi va sekin tugunlarning ortda qolishi muammolarini hal qilish uchun

## Ishlamchilik baholash

[down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) va [ureq](https://crates.io/crates/ureq) kutubxonalarini ochiq tarmoqdagi HTTPS oʻyna manbalari ostida yuklab olish unumdorligi boʻyicha solishtirish:

### 1-senariy: Koʻp ochiq oʻyna manbalaridan birlashtirilgan yuklab olish

Node.js v22.14.0 nashr paketi (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB) yuklab olinadi, oʻyna manbalar tartibi: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (302 manbalararo yoʻnaltirish bilan)

| Kutubxona / Yondashuv | Unumdorlik (MB/s) |
| --- | ---: |
| msrcget (5 manba, 8 parallel) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 manba) | 1.2 |
| cyper (yagona manba) | 14.4 |
| reqwest (yagona manba) | 14.4 |
| ureq (yagona manba) | 14.3 |

### 2-senariy: Tez va sekin manbalar aralashmasida yuklab olish

Node.js v22.14.0 sarlavha fayllar paketi (`node-v22.14.0-headers.tar.gz`, 8.4 MB) yuklab olinadi, oʻyna manbalar tartibi: Alibaba Cloud (yagona ulanish tezlik cheklovi), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js rasmiy manbasi (xalqaro chegara sekin tuguni)

| Kutubxona / Yondashuv | Unumdorlik (MB/s) |
| --- | ---: |
| msrcget (5 manba, 8 parallel) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 manba) | 0.2 |
| cyper (yagona manba) | 6.8 |
| reqwest (yagona manba) | 6.6 |
| ureq (yagona manba) | 6.7 |

### Test usuli

Baholash kodi `examples/bench.rs` faylida joylashgan (`cargo run --profile bench --example bench` ishga tushiriladi):

- Baza tayyorlash: vaqt oʻlchashdan oldin maqsad fayl bir marta oldindan yuklab olinadi, bayt uzunligi tekshiriladi va butun paket hamda har bir blok uchun CRC32 meʼyor qiymatlari hisoblanadi, bu bosqich sarf vaqqa kiritilmaydi
- Vaqt chegaralari: har bir yondashuv ketma-ket mustaqil 3 marta ishga tushiriladi, har bir raund boshlanishidan oldin chiqarish fayli va `.part` vaqtincha fayl oʻchiriladi; bitta raundda taymer tarmoq soʻrovini yuborishdan boshlanadi va fayl diskka yozilishi tugagunga qadar toʻxtatiladi (`cyper`, `reqwest`, `ureq` hammasi ochiq koʻrinishda `sync_data` orqali diskni tozalaydi; `msrcget` uzoq manba uzunligini aniqlash, noʻzlashtirilgan fayl yaratish, parallel blokli yuklab olish, `sync_data` orqali diskni tozalash, oxirgi metadatalarni kesish va atomar qayta nomlashning toʻliq jarayonini oʻz ichiga oladi)
- Mahsulotni tasdiqlash: har bir raundda taymer toʻxtatilgandan keyin oqimli oʻqishda fayl bayt uzunligi va butun paket CRC32 tekshiriladi, tasdiqlash jarayoni yuklab olish sarf vaqtiga kiritilmaydi
- Unumdorlik hisoblash: 3 raund sarf vaqtlarning medianasi olinadi va `fayl baytlari / 1048576 / medianaviy soniyalar` boʻyicha unumdorlik hisoblanadi (`1 MB = 1048576 bayt`)
- Manba taqsimoti qoidalari: koʻp manbali yondashuvlar (`msrcget`, `down`) senariydagi barcha 5 ta oʻyna manzilni oʻtkazadi; yagona manbali yondashuvlar (`cyper`, `reqwest`, `ureq`) koʻp manbali bloklanishni qoʻllab-quvvatlamaydi, soʻrov roʻyxatidagi 1-oʻyna manzili ishlatiladi

## Xususiyatlar

- Blokdan-blok tasdiqlash va nosozlikni almashtirish: chaqiruvchi tomon `Verify` tasdiqlash interfeysi orqali har bir blokni tekshiradi; tasdiqlash muvaffaqiyatsiz boʻlsa yoki tarmoq soʻrov xatolik boʻlsa, avtomatik ravishda hali sinab koʻrmagan qolgan oʻyna manbalariga yoʻnaltiriladi, barcha manbalar sinab koʻrilib ham xato boʻlsa `Error::Exhausted` qaytariladi
- Noʻzlashtirilgan fayl va uzilishdan davom ettirish: fayl umumiy uzunligi asosida noʻzlashtirilgan `.part` vaqtincha fayl oldindan yaratiladi va fayl qulfi oʻrnatiladi (qulf toʻqnashuvi boʻlsa `Error::Locked` qaytariladi); fayl oxirida ikki slotli pinq-pong CRC32 bitmap yoziladi va bajarilgan bloklar doimiy saqlanadi, uzilishdan soʻng avtomatik davom etadi, toʻliq tugagach oxirgi metadatalar kesiladi va atomar qayta nomlash orqali maqsad faylga aylantiriladi
- Ikki qismli vazifa oʻgʻirlash: oʻz intervalini eng avval tugatgan yuklab olish manbasi qolmagan eng katta oraliqni ikkiga boʻladi va ikkinchi yarmini oʻziga oladi, shunda har bir oʻyna manba yuklanishi muvozanatlanadi
- Oxirgi bosqichda koʻp yoʻnalishli yarish: boʻsh koreuntlar soni qolmagan bloklar sonidan koʻp boʻlsa, hali sinab koʻrmagan oʻyna manbalariga ortiqcha soʻrovlar yuboriladi; birinchi tasdiqlangan maʼlumot diskka yozilishi darhol shu blok boʻyicha qolgan soʻrovlarni bekor qiladi
- Avtomatik bloklanish rejasi: fayl umumiy uzunligi va parallel soniga koʻra blok oʻlchami rejalashtiriladi; mavjud `.part` faylda davom ettirilganda unda yozilgan blok oʻlchami avtomatik saqlanadi
- Ulanishni qayta ishlatish va protokollar qoʻllab-quvvatlash: ichki bir xil manba ulanish havuzi HTTP/1.1 uzun ulanishlarni qayta ishlatadi, blokli tashil kodlashi, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` yoʻl va manba orali yoʻnaltirish hamda vaqt oʻtish boshqaruvini qoʻllab-quvvatlaydi

## Ishlatish namunasi

Namuna kodi `examples/basic.rs` faylida joylashgan (`cargo run --example basic` ishga tushiriladi):

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

/// CRC32 asosidagi blokdan-blok tasdiqlagich
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

  // Manba 0 birinchi blok buzilgan maʼlumot qaytaradi, manba 1 yuqori tezlikda normal ishlaydi, manba 2 yuqori kechikishli sekin tugun
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

  // Oʻtkazilgan koʻp manbali parallel yuklab olish: avtomatik blok oʻlchamini rejalashtiradi, noʻzlashtirilgan .part yozilgach kesib qayta nomlaydi
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Blokdan-blok CRC32 tasdiqlash bilan koʻp manbali yuklab olish: buzilgan blok avtomatik manba almashtiriladi, tez manba avtomatik oʻgʻirlash va oxirgi bosqichda sekin manba bilan yarish oʻtkazadi
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

## Interfeys tavsifi

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Oʻtkazilgan parallel son `CONC` (`8`) va yagona soʻrov uchun oʻtkazilgan vaqt oʻtish `TIMEOUT` (`30s`) bilan `url_li` ni `path` ga yuklab oladi, faylning umumiy bayt sonini qaytaradi

### `Get`

Quruvchi kirish nuqtasi `Get::new(url_li, path)`, zanjirli sozlamalarni qoʻllab-quvvatlaydi:

- `.size(size: u64)`: maʼlum fayl umumiy bayt sonini ochiq koʻrsatadi, uzoq manba uzunligini aniqlashni oʻtkazib yuboradi
- `.chunk_size(chunk_size: usize)`: bitta blok bayt sonini belgilaydi (`0` berilsa `Error::ChunkSize` qaytariladi)
- `.conc(conc: usize)`: parallel yuklab olish koreuntlar sonini belgilaydi
- `.timeout(timeout: Duration)`: yagona soʻrov va oʻqish uchun vaqt oʻtish muddatini belgilaydi
- `.verify<V2: Verify>(verify: V2)`: tasdiqlagichni joylashtiradi, `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` va `Arc<T>` uchun amalga oshirilgan
- `.run().await`: yuklab olishni ishga tushiradi va `Result<u64>` qaytaradi

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` 0 asosli blok tartib raqami, `data` shu blok baytlar qismi, `true` qaytarilishi tasdiqlash muvaffaqiyatli ekanligini bildiradi

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

`MIN_CHUNK` (`64 KB`) va `MAX_CHUNK` (`4 MB`) orasida 2 ning darajasi boʻyicha blok bayt sonini qaytaradi

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Sufiks doimiyi `PART_EXT` (".part") ni qoʻshib, davomiy vaqtincha fayl yoʻlini qaytaradi

### `Error` va `Result`

Birlashtirilgan qaytarilish turi `Result<T> = std::result::Result<T, Error>`, xato enum variantlari: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`
