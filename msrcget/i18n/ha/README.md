# msrcget

Malamucin sauren HTTP/HTTPS mai karye bayanai a sassan daga tushen da yawa, wanda ya gina kan tsarin kammala a wuri mai wuya na [compio](https://crates.io/crates/compio), don magance matsalar iyakacin alamar guda ɗaya, lalatar bayanayen sassan, da kuma matsalar ƙananan tushen da ke haifar da janyo。

## Binciken Sarkin Sauri

Haɗaɗa [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) da [ureq](https://crates.io/crates/ureq) a kan sarkin sauren bayanai daga tushen alamar HTTPS a kan gine-ginen gabaɗɗi:

### Zamani 1: Sauyen haɗuwa daga tushen alamar gabaɗɗi da yawa

Sauren ƙungiyar rarraba Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), alamar ta biyo cikin tsari: Tsinghua, USTC, Huaweicloud, Tencent Cloud, npmmirror (da 302 canza gaba daga wani tushe zuwa wani)。

| Malamuci / Tsari | Sarkin Sauri (MB/s) |
| --- | ---: |
| msrcget (tushi 5, guda 8) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (tushi 5) | 1.2 |
| cyper (tushi 1) | 14.4 |
| reqwest (tushi 1) | 14.4 |
| ureq (tushi 1) | 14.3 |

### Zamani 2: Sauyen haɗuwa daga tushen sauri da janyo

Sauren ƙungiyar fayilan kolin Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8.4 MB), alamar ta biyo cikin tsari: Alibaba Cloud (iyakacin haɗin guda ɗaya), Tsinghua, Huaweicloud, Tencent Cloud, ƙungiyar asushe ta Node.js (tushe mai janyo ta tashar ƙasa)。

| Malamuci / Tsari | Sarkin Sauri (MB/s) |
| --- | ---: |
| msrcget (tushi 5, guda 8) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (tushi 5) | 0.2 |
| cyper (tushi 1) | 6.8 |
| reqwest (tushi 1) | 6.6 |
| ureq (tushi 1) | 6.7 |

### Hanyar Gwaji

Kodin bincike yana a `examples/bench.rs` (gargu da `cargo run --profile bench --example bench`):

- Shirye-shiryen ƙarƙashin: kafin fara lamba, a sauri fayil ɗin ma'ana sau ɗaya, a tantance tsayin bytes kuma a lissafa ma'aunin CRC32 na ƙungiya gaba ɗaya da sassan kowanne, wannan matakin ba a haɗa shi cikin lokaci ba。
- Ƙarshen lamba: kowanne tsari yana aiki cikin juri sau 3, kafin kowane zagayawa a cire fayilan fitarwa da ɗakin `.part`; kowane zagayawa fara lamba daga bin buƙa a kan hanyar waya har fayil ya cika a wuri (`cyper`, `reqwest`, `ureq` duk suna yin `sync_data` sosai; `msrcget` ya haɗa da binciken tsayin tushe, samun fayil mai 'yanci, sauren sassan guda-guda, `sync_data`, cire bayanayen ƙarshe da kuma canza suna cikin wuri ɗaya)。
- Tabbatar da sakamako: bayan ƙarsen lambar kowane zagayawa, a karanta fayil ɗin da aka cika a wuri a tafiya, a gwada tsayin bytes da CRC32 na ƙungiya gaba ɗaya, wannan aiki ba a haɗa shi a cikin lokacin sauri ba。
- Canjin sarkin sauri: a ɗauki tsakiya ta tsawon lokaci na zagayawa 3, a lissafa sarkin sauri da `Adadin Bytes / 1048576 / Tsakiyar Daƙiƙa` (`1 MB = 1048576 bytes`)。
- Ƙa'idar rarraba tushen: tsarin da yawa (`msrcget`, `down`) yana ɗauka ƙananan alamar gaba ɗaya 5 a cikin zamani; tsarin ɗaya (`cyper`, `reqwest`, `ureq`) ba su da goyon baya ga sassan da yawa, suna amfani da sunan alamar farko a cikin jerin buƙe。

## Sifa-shifa

- Gwada sassan kowanne da kuma canza idan akwai guba: mai ambati yana ɗauka tsarin gwada `Verify` don tabbatar da kowane sashi; idan gwadin bai yi nasara ba ko buƙar hanyar waya ya lalace, ya canza cikin sauri zuwa ƙananan alamar da ba a gwada su ba don sake gwada, idan an gwada tushen duka kuma ya lalace ya mayar da `Error::Exhausted`。
- Fayil mai 'yanci da ci gaba daga inda ya tsaya: a samu fayil mai 'yanci na `.part` na wuri ɗaya bisa tsayin fayil gaba ɗaya kuma a ƙara lauya (idan lauya ta haɗu ta mayar da `Error::Locked`); a ƙarshen fayil a rubuta CRC32 na wuri biyu na kwallayen da aka cika don a iya cika bayanai, bayan an keske kuma an sake farawa a ci gaba cikin sauri, bayan an kammala komai a cire bayanayen ƙarshe kuma a canza suna zuwa sunan fayil ɗin da a buƙata cikin wuri ɗaya。
- Ɓarcan aiki ta raba biyu: tushin da ya fara kammala sauren yankinsa yana raba yankin da har bai cika ba mafi girma zuwa gaba biyu kuma ya ɗauki ƙasan gaba, don daidaita janyar tushen alamar duka。
- Gwagwalin da yawa na ƙarshe: idan adadin tsare da ke hutto ya fi adadin sassan da har bai cika ba, sai a sake buƙe da yawa zuwa tushen alamar da ba su gwada wannan sashi ba; bayan da farkon bayanai ya wuce gwadi kuma ya cika a wuri, sai a soke sauran buƙe na wannan sashi。
- Kammala rarraba sassan cikin sauri: bisa tsayin fayil gaba ɗaya da adadin guda-guda a kammala girman sashi; idan aka ci gaba da fayil ɗin `.part` da ke akwai, ya biyo hankali da girman sashi da aka rubuta a ciki。
- Sake amfani da haɗe da goyon bayan tsarin: an sanya tushin haɗe na asushe don sake amfani da haɗen HTTP/1.1 masu tsawo, ya goyan baya ga rubutun rarraba, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` canza gaba daga wata hanya zuwa wata da kuma daga wani tushe zuwa wani, da kuma sarrafan ƙafin da a yi。

## Misalin Amfani

Kodin misali yana a `examples/basic.rs` (gargu da `cargo run --example basic`):

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

/// Tabbataccen sashi-kowanne ne bisa CRC32
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

  // Tushi 0 sashi 1 yana mai wa bayanai da suka lalata, tusi 1 yana sauri da kyau, tusi 2 mai janyo
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

  // Sauren tushen da yawa cikin sauri: rarraba girman sashi cikin sauri, bayan rubuta .part mai 'yanci a cire kuma a canza suna
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Sauren tushen da yawa tare da gwada CRC32 sashi-kowanne: sashi mara kyau ya canza tushe cikin sauri, tushe mai sauri ya ɓarci aiki da guguwa tare da tushe mai janyo
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

## Tsayin Hanyoyi

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Yana amfani da adadin guda-guda na asushe `CONC` (`8`) da ƙafin da a yi na asushe `TIMEOUT` (`30s`) don sauren `url_li` zuwa `path`, yana mayar da jimlar bytes na fayil。

### `Get`

Farkon gina `Get::new(url_li, path)`, yana goyon baya ga sariya:

- `.size(size: u64)`: a nuna jimlar bytes na fayil cikin hankali, a tsaure binciken tsayin tushe。
- `.chunk_size(chunk_size: usize)`: a nuna adadin bytes na sashi ɗaya (idan an ɗauki `0` ya mayar da `Error::ChunkSize`)。
- `.conc(conc: usize)`: a nuna adadin tsare na sauri guda-guda。
- `.timeout(timeout: Duration)`: a nuna ƙafin da a yi na buƙa da karantawa.
- `.verify<V2: Verify>(verify: V2)`: a sanya mai gwada, an gina shan `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` da `Arc<T>`。
- `.run().await`: a fara sauren kuma a mayar da `Result<u64>`。

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` lamba ce ta sashi ta asushe (0), `data` ɓangaren bytes na wannan sashi ne, a mayar da `true` yana nufin gwadi ya wuce。

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Yana mayar da adadin bytes na sashi mai ƙarfin 2 tsakanin `MIN_CHUNK` (`64 KB`) da `MAX_CHUNK` (`4 MB`)。

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

A haɗa suna na ƙarshe `PART_EXT` (`".part"`), a mayar da hanyar fayil na wuri ɗayan ci gaba。

### `Error` da `Result`

Tushe ɗaya na mayar da sakamako `Result<T> = std::result::Result<T, Error>`, kalmomin kuskure sun haɗa da: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`。
