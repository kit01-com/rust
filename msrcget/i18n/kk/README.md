# msrcget

[compio](https://crates.io/crates/compio) асинхронды аяқтау моделіне негізделген көп mirror бөліктік HTTP/HTTPS жүктеу кітапханасы; жалғыз mirror жылдамдық шектеуін, бөліктік деректердің зақымдануы және баяу тораптардың кешігу мәселесін шешуге арналған.

## Өнімділік бағалауы

[down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) және [ureq](https://crates.io/crates/ureq) кітапханаларын қоғамдық HTTPS mirror дереккөздеріндегі жүктеу өткізгіштігі бойынша салыстыру:

### Көрініс 1: Көп қоғамдық mirror топталған жүктеу

Node.js v22.14.0 таралым пакетін (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB) жүктеу; mirror реті бойынша: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (302 mirror аралық қайта бағыттау қоса).

| Кітапхана / Шешім | Өткізгіштік (MB/с) |
| --- | ---: |
| msrcget (5 mirror, 8 параллель) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 mirror) | 1.2 |
| cyper (жалғыз mirror) | 14.4 |
| reqwest (жалғыз mirror) | 14.4 |
| ureq (жалғыз mirror) | 14.3 |

### Көрініс 2: Жылдам/баяу mirror аралас жүктеу

Node.js v22.14.0 тақырыптар пакетін (`node-v22.14.0-headers.tar.gz`, 8.4 MB) жүктеу; mirror реті бойынша: Aliyun (жалғыз қосылым жылдамдық шектеуі), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js ресми mirror (шекаралық баяу торап).

| Кітапхана / Шешім | Өткізгіштік (MB/с) |
| --- | ---: |
| msrcget (5 mirror, 8 параллель) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 mirror) | 0.2 |
| cyper (жалғыз mirror) | 6.8 |
| reqwest (жалғыз mirror) | 6.6 |
| ureq (жалғыз mirror) | 6.7 |

### Тестілеу әдісі

Бағалау коды `examples/bench.rs` файлында орналасқан (`cargo run --profile bench --example bench` орындау):

- Негізгі дайындық: Уақыт өлшеуден бұрын мақсатты файлды бір рет алдын ала жүктеп, байт ұзындығын тексеру және бүкіл пакеттің және әр бөліктің CRC32 базалық мәндерін есептеу; бұл кезең уақытқа есептелмейді.
- Уақыт шектеуі: Әр шешім үздіксіз 3 дөңгелек бойынша тәуелсіз орындалады; әр дөңгелек басталмас бұрын шығыс файл мен `.part` уақытша файл жойылады. Бір дөңгелек желілік сұраныс жіберілген уақыттан басталып, файл дискке жазылу аяқталғанша өлшенеді (`cyper`, `reqwest`, `ureq` эксплицитті түрде `sync_data` арқылы дискке жууды орындайды; `msrcget` қашықталма ұзындықты зондаудан, сирек файл жасаудан, параллель бөліктік жүктеуден, `sync_data` дискке жуудан, соңғы метадеректерді қысқартудан және атомдық атайлаудан тұратын толық процессті қамтиды).
- Өнім тексеру: Әр дөңгелек уақыт өлшеу тоқтаған соң дискке жазылған файлды ағынды түрде оқып, файл байт ұзындығын және бүкіл пакет CRC32-ін тексеру; тексеру процесі жүктеу уақытына есептелмейді.
- Өткізгіштікті түрлендіру: 3 дөңгелек уақытының медианасын алып, `файл байт саны / 1048576 / медианалық секунд` формуласы бойынша өткізгіштікті есептеу (`1 MB = 1048576 байт`).
- Mirror тағайындау ережесі: Көп mirror шешімдер (`msrcget`, `down`) көрініс ішіндегі бүкіл 5 mirror мекенжайын береді; жалғыз mirror шешімдер (`cyper`, `reqwest`, `ureq`) көп mirror бөліктікті қолдамайды, сұраныс тізімінің 1-ші mirror мекенжайын қолданады.

## Мүмкіндіктер

- Бөлік сайын тексеру және ақауға ауысу: Шақырушы `Verify` тексеру интерфейсі арқылы әр бөлікті растайды; тексеру сәтсіз немесе желілік сұраныс қателескенде, автоматты түрде қалған басылмаған mirror дереккөздеріне ауысып қайта отырады, бүкіл mirrorларды шарлап та сәтсіз болса `Error::Exhausted` қайтарады.
- Сирек файл және үзілісті жалғау: Файл жалпы ұзындығы бойынша алдын ала сирек `.part` уақытша файл жасалып файл құбыласы қойылады (құбылас қақтығысы `Error::Locked` қайтарады); файл соңында қос слотты пинг-понг CRC32 битті картасы толықтырылған бөліктерді ұзақ мерзімді сақтау үшін жазылады, үзіліс/қайта бастағаннан кейін автоматты жалғау жүреді, барлығы аяқталған соң соңғы метадеректер қысқартылып атомдық атайлау арқылы мақсатты файлға айналдырылады.
- Біріккен міндет ұрлау: Өз аралығының жүктеуін бірінші аяқтаған mirror қалған ең үлкен аяқталмаған аралықты екіге бөліп екінші жартысын қабылдайды, mirrorлар арасындағы жүктемені тепе-теңдейді.
- Соңғы кезеңдегі көп жолды жарыс: Бос когория саны қалған аяқталмаған бөліктер санынан көп болғанда, сол бөлікті басылмаған mirrorларға артық сұраныстар жіберіледі; бірінші тексеруден өткен дерек дискке жазылған соң, сол бөліктің қалған сұраныстары лезде болдырылады.
- Автоматты бөліктік жоспарлау: Файл жалпы ұзындығы мен параллель саны бойынша бөлік өлшемін жоспарлайды; бар `.part` файлды жалғастырғанда онда жазылған бөлік өлшемін автоматты қабылдайды.
- Қосылым қайта пайдалану және протокол қолдау: Ішкі бір mirror қосылым пулы арқылы HTTP/1.1 ұзақ қосылымдарды қайта пайдаланады; бөліктік беру кодтауы, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` жол аралық және mirror аралық қайта бағыттау, сондай-ақ таймауды басқаруды қолдайды.

## Қолдану демонстрациясы

Мысал коды `examples/basic.rs` файлында орналасқан (`cargo run --example basic` орындау):

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

/// CRC32 негізделген бөлік сайын тексеруші
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

  // mirror 0 1-ші бөлігі зақымданған деректерді қайтарады, mirror 1 жылдам қалыпты, mirror 2 жоғары кешікті баяу торап
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

  // Оңтайлы көп mirror параллель жүктеу: бөлік өлшемін автоматты жоспарлау, сирек .part жазып қысқартып атайлау
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() аяқталды: {size_a} байт, автоматты ұсынылған бөлік {} байт",
    plan_chunk_size(size_a, 8)
  );

  // Бөлік сайын CRC32 тексеруі бар көп mirror жүктеу: зақымданған бөліктерге автоматты mirror ауыстыру, жылдам mirror автоматты ұрлау және соңғы кезеңде баяу mirrorмен жарыс
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
  println!("Get::verify() тексеріліп аяқталды: {size_b} байт");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Интерфейс сипаттамасы

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

`CONC` (`8`) оңтайлы параллель саны мен `TIMEOUT` (`30с`) оңтайлы таймаут арқылы `url_li`-ні `path`-ке жүктеу, файл жалпы байт санын қайтарады.

### `Get`

Құрылыс кірісі `Get::new(url_li, path)`, тізбекті қонфигурацияны қолдайды:

- `.size(size: u64)`: Белгілі файл жалпы байт санын эксплицитті көрсету, қашықталма ұзындық зондаудан өту.
- `.chunk_size(chunk_size: usize)`: Бір бөлік байт санын көрсету (`0` болса `Error::ChunkSize` қайтарады).
- `.conc(conc: usize)`: Параллель жүктеу когория санын көрсету.
- `.timeout(timeout: Duration)`: Бір жолғы сұраныс және оқу таймаутын көрсету.
- `.verify<V2: Verify>(verify: V2)`: Тексерушіні енгізу; `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` және `Arc<T>` үшін іске асырылған.
- `.run().await`: Жүктенуді бастап `Result<u64>` қайтару.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` — 0-ден басталатын бөлік нөмірі, `data` — сол бөліктің байт кесіндісі, `true` қайтару тексеруден өтті дегенді білдіреді.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

`MIN_CHUNK` (`64 KB`) мен `MAX_CHUNK` (`4 MB`) аралығында 2-нің дәрежесі бөлік байт санын қайтарады.

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Жоғарғы констант `PART_EXT` (`".part"`) қосып, жалғау уақытша файл жолын қайтарады.

### `Error` және `Result`

Бірыңғай қайтару түрі `Result<T> = std::result::Result<T, Error>`, қателік enum нұсқалары: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
