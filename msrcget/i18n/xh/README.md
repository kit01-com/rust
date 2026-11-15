# msrcget

Ilayibrari yokuhlawula iimpawu ze-HTTP/HTTPS ezihlwanyelweyo zezorho, esekelwe kwimodeli yokugqiba ye-async ye-[compio](https://crates.io/crates/compio), esetyenziselwa ukusombulula iingxaki zokumisa umda we-mirror enye, ukonakala kweedatha zeempawu kunye neeproblemi zomozulu weempawu eziphantsi.

## Vekiyo yePerformance

Uqhathaniso lwe-[down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) kunye ne-[ureq](https://crates.io/crates/ureq) kwisiphumo sokuhlawula kwiimirror ze-HTTPS ze-intanethi:

### Isimo 1: Ukuhlawula okudibanisa iimirror ezivulekileyo

Hlawula ipakethe yeNode.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), iimirror ziya ngesiqingatha: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (iyahlanganiswe ne-302 redirect phakathi kweemirror).

| Ilayibrari / Indlela | Isivinene (MB/s) |
| --- | ---: |
| msrcget (iimirror ezi-5, i-concurrency ezi-8) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (iimirror ezi-5) | 1.2 |
| cyper (imirra enye) | 14.4 |
| reqwest (imirra enye) | 14.4 |
| ureq (imirra enye) | 14.3 |

### Isimo 2: Ukuhlawula okudibanisa izilo ezisheshayo kunye nezintsonkothileyo

Hlawula ipakethe ye-headers yeNode.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8.4 MB), iimirror ziya ngesiqingatha: Alibaba Cloud (umda lwe-connection enye), Tsinghua, Huawei Cloud, Tencent Cloud, isilo seNode.js esidla ngokusesuka ngaphandle (isiphetho esiphantsi).

| Ilayibrari / Indlela | Isivinene (MB/s) |
| --- | ---: |
| msrcget (iimirror ezi-5, i-concurrency ezi-8) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (iimirror ezi-5) | 0.2 |
| cyper (imirra enye) | 6.8 |
| reqwest (imirra enye) | 6.6 |
| ureq (imirra enye) | 6.7 |

### Indlela yeVekiyo

Ikhodi yevekiyo ikwikhodi `examples/bench.rs` (yenza `cargo run --profile bench --example bench`):

- Uhlambululuo lokuhlela: Ngaphambi kokubala ixesha, hlawula ifayile ekufuneka ifike ngaphambili, qinisekisa ubude bezibithi kwaye ubale i-CRC32 yonke ipakethe kunye neempawu zayo, esi sitha asibalwa kwixesha.
- Imida yobalo lweXesha: Indlela nganye isebenza ka-3 ngokwahlukeneyo, ngaphambi kwevekiyo nganye susa ifayile esinikiweyo kunye nefayile ye-`.part` ethelekayo; ixesha liqala xa i-network request ithunjwe liye xa ifayile igqitywe kwidiski (`cyper`, `reqwest`, `ureq` zombini zenza `sync_data` ngokucacileyo; `msrcget` ihlanganisa ukufunyanwa kwebanga kwikhehu, ukwenziwa kwefayile ye-sparse, ukuhlawula kweempawu ngexesha elifanayo, `sync_data`, ukucinywa kotywala lwe-metadata kunye nokulungiswa ngokuzenzekelayo kokuphinda-ubizo).
- Ukuqinisekisa izinto: Emva kokumisa ubalo lweXesha ngalunye, funda ifayile esinikiweyo ngokulandelanisa, qinisekisa ubude bezibithi kunye ne-CRC32 yonke ipakethe, esi sitha asibalwa kwixesha lokuhlawula.
- Uguqulo lweSivinene: Thatha i-median ye-3 zevekiyo zexesha, bala isivinene ngokusebenzisa `izibithi zefayile / 1048576 / iimephingi` (`1 MB = 1048576 izibithi`).
- Umgaqo wokwandisa izilo: Izindlela ezininzi zezilo (`msrcget`, `down`) zisebenzisa iimirror ezili-5 ezisezimeni; izindlela ezilinye zezilo (`cyper`, `reqwest`, `ureq`) azikwazi ukusebenzisa iimpawu ezininzi, sisebenzisa i-1 kwiimirror ekulayisithi.

## Iimpawu

- Ukuqinisekisa ngqo kunye nokutshintsha kweempawu: Umsebenzisi unika i-`Verify` interface ukuqinisekisa impawu nganye; xa i-verify ingaqiniseki okanye i-network request ivaliwe, kutshintshwa ngokuzenzekelayo kwezinye iimirror ezingasebenziswanga ukuze kufunye kwenze ngokuphindaphinda, xa zonke iimirror zivaliwe buya i-`Error::Exhausted`.
- Iimpawu ze-sparse kunye nokuhlawula ngokuphuma: Ngokusebenzisa ubude befile ibhalwe ngaphambili ifayile ye-`.part` ethelekayo kwaye ibekwe i-lock kwifayile (i-lock encinisiwe ibuya i-`Error::Locked`); utywala lwe-CRC32 double-slot bitmap lubhalwa kwiphelo lefayile ukubalekisa iimpawu ezigcwele, emva kokunyanzelwa kwakhona qala ngokuphuma, xa zonke zigqitywe zicinywe utywala lwe-metadata kwaye ziguqulwe ngokuzenzekelayo kwimpawu.
- Ukudibana kwezibini: Isilo esigqiba ummandla walo kuqala sithatha isiphambili esinqumayo kwindawo engakho ezinkulu ngokudibanisa ezibini kwaye sithathe icala lesibini, ukulinganisa ukusebenza kweemirror ngazinye.
- Ukwandisa ngamaphepha amaninzi: Xa inani le-correctives ezikhululekileyo lininzi kuneliyimpawu engakho egcweleyo, uthumela iimpawu ezongezelelekileyo kweemirror ezingasebenzisanga impawu ngaphambili; idatha yokuqala eqinisekisiweyo iya kwidiski ngokushesha isusa iimpawu ezinye kule mpawu.
- Ukucoca iimpawu ngokuzenzekelayo: Ngokusebenzisa ubude befile kunye nenani le-concurrency, qaqamba ubungakanani bempawu; xa uphuma kwakhona kwefayile ye-`.part`, usebenzisa ubungakanani bempawu obhalwe kuyo.
- Ukusebenzisa iimpawu ezidibeneyo kunye nokwazi ngokwemithetho: Iimpawu ze-same-source ezihlanganisiweyo ze-HTTP/1.1 long-connection, zazi ukusebenzisa i-chunked transfer encoding, i-HTTPS ([rustls](https://crates.io/crates/rustls)), i-`301`/`302`/`303`/`307`/`308` redirects ezivuka phakathi kweemirror kunye nokuqonda ixesha.

## Ubonakalo lwekhuselo

Ikhodi yesibonakalo ikwikhodi `examples/basic.rs` (yenza `cargo run --example basic`):

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

/// I-verify ngqo esekelwe kwi-CRC32
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

  // Isilo 0 impawu yokuqala ibuya idatha eonakaliweyo, isilo 1 kungxamisekile ngokufanelekileyo, isilo 2 inomozulu omde
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

  // I-download yeemirror ezininzi ngokuzenzekelayo: qaqamba ubungakanani bempawu, bhalwe i-.part ye-sparse ngemva koko icinywe ithi ifayile iguqulwe
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() igqityiwe: {size_a} izibithi, ubungakanani bempawu obunikelwe ngokuzenzekelayo {} izibithi",
    plan_chunk_size(size_a, 8)
  );

  // I-download yeemirror ezininzi ene-CRC32 verify ngqo: iimpawu eonakileyo ziguqulwa kwisilo ngokuzenzekelayo, isilo esishesha sithatha ngokuzenzekelayo kwaye sikhuphe ngokushesha kunye nesilo esiphantsi
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
  println!("Get::verify() iqinisekise kwaye igqityiwe: {size_b} izibithi");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Inkcazo yeAPI

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Sebenzisa i-default concurrency `CONC` (`8`) kunye ne-default single-request timeout `TIMEOUT` (`30s`) ukuhlawula i-`url_li` kwi-`path`, ibuya inani lizonke lezibithi zefayile.

### `Get`

I-entrance ye-builder `Get::new(url_li, path)`, iyazi ukungena ngokulandelanisa:

- `.size(size: u64)`: Cacisa ngokucacileyo inani lezibithi zefayile elaziwayo, qhubela phambili ukufunyanwa kwebanga kwikhehu.
- `.chunk_size(chunk_size: usize)`: Cacisa inani lezibithi zempawu enye (ukuba `0` ibuyiswe ibuya i-`Error::ChunkSize`).
- `.conc(conc: usize)`: Cacisa inani le-correctives zokuhlawula ngexesha elifanayo.
- `.timeout(timeout: Duration)`: Cacisa ixesha lokuphela kwe-request enye kunye nokufunda.
- `.verify<V2: Verify>(verify: V2)`: Ngenisa i-verify, sele yenziwe ye-`()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` kunye ne-`Arc<T>`.
- `.run().await`: Qalisa ukuhlawula kwaye ibuye i-`Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

I-`idx` inani leqhekeze liqala ku-0, i-`data` isitya sezibithi zale mpawu, ubuya `true` ubonisa ukuba i-verify iye.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Ibuya inani lezibithi zempawu ye-2 yi-power phakathi kwe-`MIN_CHUNK` (`64 KB`) kunye ne-`MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Hlanganisa i-suffix constant `PART_EXT` (`".part"`), ibuya indlela yefayile ethelekayo yokuphuma.

### I-`Error` kunye ne-`Result`

Uhlobo olunye lobuya `Result<T> = std::result::Result<T, Error>`, iimpawu ze-enum ezihlukeneyo ziquka: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
