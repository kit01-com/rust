# msrcget

[compio](https://crates.io/crates/compio) ৰ অসংক্ৰমিক কম্প্লিছন ম'ডেলৰ ওপৰত নিৰ্মিত বহু-উৎস চাংকড HTTP/HTTPS ডাউনল'ড লাইব্ৰেৰী, যি একক মীৰাৰ বেগ সীমা, চাংকড ডাটা ক্ষতি আৰু ধীৰ ন'ডৰ পশ্চাদ-স্যাং সমস্যা সমাধান কৰে।

## কাৰ্যক্ষমতা পৰীক্ষণ

[down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) আৰু [ureq](https://crates.io/crates/ureq) কে পাব্লিক নেটৱৰ্ক HTTPS মীৰাৰ উৎসত ডাউনল'ড থ্রুপুট তুলনা কৰা হ'ল:

### তলক ১: বহু-পাব্লিক মীৰাৰ সমষ্টিগত ডাউনল'ড

Node.js v22.14.0 বিতৰণ পেকেজ (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB) ডাউনল'ড কৰা হ'ল, মীৰাৰ ক্ৰম: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (302 ক্ৰ'ছ-উৎস ৰেডাইৰেক্টেশনসহ)।

| লাইব্ৰেৰী / পদ্ধতি | থ্রুপুট (MB/s) |
| --- | ---: |
| msrcget (5 উৎস, 8 সমান্তৰাল) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 উৎস) | 1.2 |
| cyper (একক উৎস) | 14.4 |
| reqwest (একক উৎস) | 14.4 |
| ureq (একক উৎস) | 14.3 |

### তলক ২: দ্ৰুত-ধীৰ উৎসৰ মিশ্ৰিত ডাউনল'ড

Node.js v22.14.0 হেডাৰ ফাইল পেকেজ (`node-v22.14.0-headers.tar.gz`, 8.4 MB) ডাউনল'ড কৰা হ'ল, মীৰাৰ ক্ৰম: Alibaba Cloud (একক সংযোগ বেগ সীমা), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js অফিচিয়েল উৎস (বৰ্ডাৰ-ক্ৰ'ছ ধীৰ ন'ড)।

| লাইব্ৰেৰী / পদ্ধতি | থ্রুপুট (MB/s) |
| --- | ---: |
| msrcget (5 উৎস, 8 সমান্তৰাল) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 উৎস) | 0.2 |
| cyper (একক উৎস) | 6.8 |
| reqwest (একক উৎস) | 6.6 |
| ureq (একক উৎস) | 6.7 |

### পৰীক্ষা পদ্ধতি

পৰীক্ষা কোড `examples/bench.rs`-ত আছে (`cargo run --profile bench --example bench` চলাওক):

- বেছলাৰ প্ৰস্তুতি: টাইমিং আৰম্ভ কৰোঁতে লক্ষ্য ফাইল আগতেই এবাৰ ডাউনল'ড কৰি বাইট দৈৰ্ঘ্য পৰীক্ষা কৰা হয় আৰু গোটেই পেকেজ আৰু প্ৰতিটো চাংকৰ CRC32 বেছলাৰ মান গণনা কৰা হয়; এই পৰ্যায়টো সময়-পৰীক্ষাত নাগণ্য কৰা হয়।
- টাইমিং সীমা: প্ৰতিটো পদ্ধতি অবিৰত ৩ পালি স্বতন্ত্ৰভাৱে চলোৱা হয়; প্ৰতি পালিৰ আগতে আউটপুট ফাইল আৰু `.part` সংকালীন ফাইল আঁতৰোৱা হয়; এপালি নেটৱৰ্ক আৰ্জু সমৰ্পণৰ পৰা আৰম্ভ হৈ ফাইল ডিছকত সঁজিওৱাৰ পৰিৱৰ্তনলৈ চলি থাকে (`cyper`, `reqwest`, `ureq` ৰ সকলোৱে স্পষ্টভাৱে `sync_data` ফ্ল'চ কৰে; `msrcget`-ত দূৰবৰ্তী দৈৰ্ঘ্য অনুসন্ধান, সংকুচিত ফাইল সৃষ্টি, সমান্তৰাল চাংকড ডাউনল'ড, `sync_data` ফ্ল'চ, শেষ-ধৰা মেটাডাটা পিচাই আৰু পৰমাণু পুনৰ্নামকৰণ গোটেই প্ৰক্ৰিয়া অন্তৰ্ভুক্ত)।
- ফলাফল পৰীক্ষা: প্ৰতি পালিৰ টাইমিং বন্ধ কৰাৰ পিছত ডিছকত থকা ফাইল স্টিমিং পাঠ কৰি ফাইলৰ বাইট দৈৰ্ঘ্য আৰু গোটেই পেকেজৰ CRC32 পৰীক্ষা কৰা হয়; পৰীক্ষা প্ৰক্ৰিয়া ডাউনল'ড সময়ত নাগণ্য কৰা হয়।
- থ্রুপুট গণনা: ৩ পালিৰ সময়ৰ মধ্যমান লৈ `ফাইল বাইট / 1048576 / মধ্যমান ছেকেণ্ড` ফৰ্মুলাৰে থ্রুপুট গণনা কৰা হয় (`1 MB = 1048576 বাইট`)।
- উৎস বিতৰণ নিয়ম: বহু-উৎস পদ্ধতি (`msrcget`, `down`) ত তলকৰ ৫ টা মীৰাৰ ঠিকানাই প্ৰদান কৰা হয়; একক-উৎস পদ্ধতি (`cyper`, `reqwest`, `ureq`) বহু-উৎস চাংকক সমৰ্থন নকৰে, আৰ্জু তালিকাৰ প্ৰথম মীৰাৰ ঠিকানা ব্যৱহাৰ কৰা হয়।

## বৈশিষ্ট্য

- চাংক-অ-চাংক পৰীক্ষা আৰু ত্ৰুটি সুইচ-অ'ভাৰ: কলকৰ্তাই `Verify` পৰীক্ষা ইন্টাফেছ জৰিয়তে প্ৰতিটো চাংক পৰীক্ষা কৰে; পৰীক্ষাত অসফল হ'লে বা নেটৱৰ্ক আৰ্জু বিফল হ'লে, স্বয়ংক্ৰিয়ভাৱে বাকী অচেষ্টিত মীৰাৰ উৎসলৈ সুইচ কৰি পুনৰ চেষ্টা কৰা হয়; সকলো উৎসৰ পিছতো বিফল হ'লে `Error::Exhausted` ল'ত হয়।
- সংকুচিত ফাইল আৰু ব্ৰেকপ'ইণ্ট ৰিয়েজিউম: ফাইলৰ মুঠ দৈৰ্ঘ্য অনুসৰি আগতেই সংকুচিত `.part` সংকালীন ফাইল সৃষ্টি কৰি ফাইল লাচ সংযোগ কৰা হয় (লাচ প্ৰতিযোগিতাত `Error::Locked` ল'ত হয়); ফাইলৰ শেষত ডাবল-ছ্ল'ট পিং-পং CRC32 বিটম'প লিখি সম্পূৰ্ণ হ'ল চাংক স্থিৰভাৱে সংৰক্ষিত কৰা হয়, সংযমে আৰম্ভ কৰাৰ পিছত স্বয়ংক্ৰিয়ভাৱে ৰিয়েজিউম কৰা হয়, সকলো সম্পূৰ্ণ হ'লে শেষ-ধৰা মেটাডাটা পিচাই কৰি লক্ষ্য ফাইলৰ নাম পৰমাণুভাৱে সলনি কৰা হয়।
- দ্বিচাংক টাইম-ছটি'লিং: নিজৰ আয়তনৰ ডাউনল'ড প্ৰথমে সম্পূৰ্ণ কৰা উৎসে বাকী অসম্পূৰ্ণ সৰ্বাধিক আয়তনৰ অৰ্ধেক কাটি পশ্চাৎ অৰ্ধাংশ গ্ৰহণ কৰে, প্ৰতিটো মীৰাৰ উৎসৰ চাপ সমান কৰে।
- শেষ-ধৰা বহু-পথ ৰেছিং: মুক্ত ক'ৰেটিণ সংখ্যা বাকী অসম্পূৰ্ণ চাংকতকৈ বেছি হ'লে, এতিয়ালৈকে সেই চাংক চেষ্টা নকৰা মীৰাৰ উৎসলৈ অতিৰিক্ত আৰ্জু প্ৰেৰণ কৰা হয়; প্ৰথম পৰীক্ষাত উত্তীৰ্ণ ডাটা ডিছকত সঁজিওৱাৰ পিছত সেই চাংকৰ বাকী আৰ্জু ক্ৰমে বাতিল কৰা হয়।
- স্বয়ংক্ৰিয় চাংক পৰিকল্পনা: ফাইলৰ মুঠ দৈৰ্ঘ্য আৰু সমান্তৰাল সংখ্যাৰ ওপৰত ভিত্তি কৰি চাংকৰ আকাৰ পৰিকল্পিত কৰা হয়; পূৰ্বৰ `.part` ফাইলৰ পৰা ৰিয়েজিউম কৰোঁতে তাত ক'ৰা চাংকৰ আকাৰ স্বয়ংক্ৰিয়ভাৱে ব্যৱহাৰ কৰা হয়।
- সংযোগ পুনৰ্ব্যৱহাৰ আৰু প্ৰট'ক'ল সমৰ্থন: একে উৎসৰ সংযোগ প'লত HTTP/1.1 দীৰ্ঘ সংযোগ পুনৰ্ব্যৱহাৰ কৰা হয়, চাংকড ট্ৰান্সফাৰ এনক'ডিং, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` ক্ৰ'ছ-পথ আৰু ক্ৰ'ছ-উৎস ৰেডাইৰেক্টেশন আৰু টাইম-আউট নিয়ন্ত্ৰণ সমৰ্থিত।

## ব্যৱহাৰ প্ৰদৰ্শন

উদাহৰণ কোড `examples/basic.rs`-ত আছে (`cargo run --example basic` চলাওক):

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

/// CRC32-ৰ আধাৰত চাংক-অ-চাংক পৰীক্ষক
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

  // উৎস 0 ৰ প্ৰথম চাংকত ক্ষতিগ্রস্থ ডাটা ঘূৰাই দিয়ে, উৎস 1 দ্ৰুত-নিৰ্ভৰযোগ্য, উৎস 2 উচ্চ-বিলম্ব ধীৰ ন'ড
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

  // প্ৰাথমিক বহু-উৎস সমান্তৰাল ডাউনল'ড: স্বয়ংক্ৰিয় চাংক আকাৰ পৰিকল্পনা, সংকুচিত .part লিখি শেষত পিচাই নাম সলনি
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // চাংক-অ-চাংক CRC32 পৰীক্ষাসহ বহু-উৎস ডাউনল'ড: ক্ষতিগ্রস্থ চাংকত স্বয়ংক্ৰিয় উৎস সুইচিং, দ্ৰুত উৎসৰ স্বয়ংক্ৰিয় টাইম-ছটি'লিং আৰু শেষ-ধৰা ধীৰ উৎসৰ সৈতে ৰেছিং
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

## ইন্টাফেছ বৰ্ণনা

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

প্ৰাথমিক সমান্তৰাল সংখ্যা `CONC` (`8`) আৰু প্ৰাথমিক একক টাইম-আউট `TIMEOUT` (`30s`) ব্যৱহাৰ কৰি `url_li` ৰ পৰা `path` লৈ ডাউনল'ড কৰে, ফাইলৰ মুঠ বাইট সংখ্যা ঘূৰাই দিয়ে।

### `Get`

বিল্ডাৰ প্ৰবেশ `Get::new(url_li, path)`, চেইনড কনফিগাৰেশ্যন সমৰ্থিত:

- `.size(size: u64)`: পৰিচিত ফাইলৰ মুঠ বাইট সংখ্যা স্পষ্টভাৱে নিৰ্ধাৰিত কৰে, দূৰবৰ্তী দৈৰ্ঘ্য অনুসন্ধান বন্ধ কৰে।
- `.chunk_size(chunk_size: usize)`: একক চাংকৰ বাইট সংখ্যা নিৰ্ধাৰিত কৰে (`0` দিলে `Error::ChunkSize` ল'ত হয়)।
- `.conc(conc: usize)`: সমান্তৰাল ডাউনল'ড ক'ৰেটিণ সংখ্যা নিৰ্ধাৰিত কৰে।
- `.timeout(timeout: Duration)`: একক আৰ্জু আৰু পঢ়াৰ টাইম-আউট নিৰ্ধাৰিত কৰে।
- `.verify<V2: Verify>(verify: V2)`: পৰীক্ষক ইনজেক্ট কৰে, `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` আৰু `Arc<T>` ৰ বাবে ইলিমেন্ট কৰা আছে।
- `.run().await`: ডাউনল'ড আৰম্ভ কৰে আৰু `Result<u64>` ঘূৰাই দিয়ে।

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` হ'ল ০-আধাৰিত চাংক ক্ৰমসংখ্যা, `data` হ'ল সেই চাংকৰ বাইট স্ল'চ, `true` ঘূৰাই দিয়া মানে পৰীক্ষাত উত্তীৰ্ণ।

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

`MIN_CHUNK` (`64 KB`) আৰু `MAX_CHUNK` (`4 MB`) ৰ ভিতৰত 2 ৰ ঘাত চাংক বাইট সংখ্যা ঘূৰাই দিয়ে।

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

উপশৰ্গ ধ্ৰুৱক `PART_EXT` (`".part"`) সংযোজিত কৰি ৰিয়েজিউম সংকালীন ফাইলৰ পথ ঘূৰাই দিয়ে।

### `Error` আৰু `Result`

একক ফিৰ্টা তৰহ `Result<T> = std::result::Result<T, Error>`, ত্ৰুটি এনাম ভেৰিয়েণ্ট: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`।
