# msrcget

በ [compio](https://crates.io/crates/compio) ባለ-ጊዜ ምድረታ ሞዴል ላይ የተመሰረተ ብዙ-ምንጭ ቆርቆሮ HTTP/HTTPS ማውረዳት መጽሐፍ፣ የፊደል መጋጠሚያ ፍጥነት ገዢ፣ የቆርቆሮ ዳታ መበላሽ እና የዘገይተኛ ማዕከላትን ማጠቃለል ችግሮችን ለመፍታት የተዘጋጀ ነው።

## የፍጥነት ምርመራ

[down](https://crates.io/crates/down)፣ [cyper](https://crates.io/crates/cyper)፣ [reqwest](https://crates.io/crates/reqwest) እና [ureq](https://crates.io/crates/ureq) በአልባድ HTTPS መጋጠሚያ ምንጮች ስር የማውረዳት ፍጥነትን ይነጻጸራል፦

### ቅስቀት 1፦ የብዙ አልባድ መጋጠሚያ ምንጮች ስር የተካፈለ ማውረዳት

Node.js v22.14.0 የሚለካ ገጽ (`node-v22.14.0-linux-x64.tar.xz`፣ 28.5 MB) ማውረድ፤ መጋጠሚያዎች በደረጃ፦ ቲንጻ፣ USTC፣ ሁኛይ ክላውድ፣ ቲንጄን ክላውድ፣ npmmirror (302 የምንጭ መቀየሪያ ጨምሮ)።

| መጽሐፍ / ዘዴ | ፍጥነት (MB/s) |
| --- | ---: |
| msrcget (5 ምንጮች፣ 8 ቅጥያ) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 ምንጮች) | 1.2 |
| cyper (ፊደል ምንጭ) | 14.4 |
| reqwest (ፊደል ምንጭ) | 14.4 |
| ureq (ፊደል ምንጭ) | 14.3 |

### ቅስቀት 2፦ የፈጣንና ዘገይተኛ ምንጮች ሚናማ ማውረዳት

Node.js v22.14.0 የመጀመሪያ ፋይል ገጽ (`node-v22.14.0-headers.tar.gz`፣ 8.4 MB) ማውረድ፤ መጋጠሚያዎች በደረጃ፦ አሊ ጄም (የፊደል ግንኙነት ገዢ)፣ ቲንጻ፣ ሁኛይ ክላውድ፣ ቲንጄን ክላውድ፣ Node.js ዋና ምንጭ (የዓለም አቀፍ ዘገይተኛ ማዕከል)።

| መጽሐፍ / ዘዴ | ፍጥነት (MB/s) |
| --- | ---: |
| msrcget (5 ምንጮች፣ 8 ቅጥያ) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 ምንጮች) | 0.2 |
| cyper (ፊደል ምንጭ) | 6.8 |
| reqwest (ፊደል ምንጭ) | 6.6 |
| ureq (ፊደል ምንጭ) | 6.7 |

### የመሞከሪያ ዘዴ

የምርመራ ኮድ በ `examples/bench.rs` ውስጥ ላለ (`cargo run --profile bench --example bench` ማከሰብ)፦

- የመገንገል ዝግጅት፦ ከጊዜ ማቆም በፊት ተመሳጣ ፋይልን አንግሁ በፊት ማውረድ፣ የባይት ርዝመትን ማረጋገጥ እና ጠቅላላ ገጽን እና እያንዳንዱን ቆርቆሮ CRC32 መለኪያ ክፍሎችን ማስላት፤ ይህ ደረጃ በጊዜው ውስጥ አይቆጠርም።
- የጊዜ ገደብ፦ እያንዳንዱ ዘዴ በቀጥሎ 3 ጊዜ ተለጠፍቶ ይሰራል፣ ከእያንዳንዱ ጊዜ ጀምሯል የመውጫ ፋይልን እና `.part` ጊዜያዊ ፋይልን ያወድቃል፤ አንድ ጊዜ ከውብ መጠይቅ መጀመሪያ እስከ ፋይል በዴስክ ላይ ሲጻፍ ድረስ ይቆማል (`cyper`፣ `reqwest`፣ `ureq` በአስፈላጊነት `sync_data` ግራግሶ፤ `msrcget` የራሱን ርዝመት መፈተሽ፣ የአፍታ ፋይል መፍጠር፣ ቅጥያ የቆርቆሮ ማውረዳት፣ `sync_data` ግራግሶ፣ የመጨረሻውን ጥሪ መቁረጥ እና ቀጥታ ማስተካከል ሁሉንም ይሳጣል)።
- የመፈተሻ ፍረፋ፦ ከእያንዳንዱ ጊዜ ማቆም በኋላ በደገፍ የዴስክ ፋይልን መነብብ፣ የፋይል ባይት ርዝመትን እና ጠቅላላ CRC32ን ማረጋገጥ፤ የመፈተሻው ሂደት በማውረዳት ጊዜ ውስጥ አይቆጠርም።
- የፍጥነት ትርጉም፦ ከ3 ጊዜ ጊዜ ማዕከል (ሜዲያን) ይውሰድ፣ በ `የፋይል ባይቶች / 1048576 / ማዕከል ደቂቃ` ይሰላል (`1 MB = 1048576 ባይቶች`)።
- የምንጭ ክፍፍል ዋና መርህ፦ ብዙ-ምንጭ ዘዴዎች (`msrcget`፣ `down`) በቅስቀቱ ውስጥ ያሉትን ሁሉም 5 መጋጠሚያ አድራሻዎች ያስገባሉ፤ ፊደል-ምንጭ ዘዴዎች (`cyper`፣ `reqwest`፣ `ureq`) ብዙ-ምንጭ ቆርቆሮን አይደግፉም፣ የመጠይቅ ዝርዝር ላይ ያለውን የመጀመሪያ ምንጭ አድራሻ ይጠይቃሉ።

## ልቀቶች

- ቁስ-ቁስ ማረጋገጫ እና ዉድቀት ማስተካከል፦ መጠቀሚያው `Verify` ማረጋገጫ መንገዱን በማስገባት እያንዳንዱን ቆርቆሮ ያረጋግጣል፤ ማረጋገጫ ሲጥል ወይም የውብ መጠይቅ ሲያቆርጥ፣ በቀሪው ዘገው ያልተሞከረ መጋጠሚያ ምንጭ ይቀያየራል፣ ሁሉንም ምንጮች በመጠቀም ጊዜ እስከጥል በሆነበት ደረጃ `Error::Exhausted` ይመልሳል።
- የአፍታ ፋይሎች እና የከፈታ መቀጠል፦ በፋይል ጠቅላላ ርዝመት ላይ በቀዳሚነት የአፍታ `.part` ጊዜያዊ ፋይል ይፈጠራል እና የፋይል ጥገት ይገባል (የጥገት ቅርብብ `Error::Locked` ይመልሳል)፤ በፋይል መጨረሻ ላይ የሁለት-ግማሽ ፒንግፕንግ CRC32 ቢትማፕ ጽርቶ የተጠናቀቁ ቆርቆሮዎች ይጸናሉ፣ ከቆረጭት በኋላ በቅጥያ ይቀጥላል፣ ሁሉ ሲጠናቀቅ የመጨረሻውን ጥሪ ይቁራጭቃል እና በአንድ ሂደት ወደ ተመሳጣ ፋይል ይቀይራል።
- የገጭ-ክፍፍል ስራ ንግግር፦ የራሱን ዞን በቅጥያ ፈጽሞ ያለው ምንጭ ከተቀረው ስራ ውስጥ ረጅሙን ክፍል በሁለት ቀርፎ የሁለተኛውን ክፍል ይቀበላል፣ የእያንዳንዱን መጋጠሚያ ምንጭ ጫጫጭ ይመዝግባል።
- የመጨረሻ ትኹል ፍጥነት መጠየቅ፦ ነፃ ገባነቶች ቁጥር ከተቀረው ስራ ተቆርቆሮቹ ሲበልጥ፣ ያንታይ ቆርቆሮ ያልተሞከረላቸው መጋጠሚያ ምንጮች ወደ ተጨማሪ ጠያቂዎች ይላካል፤ ቀዳዮት ማረጋገጫውን ሳለ ቀይ ጊዜ ያለ ተመሳሳይ ቆርቆሮ ላይ ያለውን ጠያቂ ይሰረዛል።
- የቆርቆሮ መዋቅር ምርጫ፦ በፋይል ጠቅላላ ርዝመት እና ቅጥያ ቁጥር ላይ የተመሰረተ የቆርቆሮ መጠን ይወሰናል፤ ጠቅላላ `.part` ፋይል ሲቀጥል ከመጽሐፈው ውስጥ የተመዘገበውን የቆርቆሮ መጠን በቅጥያ ይጠቀማል።
- የግንኙነት ቅዳጅ እና የፕሮቶኮል ድጋፍ፦ ውስጥ ባለው ፊደል-ምንጭ የግንኙነት ጥቅል HTTP/1.1 ረጅም ግንኙነት ይጠቀማል፣ የቆርቆሮ ምጭ ኮድ፣ HTTPS ([rustls](https://crates.io/crates/rustls))፣ `301`/`302`/`303`/`307`/`308` የመንገድ እና የምንጭ መቀያየር እና ጊዜ ክፍት ክትትል ይደግፋል።

## የመጠቀሚያ መገኘት

የትክክል ኮድ በ `examples/basic.rs` ውስጥ ላለ (`cargo run --example basic` ማከሰብ)፦

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

/// የ CRC32 ላይ ቆርቆሮ-ቆርቆሮ ማረጋገጫ
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

  // ምንጭ 0 የመጀመሪያው ቆርቆሮ መታተን ዳታ ይመልሳል፣ ምንጭ 1 በፍጥነት ደንበኛ ነው፣ ምንጭ 2 በጉዞ ዘገይተኛ ነው
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

  // በቅጥያ ብዙ-ምንጭ ቅጥያ ማውረዳት፦ የቆርቆሮ መጠን በቅጥያ ይወሰናል፣ የአፍታ .part ከተጻፈ በኋላ ይቁረጣል እና ይቀይራል
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() ፈጽሟል: {size_a} ባይቶች፣ በቅጥያ የተመኘ ቆርቆሮ {} ባይቶች",
    plan_chunk_size(size_a, 8)
  );

  // በቆርቆሮ CRC32 ማረጋገጫ ጋር ብዙ-ምንጭ ማውረዳት፦ ጥገ ቆርቆሮ ምንጭ በቅጥያ ይቀያየራል፣ ትኹር ምንጭ የተቀረውን በቅጥያ ንግግሮ ያገዛል
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
  println!("Get::verify() ማረጋገጫ ፈጽሞ: {size_b} ባይቶች");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## የመገኘት ማብራሪያ

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

በቅጥያ ቅጥያ ቁጥር `CONC` (`8`) እና በቅጥያ የጊዜ ክፍት `TIMEOUT` (`30s`) `url_li` ወደ `path` በማውረድ፣ የፋይል ጠቅላላ ባይቶች ይመልሳል።

### `Get`

የመገንባት መግቢያ `Get::new(url_li, path)`፣ የተሳሳብ አቀማመጥ ይደግፋል፦

- `.size(size: u64)`፦ የታወቀውን የፋይል ጠቅላላ ባይቶች በግልጽ ይገልጻል፣ የራሱን ርዝመት መፈተሽ ይሰረዛል።
- `.chunk_size(chunk_size: usize)`፦ የፊደል ቆርቆሮ ባይቶች ይገልጻል (ከ `0` `Error::ChunkSize` ይመልሳል)።
- `.conc(conc: usize)`፦ የቅጥያ የማውረዳት ገባነቶችን ይገልጻል።
- `.timeout(timeout: Duration)`፦ የፊደል ጠያቂ እና የማንብብ ጊዜ ክፍት ይገልጻል።
- `.verify<V2: Verify>(verify: V2)`፦ ማረጋገጫ ያስገባል፣ ለ `()`፣ `Fn(usize, &[u8]) -> bool`፣ `Rc<T>` እና `Arc<T>` ተፈጻሚ ሆኖ አለ።
- `.run().await`፦ ማውረዳትን ይጀምራል እና `Result<u64>` ይመልሳል።

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` ከ 0 ጀምሮ የቆርቆሮ ቁጥር ነው፣ `data` የዚህ ቆርቆሮ ባይት ቅርጅ ነው፣ `true` መመለስ ማረጋገጫው እንደተሳካ ያሳያል።

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

ከ `MIN_CHUNK` (`64 KB`) እስከ `MAX_CHUNK` (`4 MB`) ባለው ጫጫሚ 2 ገጭ ቆርቆሮ ባይቶች ይመልሳል።

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

የመጨረሻ ቋሚ `PART_EXT` (`".part"`) ይጨምራል፣ የቅጥያ ጊዜያዊ ፋይል መንገድ ይመልሳል።

### `Error` እና `Result`

ተመሳሳይ መመለሻ አይነት `Result<T> = std::result::Result<T, Error>`፣ የስህተት ተመሳሳይ ቅጦች ጨምረዋል፦ `EmptySrc`፣ `ChunkSize`፣ `Url`፣ `Scheme`፣ `Io`፣ `Tls`፣ `Timeout`፣ `Redirect`፣ `Status`፣ `Proto`፣ `Probe`፣ `Locked`፣ `Exhausted { idx }`።
