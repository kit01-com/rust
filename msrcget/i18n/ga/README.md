# msrcget

Leabharlann íoslódála HTTP/HTTPS i mbloic ó iolrach ionaid bunaithe ar mhúnla críochnú as-sincrónach [compio](https://crates.io/crates/compio), le hionchas a chur in aghaidh sriovadh rátaí ionad amháin, tréigheán sonraí bloic, agus fadhbanna noibléanna mall a mhéadaíonn an ama iomlán.

## Tomhas perfoirme

Comparáid threimhse íoslódála [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) agus [ureq](https://crates.io/crates/ureq) ó ionaid HTTPS poiblí:

### Cúis 1: Íoslódáil chomhtháthach ó iolrach ionaid poiblí

Íoslódáil páca seachadta Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), in ord na nionad: Tsinghua, SUST, Huawei Cloud, Tencent Cloud, npmmirror (le hathréimh 302 idir-ionad).

| Leabharlann / Plean | Treimhs (MB/s) |
| --- | ---: |
| msrcget (5 ionad, 8 comhéagán) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (5 ionad) | 1.2 |
| cyper (ionad amháin) | 14.4 |
| reqwest (ionad amháin) | 14.4 |
| ureq (ionad amháin) | 14.3 |

### Cúis 2: Íoslódáil chomhangailt ionad tapa agus mall

Íoslódáil páca comhad ceanntóise Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8.4 MB), in ord na nionad: Alibaba Cloud (sriavh rátaí ceangail amháin), Tsinghua, Huawei Cloud, Tencent Cloud, ionad oifigiúil Node.js (noiblé trasnárasach mall).

| Leabharlann / Plean | Treimhs (MB/s) |
| --- | ---: |
| msrcget (5 ionad, 8 comhéagán) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (5 ionad) | 0.2 |
| cyper (ionad amháin) | 6.8 |
| reqwest (ionad amháin) | 6.6 |
| ureq (ionad amháin) | 6.7 |

### Modh tástála

Tá an cód tástála i `examples/bench.rs` (rith `cargo run --profile bench --example bench`):

- Easaithiú bonnlíne: sula n-éagóireoidh an t-am, íoslódáil an comhad sprioc arís; seiceáil fad na mbpeití agus ríomh luachanna bonnlíne CRC32 don chomhad iomlán agus do gach bloic; ní áirítear an chéim seo san am.
- Teorainneacha ama: rith gach plean go neamhspleách ar an eagar 3 chuaire; sula n-éagóireoidh gach cuairt, scrios an comhad aschuir agus comhad cast `.part`; tosaíonn an t-am nuair a chuirtear iarratas gréasán, agus stopaíonn sé nuair a bhíonn an comhad foirdiúil scríofa (déanann `cyper`, `reqwest` agus `ureq` uile `sync_data` go foirdiúil; áiríonn `msrcget` céim fad fhrithpháirteach, cruthú comhaid sparse, íoslódáil bhlóic chomhpháirteach, `sync_data` foirdiúil, gearradh meiteashonraí na bunlaistí agus athainmniú atomach ar fad).
- Seiceáil táirgí: tar éis an ama a stopú i ngach cuairt, léigh an comhad foirdiúil go sreabhach; seiceáil fad beit an chomhaid agus CRC32 an phacáistigh iomláine; ní áirítear an próiseas seiceála sa t-am íoslódála.
- Tiontú treimhse: tóg meánchuid ama na dtrí chuaire; ríomh treimhs de réir `fad comhad / 1048576 / meánchuid soicindí` (`1 MB = 1,048,576 beit`).
- Rialacha dháil ionad: do phlanna ilionad (`msrcget`, `down`), seolta gach ceann de 5 seoltar ionad sa chomhthéacs; do phlanna aonionad (`cyper`, `reqwest`, `ureq`), ní thacaítear le híoslódaíl bhlóic ilionad, úsáidtear an chéad seoltar as an liosta iarratas.

## Gnéithe

- Seiceáil bhloic ar bhlóic agus athruithe fiosaíochta: seolann gairmitheoir an `Verify` chun gach bloic a sheiceáil; nuair a theipeann ar an seiceáil nó nuair a theipeann ar iarratas gréasán, athraítear go huathoigh chuig ionad eile nach dtriailtear chun é a athrú, agus má theipeann ar gach ionad, filltear `Error::Exhausted`.
- Comhaid sparse agus leanúint ón bpunt: cruthaítear comhad cast sparse `.part` go roimh láimhe de réir fhad an chomhaid iomláin le clúdach comhaid (maith `Error::Locked` i gcónflict clúdaigh); scríobhtar léaráid dhúbhloc CRC32 ag bun an chomhaid chun na bloic ardaite a choinneáil, agus leanann sé go huathoigh tar éis aindigh; nuair a bhíonn gach rud críochnaithe, gearraítear meiteashonraí na bunlaistí agus déantar athainmniú atomach chuig an gcomhad sprioc.
- Corru dhá pháirt: nuair a bhíonn an t-ionad a chuir críochnú lena chuid féin ar an gcéad chéim, gearraítear an tréimhse iad éilithe iad is mó i dhá leath agus glactar leis an dara leath, ag cothabháil cothromaíochta na n-ábhar idir na hionaid.
- Comórtas ilbhealach ar an gceann deiridh: nuair a bhíonn líon na gcomhéagán saor níos mó ná líon na mbloic éilithe, seoltar iarrataí chasta chuig ionaid nach dtriail an bhloc sin; nuair a shcríobhtar an chéad doimhneacht a rannóigíonn, cuirtear i gcrích na hiarratais eile don bhloc céanna go díreach.
- Pleanáil bhlóic uathoigh: pleanáil méid na mbloic de réir fhad an chomhaid iomláin agus líon na gcomhéagán; nuair a leanann tú ó chomhad `.part` atá ann cheana, úsáidtear an méid bloic a taifeadtar sa chomhad sin go huathoigh.
- Athúsáid cheangail agus tacaíocht phrótacal: pool ceangal comh-ionaid inmheánach a athúsáideann ceangail fhada HTTP/1.1, tacaíocht do chódú sreabha bhlóca, HTTPS ([rustls](https://crates.io/crates/rustls)), athréimh `301`/`302`/`303`/`307`/`308` tras-pháirt agus tras-ionad, agus rialú ama socrú.

## Sampla úsáide

Tá an sampla cód i `examples/basic.rs` (rith `cargo run --example basic`):

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

/// Seiceálaí bloic ar bhlóic bunaithe ar CRC32
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

  // Ionad 0: aon bhloc 1 a thugann doimhneacht tréigthe; ionad 1: tapa normalta; ionad 2: noiblé mall le máthas ard
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

  // Íoslódáil ilionad comhpháirteach réamhshocraithe: pleanáil méid bhlóic go huathoigh, scríobh sparse .part ansin gearr agus athainmnigh
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Íoslódáil ilionad le seiceáil CRC32 bloic ar bhlóic: bloic dhroga athraithe go huathoigh, ionaid tapa a gcorraíonn go huathoigh agus comórtaíonn le hionaid mall ag an gceann
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

## Sonraíocht eangach

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Íoslódáil `url_li` go `path` le líon comhéagán réamhshocraithe `CONC` (`8`) agus ama socrú aon iarratais `TIMEOUT` (`30s`); filltear líon iomlán beit an chomhaid.

### `Get`

Inneall bunaithe `Get::new(url_li, path)`, le tacaíocht chumraíochta sraithe:

- `.size(size: u64)`: sainmhíniú sainiúil fhad beit an chomhaid iomláin, láimhseáil an bhreathnóireacht fad faide.
- `.chunk_size(chunk_size: usize)`: sainmhíniú méid beit na mbloic (maith `Error::ChunkSize` má chuirtear `0`).
- `.conc(conc: usize)`: sainmhíniú líon na gcomhéagán íoslódála comhpháirteach.
- `.timeout(timeout: Duration)`: sainmhíniú ama socrú iarratais amháin agus léamh.
- `.verify<V2: Verify>(verify: V2)`: ionramháil seiceálaí; déanta do `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` agus `Arc<T>`.
- `.run().await`: tosaigh an t-íoslódáil agus fill `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

Is earráid bhloic 0-bhunaithe é `idx`, is gearán beit an bhloic é `data`, agus `true` a fhilleadh a léiríonn go raibh an seiceáil ceart.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Filltear méid bloic a is cumhacht de 2 idir `MIN_CHUNK` (`64 KB`) agus `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Cuingítear an tairseach tairiseach `PART_EXT` (`".part"`), agus filltear slí an chomhaid cast leanúnach.

### `Error` agus `Result`

Cineál filloigh aonair `Result<T> = std::result::Result<T, Error>`, áiríonn an t-eanamh fosa inbhéartaithe: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
