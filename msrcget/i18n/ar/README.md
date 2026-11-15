# msrcget

مكتبة تنزيل HTTP/HTTPS متعددة المصادر مع تقسيم البيانات، مبنية على نموذج الإكمال غير المتزامن في [compio](https://crates.io/crates/compio)، تهدف إلى معالجة مشكلات تقييد السرعة لمصدر مرآة واحد، تلف بيانات التقسيم، وتأخر العقد البطيئة.

## معايير الأداء

مقارنة سرعة التنزيل لكل من [down](https://crates.io/crates/down) و[cyper](https://crates.io/crates/cyper) و[reqwest](https://crates.io/crates/reqwest) و[ureq](https://crates.io/crates/ureq) على مصادر مرايا HTTPS عامة:

### السيناريو 1: تنزيل مجمّع من عدة مرايا عامة

تنزيل حزمة توزيع Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`، 28.5 MB)، بترتيب المرايا: Tsinghua، USTC، Huawei Cloud، Tencent Cloud، npmmirror (يتضمن إعادة توجيه 302 عبر مصادر).

| المكتبة / الحل | الإنتاجية (ميغابايت/ثانية) |
| --------------------- | ----------: |
| msrcget (مصادر 5، موازاة 8) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (مصادر 5) | 1.2 |
| cyper (مصدر واحد) | 14.4 |
| reqwest (مصدر واحد) | 14.4 |
| ureq (مصدر واحد) | 14.3 |

### السيناريو 2: تنزيل من مصادر سريعة وبطيئة مختلطة

تنزيل حزمة ترويسات Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`، 8.4 MB)، بترتيب المرايا: Alibaba Cloud (تقييد معدل اتصال واحد)، Tsinghua، Huawei Cloud، Tencent Cloud، المصدر الرسمي لـ Node.js (عقدة بطيئة عابرة للحدود).

| المكتبة / الحل | الإنتاجية (ميغابايت/ثانية) |
| --------------------- | ----------: |
| msrcget (مصادر 5، موازاة 8) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (مصادر 5) | 0.2 |
| cyper (مصدر واحد) | 6.8 |
| reqwest (مصدر واحد) | 6.6 |
| ureq (مصدر واحد) | 6.7 |

### منهجية الاختبار

كود التقييم موجود في `examples/bench.rs` (تنفيذ `cargo run --profile bench --example bench`):

- التحضير المسبق: قبل بدء التوقيت، يتم تنزيل الملف المستهدف مسبقًا مرة واحدة، والتحقق من طول البايتات وحساب قيم CRC32 المرجعية للملف الكامل ولكل قسم، ولا يُحتسب هذا الوقت ضمن المدة.
- حدود القياس الزمني: يُشغَّل كل حلّ بشكل مستقل ومتسلسل لـ3 جولات، وتُحذف ملفات الإخراج والملفات المؤقتة `.part` قبل بدء كل جولة؛ يبدأ التوقيت من لحظة إرسال طلب الشبكة حتى اكتمال كتابة الملف على القرص (تنفّذ كل من `cyper` و`reqwest` و`ureq` صراحةً استدعاء `sync_data` لفرز القرص؛ تشمل `msrcget` كامل العملية: اكتشاف الطول البعيد، إنشاء الملف المتخلخل، التنزيل المتوازي بالتجزئة، قصّ بيانات الوصف النهائية، فرز القرص عبر `sync_data`، وإعادة التسمية الذرّية).
- التحقق من الناتج: بعد إيقاف التوقيت في كل جولة، يُقرأ الملف المكتوب بشكل تدفقي، ويُتحقق من طول البايتات وCRC32 للملف الكامل، ولا يُحتسب وقت التحقق ضمن زمن التنزيل.
- تحويل المعدل: يُؤخذ وسيط أزمنة الجولات الثلاث، ويُحسب المعدل عبر `عدد بايتات الملف / 1048576 / الوسيط بالثواني` (`1 MB = 1048576 بايت`).
- قاعدة توزيع المصادر: مخططات المصادر المتعددة (`msrcget`، `down`) تمرر جميع عناوين المرايا الخمسة في السيناريو؛ مخططات المصدر الواحد (`cyper`، `reqwest`، `ureq`) لا تدعم تقسيم المصادر المتعددة، فتطلب العنوان الأول فقط من قائمة المرايا.

## الميزات

- التحقق الشامل لكل قسم والتبديل التلقائي عند الأعطال: يمرر المُستدعي واجهة `Verify` للتحقق من كل قسم؛ عند فشل التحقق أو فشل طلب الشبكة، يتم التبديل تلقائيًا إلى مصادر مرايا غير المحاولة لإعادة المحاولة، وعند فشل جميع المصادر يُعاد `Error::Exhausted`.
- الملفات المتخلخلة والمتابعة من نقطة التوقف: يُنشَأ مسبقًا ملف `.part` مؤقت متخلخل حسب الطول الكلي للملف مع إضافة قفل على الملف (في حالة تعارض القفل يُعاد `Error::Locked`)؛ يُضاف خريطة بتية في نهاية الملف لتمييز التجزئات المكتملة، وبعد التوقف والاستئناف تُستأنف العملية تلقائيًا، وبعد اكتمالها يُقَصّ بيانات الوصف النهائية ويُعاد التسمية الذرّية إلى الملف المستهدف.
- سرقة المهام بالتقسيم الثنائي: المصدر الذي يُنهي تنزيل نطاقه أولًا يُقسّم أكبر النطاقات غير المكتملة المتبقية إلى نصفين ويتولى النصف الثاني، لتوازن الأحمال بين مصادر المرايا.
- سباق متعدد المسارات في المرحلة النهائية: عندما يتجاوز عدد الخيوط غير المشغولة عدد الأقسام غير المكتملة المتبقية، تُرسل طلبات زائدة إلى مصادر مرايا لم تُجرَّب بعد لتلك الأقسام؛ أول بيانات تجتاز التحقق وتُكتب على القرص يُلغى فورًا بقية الطلبات لنفس القسم.
- التخطيط التلقائي للأقسام: يُخطط حجم الأقسام بناءً على الطول الإجمالي للملف وعدد العمليات المتوازية؛ عند استئناف ملف `.part` موجود، يُستخدَم تلقائيًا حجم الأقسام المسجَّل فيه.
- إعادة استخدام الاتصالات ودعم البروتوكولات: حوض اتصالات مدمج للمرايا نفسها لإعادة استخدام اتصالات HTTP/1.1 طويلة العمر، مع دعم ترميز التحويل المقسّم، HTTPS ([rustls](https://crates.io/crates/rustls))، إعادة التوجيه عبر المسارات والمصادر `301`/`302`/`303`/`307`/`308`، والتحكم في فترات الانتظار.

## عرض الاستخدام

كود المثال موجود في `examples/basic.rs` (تنفيذ `cargo run --example basic`):

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

/// مُتحقق CRC32 لكل قسم
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

  // المصدر 0 يُرجع بيانات تالفة للقسم 1، المصدر 1 سريع وطبيعي، المصدر 2 بطيء عالي التأخير
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

  // تنزيل متوازٍ متعدد المصادر افتراضي: تخطيط تلقائي لحجم الأقسام، كتابة .part متفرّق ثم قطع وإعادة تسمية
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // تنزيل متعدد مصادر مع تحقق CRC32 لكل قسم: تبديل تلقائي للمصدر عند التالف، سرقة تلقائية من المصادر السريعة وسباق نهائي مع المصادر البطيئة
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

## مرجع الواجهات

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64> where I: IntoIterator<Item = S>, S: AsRef<str>`

تنزيل `url_li` إلى `path` باستخدام عدد التوازي الافتراضي `CONC` (`8`) ووقت التوقف الافتراضي لكل عملية `TIMEOUT` (`30 ثانية`)، ويعيد العدد الكلي لبايتات الملف.

### `Get`

مدخل الباني `Get::new(url_li, path)`، مع دعم التكوين التسلسلي:

- `.size(size: u64)` تحديد العدد الكلي لبايتات الملف المسبق معرفته صراحةً، وتخطّي اكتشاف الطول البعيد (عند تمرير `0` يُعتبَر اكتمال ملف فارغ).
- `.chunk_size(chunk_size: usize)`: تعيين عدد بايتات القسم الواحد (تمرير `0` يُرجع `Error::ChunkSize`).
- `.conc(conc: usize)` تحديد عدد خيوط التنزيل المتوازي (عند تمرير `0` يُعامَل على أنه `1`).
- `.timeout(timeout: Duration)`: تعيين فترة انتظار الطلب والقراءة.
- `.verify<V2: Verify>(verify: V2)`: حقن مُتحقق، مُنفَّذ لـ `()`، `Fn(usize, &[u8]) -> bool`، `Rc<T>` و`Arc<T>`.
- `.run().await`: بدء التنزيل وإرجاع `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` هو فهرس القسم بدءًا من 0، و`data` هي شريحة بايتات هذا القسم، وإرجاع `true` يعني نجاح التحقق.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

يُرجع حجم قسم من قوى 2 بين `MIN_CHUNK` (`64 KB`) و`MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

يُلحق بامتداد `PART_EXT` (`".part"`) ويُرجع مسار الملف المؤقت للاستئناف.

### `Error` و`Result`

نوع الإرجاع الموحد `Result<T> = std::result::Result<T, Error>`، تُغيّرات التعداد تشمل: `EmptySrc`، `ChunkSize`، `Url`، `Scheme`، `Io`، `Tls`، `Timeout`، `Redirect`، `Status`، `Proto`، `Probe`، `Locked`، `Exhausted { idx }`.
