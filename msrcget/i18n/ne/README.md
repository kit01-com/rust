# msrcget

[compio](https://crates.io/crates/compio) को असिङक्रोनस कम्प्लीशन मोडेलमा आधारित बहु-स्रोत चक HTTP/HTTPS डाउनलोड लाइब्रेरी, एकल मिरर गति सीमा, चक डेटा क्षति र ढिला नोड टेल-ड्र्याग समस्या समाधान गर्न।

## प्रदर्शन मूल्याङ्कन

सार्वजनिक नेटवर्क HTTPS मिरर स्रोतमा [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) र [ureq](https://crates.io/crates/ureq) को डाउनलोड थ्रुपुट तुलना:

### दृश्य 1: बहु-सार्वजनिक मिरर एग्रीगेट डाउनलोड

Node.js v22.14.0 रिलिज प्याकेज (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB) डाउनलोड, मिरर क्रम: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (302 क्रस-स्रोत रिडायरेक्टसहित)।

| लाइब्रेरी / योजना | थ्रुपुट (MB/s) |
| --- | ---: |
| msrcget (5 स्रोत, 8 कन्करन्स) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 स्रोत) | 1.2 |
| cyper (एकल स्रोत) | 14.4 |
| reqwest (एकल स्रोत) | 14.4 |
| ureq (एकल स्रोत) | 14.3 |

### दृश्य 2: छिटो/ढिला स्रोत मिश्रित डाउनलोड

Node.js v22.14.0 हेडर प्याकेज (`node-v22.14.0-headers.tar.gz`, 8.4 MB) डाउनलोड, मिरर क्रम: Alibaba Cloud (एकल कनेक्सन गति सीमा), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js आधिकारिक स्रोत (क्रस-बोर्डर ढिला नोड)।

| लाइब्रेरी / योजना | थ्रुपुट (MB/s) |
| --- | ---: |
| msrcget (5 स्रोत, 8 कन्करन्स) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 स्रोत) | 0.2 |
| cyper (एकल स्रोत) | 6.8 |
| reqwest (एकल स्रोत) | 6.6 |
| ureq (एकल स्रोत) | 6.7 |

### परीक्षण विधि

मूल्याङ्कन कोड `examples/bench.rs` मा अवस्थित छ (`cargo run --profile bench --example bench` चलाउनुहोस्):

- बेसलाइन तयारी: टाइमिङ सुरु गर्नुअघि लक्ष्य फाइल एकपटक पूर्व-डाउनलोड गरी, बाइट लम्बाइ सत्यापन र पूरा प्याकेज तथा प्रत्येक चकको CRC32 बेसलाइन मान गणना गरिन्छ; यो चरणमा खर्च भएको समय समावेश गरिँदैन।
- टाइमिङ सीमा: प्रत्येक योजना निरन्तर स्वतन्तर रूपमा 3 राउन्ड चलाइन्छ, प्रत्येक राउन्ड सुरु गर्नुअघि आउटपुट फाइल र `.part` ट्रांसियरेंट फाइल हटाइन्छ; एक राउन्डमा नेटवर्क अनुरोध सुरु हुँदा टाइमर सुरु हुन्छ र फाइल डिस्कमा लेखिनەوە टाइमर रोकिन्छ (`cyper`, `reqwest`, `ureq` सबै स्पष्ट रूपमा `sync_data` फ्लश गर्छन्; `msrcget` मा रिमोट लम्बाइ पृथक्करण, रिड्युसेन्स फाइल सिर्जना, कन्करrent चक डाउनलोड, `sync_data` फ्लश, टेल मेटाडेटा ट्रंकेशन र एटामिक रिनेमिङ समावेश छ)।
- आउटपुट सत्यापन: प्रत्येक राउन्ड पछि टाइमर रोकिने परन्तु लेखिएका फाइल स्ट्रिमिङ रिकोर्ड गरी, फाइल बाइट लम्बाइ र पूरा प्याकेज CRC32 सत्यापन गरिन्छ; सत्यापन प्रक्रिया डाउनलोड समयमा समावेश हुँदैन।
- थ्रुपुट रूपान्तरण: 3 राउन्ड समयको मध्यमा लिई, `फाइल बाइट / 1048576 / मध्यमा सेकेन्ड` अनुसार थ्रुपुट गणना गरिन्छ (`1 MB = 1048576 बाइट`)।
- स्रोत आश्रय नियम: बहु-स्रोत योजना (`msrcget`, `down`) मा दृश्यको सारा 5 मिरर ठेगानाहरू पास गरिन्छ; एकल-स्रोत योजना (`cyper`, `reqwest`, `ureq`) बहु-स्रोत चक समर्थन गर्दैन, अनुरोध सूचीको पहिलो मिरर ठेगाना प्रयोग गर्छ।

## विशेषताहरू

- चक-बাই-चक सत्यापन र फिल्टर स्विच: कोलरले `Verify` सत्यापन इन्टरफेस प्रत्येक चक सत्यापन गर्न पास गर्छ; सत्यापन विफल वा नेटवर्क अनुरोध असफल भएमा, स्वतः बाँकी अप्रयास मिरर स्रोतमा रिट्राइ गर्छ, सबै स्रोतमा विफल भए `Error::Exhausted` फर्काउँछ।
- रिड्युसेन्स फाइल र ब्रेकपॉइन्ट रस्युमे: फाइलको कुल लम्बाइ अनुसार पूर्वाग्रहमा रिड्युसेन्स `.part` ट्रांसियरेंट फाइल सिर्जना गरी फाइल लक राखिन्छ (लक कन्फ्लिक्टमा `Error::Locked` फर्कन्छ); फाइलको अन्त्यमा डुअल-सट पिङ-पङ्ग CRC32 बिटम्याप लेखेर पूरा भएको चकहरू परिचालन गर्छ, बाधा पछि पुनः सुरु गर्दा स्वतः जारी राख्छ, सबै पूरा भएपछि टेल मेटाडेटा ट्रंक गरी एटामिक रूपमा लक्ष्य फाइलमा रिनेम गर्छ।
- बाइनरी टास्क स्टेल्थिङ: पहिले नै आफ्नो दायरा डाउनलोड पूरा गर्ने स्रोतले बाँकी असंपूर्ण सर्वाधिक विस्तार भएको दायरा दुई भागमा काट्दै दोस्रो आधा भाग स्वीकार गर्छ, यसले प्रत्येक मिरर स्रोतको लोड संतुलन गर्छ।
- टेल रन रस डरिभिङ: खाली कोर्युटिन संख्या बाँकी असंपूर्ण चक संख्या भन्दा बढी भएमा, त्यो चक अहिलेसम्म प्रयास नभएको मिरर स्रोतमा अतिरिक्त अनुरोध पठाइन्छ; पहिलो सत्यापन-पारित डाटा डिस्कमा लेकिनەوە तुरुन्तै त्यसै चकका बाँकी अनुरोधहरू रद्द गरिन्छ।
- स्वतः चक योजना: फाइलको कुल लम्बाइ र कन्करन्सी अनुसार चक साइज योजना गरिन्छ; मौजदा `.part` फाइल जारी गर्दा त्यसमा रेकर्ड गरिएको चक साइज स्वतः प्रयोग गरिन्छ।
- कनेक्सन पुनःप्रयोग र प्रोटोकल समर्थन: इनबिल्ड सम-स्रोत कनेक्सन पुलले HTTP/1.1 लोङ-कनेक्सन पुनःप्रयोग गर्छ, चक ट्रान्स्फर एन्कोडिङ, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` क्रस-पथ र क्रस-स्रोत रिडायरेक्ट र टाइमआउट नियन्त्रण समर्थन गर्छ।

## प्रयोग प्रदर्शन

उदाहरण कोड `examples/basic.rs` मा अवस्थित छ (`cargo run --example basic` चलाउनुहोस्):

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

/// CRC32-आधारित चक-बাই-चक सत्यापक
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

  // स्रोत 0 को पहिलो चकबाट क्षतिग्रस्त डाटा फर्कन्छ, स्रोत 1 उच्च गतिमा सामान्य, स्रोत 2 उच्च डेली ढिला नोड
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

  // डिफाल्ट बहु-स्रोत कन्करrent डाउनलोड: स्वतः चक साइज योजना, रिड्युसेन्स .part लेखेर ट्रंक र रिनेम
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // चक-बাই-चक CRC32 सत्यापनसहित बहु-स्रोत डाउनलोड: खराब चक स्वतः स्रोत बदल्छ, छिटो स्रोतले स्वतः टास्क स्टेल्थिङ र टेल रस मा ढिला स्रोतसँग प्रतियोगिता
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

## इन्टरफेस विवरण

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

डिफाल्ट कन्करन्सी `CONC` (`8`) र डिफाल्ट एकल टाइमआउट `TIMEOUT` (`30s`) प्रयोग गरी `url_li` लाई `path` मा डाउनलोड गर्छ, फाइलको कुल बाइट संख्या फर्काउँछ।

### `Get`

बिल्डर इनट्रपुट `Get::new(url_li, path)`, चेन-कन्फिग समर्थन गर्छ:

- `.size(size: u64)`: प्रख्याक रूपमा ज्ञात फाइलको कुल बाइट संख्या निर्धारण, रिमोट लम्बाइ पृथक्करण छुट्याउने।
- `.chunk_size(chunk_size: usize)`: एकल चक बाइट संख्या निर्धारण (`0` पास गर्दा `Error::ChunkSize` फर्कन्छ)।
- `.conc(conc: usize)`: कन्करrent डाउनलोड कोर्युटिन संख्या निर्धारण।
- `.timeout(timeout: Duration)`: एकल अनुरोध र पठन टाइमआउट निर्धारण।
- `.verify<V2: Verify>(verify: V2)`: सत्यापक इन्जेक्ट गर्छ, `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` र `Arc<T>` को लागि पहिल्यै लागू छ।
- `.run().await`: डाउनलोड सुरु गर्छ र `Result<u64>` फर्काउँछ।

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` 0-आधारित चक क्रमांक हो, `data` त्यो चकको बाइट स्लाइस हो, `true` फर्कनुमा सत्यापन सफल भएको संकेत हुन्छ।

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

`MIN_CHUNK` (`64 KB`) र `MAX_CHUNK` (`4 MB`) बीच 2 को घात चक बाइट संख्या फर्काउँछ।

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

सफिक्स कन्स्टन्ट `PART_EXT` (`".part"`) जोडी, रस्युमे ट्रांसियरेंट फाइल पथ फर्काउँछ।

### `Error` र `Result`

एकात्मक रिटर्न टाइप `Result<T> = std::result::Result<T, Error>`, त्रुटि इनम भेरिएन्टहरू: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`।
