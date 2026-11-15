# msrcget

[compio](https://crates.io/crates/compio) asenk tamamlama modeline dayalı çok kaynaklı parçalı HTTP/HTTPS indirme kütüphanesi; tek aynada hız sınırı, parça verisi bozulması ve yavaş düğüm gecikmesi sorunlarını çözmek amacıyla tasarlanmıştır.

## Performans Değerlendirmesi

[down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) ve [ureq](https://crates.io/crates/ureq) kütüphanelerinin genel ağ HTTPS ayna kaynaklarındaki indirme verimliliği karşılaştırması:

### Senaryo 1: Çoklu genel ayna birleştirilmiş indirme

Node.js v22.14.0 dağıtım paketi (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB) indirilir; aynalar sırasıyla: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (302 kaynağarası yönlendirme dahil).

| Kütüphane / Çözüm | Verimlilik (MB/s) |
| --- | ---: |
| msrcget (5 kaynak, 8 eşzamanlı) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 kaynak) | 1.2 |
| cyper (tek kaynak) | 14.4 |
| reqwest (tek kaynak) | 14.4 |
| ureq (tek kaynak) | 14.3 |

### Senaryo 2: Hızlı-yavaş kaynak karışık indirme

Node.js v22.14.0 başlık dosyası paketi (`node-v22.14.0-headers.tar.gz`, 8.4 MB) indirilir; aynalar sırasıyla: Alibaba Cloud (tek bağlantı hız sınırı), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js resmi kaynağı (kıtalararası yavaş düğüm).

| Kütüphane / Çözüm | Verimlilik (MB/s) |
| --- | ---: |
| msrcget (5 kaynak, 8 eşzamanlı) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 kaynak) | 0.2 |
| cyper (tek kaynak) | 6.8 |
| reqwest (tek kaynak) | 6.6 |
| ureq (tek kaynak) | 6.7 |

### Test Yöntemi

Değerlendirme kodu `examples/bench.rs` dosyasında bulunur (`cargo run --profile bench --example bench` çalıştırılır):

- Taban hazırlık: Zamanlama başlamadan önce hedef dosya bir kez önceden indirilir, bayt uzunluğu kontrol edilir ve tüm paket ile her parça için CRC32 referans değerleri hesaplanır; bu aşama zamanlamaya dahil edilmez.
- Zamanlama sınırları: Her çözüm bağımsız olarak 3 tur boyunca ardışık çalıştırılır; her turun başında çıktı dosyası ve `.part` geçici dosyası silinir; tek tur, ağ isteğinin başlatıldığı an başlar ve dosyanın diske yazımı tamamlandığında biter (`cyper`, `reqwest`, `ureq` hepsi açıkça `sync_data` ile disk yazdırma yapar; `msrcget` uzak uzunluk keşfi, seyrek dosya oluşturma, eşzamanlı parçalı indirme, `sync_data` disk yazdırma, kuyrukta meta veri kısaltma ve atomik yeniden adlandırma dâhil tüm akışı kapsar).
- Ürün doğrulama: Her turda zamanlama durdurulduktan sonra diske yazılan dosya akış okunarak bayt uzunluğu ve tüm paket CRC32'si doğrulanır; doğrulama süreci indirme zamanına dahil edilmez.
- Verimlilik dönüşümü: 3 turun süresinin medyanı alınır ve `dosya bayt sayısı / 1048576 / medyan saniye` formülüyle verimlilik hesaplanır (`1 MB = 1048576 bayt`).
- Kaynak atama kuralı: Çok kaynaklı çözümler (`msrcget`, `down`) senaryodaki tüm 5 ayna adresini alır; tek kaynaklı çözümler (`cyper`, `reqwest`, `ureq`) çok kaynaklı parçalama desteklemez, istek listesinin 1. ayna adresini kullanır.

## Özellikler

- Parça parça doğrulama ve arıza değiştirme: Çağıran taraf, her parçayı doğrulamak için `Verify` doğrulama arayüzünü iletir; doğrulama başarısız olursa veya ağ isteği başarısız olursa, henüz denememiş diğer ayna kaynaklarına otomatik olarak geçilerek yeniden deneme yapılır; tüm kaynaklar denendiğinde hâlâ başarısızsa `Error::Exhausted` döndürülür.
- Seyrek dosya ve kesintiye devam: Dosyanın toplam uzunluğuna göre seyrek `.part` geçici dosya önceden oluşturulur ve dosya kilidi uygulanır (kilit çakışması `Error::Locked` döndürür); dosya sonunda çift slot ping-pong CRC32 biti kalıbı yazılarak tamamlanmış parçalar kalıcı hale getirilir, kesinti sonrasında otomatik olarak devam edilir, tamamlandıktan sonra kuyrukta meta veri kısaltılır ve hedef dosyaya atomik olarak yeniden adlandırılır.
- İkili görev ele geçirme: Aralığını ilk tamamlayan kaynak, kalan en büyük tamamlanmamış aralığı ikiye böler ve ikinci yarısını devralır; böylece her ayna kaynağındaki yük dengelenir.
- Kuyrukta çoklu yol yarışması: Boşta bekleyen eşzamanlı iş birimi sayısı, kalan tamamlanmamış parça sayısından fazlaysa, henüz bu parçayı denememiş ayna kaynaklarına yedek istekler gönderilir; ilk doğrulaması geçen veri diske yazılır yazılmaz aynı parçaya ait diğer istekler anında iptal edilir.
- Otomatik parça planlaması: Dosyanın toplam uzunluğuna ve eşzamanlılık sayısına göre parça boyutu planlanır; mevcut `.part` dosyasından devam edildiğinde kayıtlı parça boyutu otomatik olarak kullanılır.
- Bağlantı yeniden kullanımı ve protokol desteği: Dahili aynı kaynak bağlantı havuzunda HTTP/1.1 uzun bağlantılar yeniden kullanılır; parçalı aktarım kodlaması, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` yol ve kaynakarası yönlendirmeler ile zaman aşımı kontrolü desteklenir.

## Kullanım Örneği

Örnek kod `examples/basic.rs` dosyasında bulunur (`cargo run --example basic` çalıştırılır):

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

/// CRC32 tabanlı parça parça doğrulayıcı
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

  // Kaynak 0, 1. parçayı bozuk veri olarak döndürür; kaynak 1 hızlı ve normal; kaynak 2 yüksek gecikmeli yavaş düğüm
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

  // Varsayılan çok kaynaklı eşzamanlı indirme: parça boyutu otomatik planlanır, seyrek .part dosyası yazıldıktan sonra kısaltılıp yeniden adlandırılır
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Parça parça CRC32 doğrulamalı çok kaynaklı indirme: bozuk parça otomatik olarak kaynağı değiştirir, hızlı kaynak otomatik görev ele geçirir ve kuyrukta yavaş kaynakla yarışır
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

## Arayüz Açıklaması

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Varsayılan eşzamanlılık sayısı `CONC` (`8`) ve varsayılan tek istek zaman aşımı `TIMEOUT` (`30s`) ile `url_li` listesinden `path` konumuna indirme yapar; dosyanın toplam bayt sayısını döndürür.

### `Get`

Kurucu giriş noktası `Get::new(url_li, path)`; zincirleme yapılandırma desteklenir:

- `.size(size: u64)`: Bilinen dosya toplam bayt sayısını açıkça belirtir, uzak uzunluk keşfini atlar.
- `.chunk_size(chunk_size: usize)`: Tek parça bayt sayısını belirtir (`0` verilirse `Error::ChunkSize` döner).
- `.conc(conc: usize)`: Eşzamanlı indirme iş birimi sayısını belirtir.
- `.timeout(timeout: Duration)`: Tek istek ve okuma zaman aşımını belirtir.
- `.verify<V2: Verify>(verify: V2)`: Doğrulayıcı enjekte eder; `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` ve `Arc<T>` için hazır implementasyona sahiptir.
- `.run().await`: İndirmeyi başlatır ve `Result<u64>` döndürür.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx`, 0 tabanlı parça sıra numarasıdır; `data`, ilgili parçanın bayt dilimidir; `true` döndürülmesi doğrulamanın geçtiğini gösterir.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

`MIN_CHUNK` (`64 KB`) ile `MAX_CHUNK` (`4 MB`) arasında 2'nin kuvveti olan parça bayt sayısını döndürür.

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Sone sabiti `PART_EXT` (`".part"`) eklenerek devam dosyası için geçici dosya yolunu döndürür.

### `Error` ve `Result`

Özel dönüş türü `Result<T> = std::result::Result<T, Error>`; hata sayısal varyantları şunları içerir: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
