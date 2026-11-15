# msrcget

Kitaibabu cha kupakua HTTP/HTTPS kwa vipande vya vyanzo vingi, kililotegemea mtaala wa usawa wa kufulumalua wa [compio](https://crates.io/crates/compio), kinachotumika kutatua matatizo ya kikwazo cha kasi cha ramani moja, unyoaji wa data ya vipande, na nusuriko la nodi nene.

## Uthibitishaji wa uwezo

Ulinganisho wa [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) na [ureq](https://crates.io/crates/ureq) kwenye mto wa kupakua chini ya vyanzo vya ramani vya HTTPS vya mtandao wa umma:

### Hali 1: Kupakua kwa vyanzo vingi vya ramani za umma

Kupakua kifurushi cha Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28.5 MB), ramani kufuatilia: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (ikiwa na upanga 302 wa kurudia vyanzo).

| Kitaibabu / Njia | Mto (MB/s) |
| --- | ---: |
| msrcget (chanzo 5, pamoja 8) | 13.9 |
| msrcget + CRC32 | 13.4 |
| down (chanzo 5) | 1.2 |
| cyper (chanzo kimoja) | 14.4 |
| reqwest (chanzo kimoja) | 14.4 |
| ureq (chanzo kimoja) | 14.3 |

### Hali 2: Kupakua kwa mchanganyiko wa vyanzo vya haraka na polepole

Kupakua kifurushi cha faili za kichwa cha Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8.4 MB), ramani kufuatilia: Alibaba Cloud (kikwazo cha uunganisho mmoja), Tsinghua, Huawei Cloud, Tencent Cloud, chanzu rasmi cha Node.js (nodi nene za nchi nyingine).

| Kitaibabu / Njia | Mto (MB/s) |
| --- | ---: |
| msrcget (chanzo 5, pamoja 8) | 13.5 |
| msrcget + CRC32 | 13.0 |
| down (chanzo 5) | 0.2 |
| cyper (chanzo kimoja) | 6.8 |
| reqwest (chanzo kimoja) | 6.6 |
| ureq (chanzo kimoja) | 6.7 |

### Mbinu za upimaji

Msimbo wa upimaji upo katika `examples/bench.rs` (uteleze `cargo run --profile bench --example bench`):

- Uwandani wa mwelekeo: kabla ya kuanza kukadiria muda, pakua kwanza faili lengo, kagua urefu wa baibiti na kuhesabu thamani za msingi za CRC32 za kifurushi zote na vipande vya moja kwa moja, awamu hii haihesabiwa katika muda.
- Mipaka ya muda: kila njia inaendesha mara 3 kwa mstari, kabla ya kila mzunguko kutumia faili za matokeo na faili za muda `part`; mzunguko mmoja unakadiria muda tangu anza ya ombi la mtandao hadi kumalizika kwa faili kwenye diski (`cyper`, `reqwest`, `ureq` wote wafanya `sync_data` kufuta kwenye diski kwa wazi; `msrcget` inaunda uchunguzi wa urefu wa mbali, kutengeneza faili ya nyepesi, kupakua kwa vipande kwa pamoja, `sync_data` kufuta kwenye diski, kukatiza data ya mwisho na kubadilisha jina kwa atomiki).
- Uthibitishaji wa matokeo: baada ya kusimamisha muda kila mzunguko, soma faili kwa mtiririko, kagua urefu wa baibiti na CRC32 ya kifurushi zote, mchakato wa uthibitisho haihesabiwa katika muda wa kupakua.
- Ubadilishaji wa mto: chukua wastani wa kati wa muda wa mizunguko 3, hesabu mto kwa `idadi ya baibiti za faili / 1048576 / sekunde za wastani wa kati` (`1 MB = baibiti 1048576`).
- Kanuni za usambazaji wa vyanzo: njia za vyanzo vingi (`msrcget`, `down`) zinasafirisha maeneo yote 5 ya ramani katika hali; njia za chanzo kimoja (`cyper`, `reqwest`, `ureq`) hazitaki kupakua kwa vipande vya vyanzo vingi, omba eneo la ramani la 1 kwenye orodha.

## Sifa

- Uthibitishaji wa kila kipande na kubadilisha hatari: mtanzu anaingiza mnyingamo wa `Verify` kuthibitisha kila kipande; ukiwa na kushindwa kwa uthibitishaji au ombi la mtandao kushindwa, badilika kwa utambulisho hadi chanzo kingine cha ramani ambacho hakijarajiwa, ikiwa kuzungusha vyanzo vyote bado kimeshindwa, rudi `Error::Exhausted`.
- Faili ya nyepesi na kupakua upya: kulingana na urefu wa jumla wa faili, weka faili ya muda ya nyepesi `.part` iliyojengwa na kufunga faili (mgongano wa kufunga rudi `Error::Locked`); kwenye mwisho wa faili, andika ramani ya ping-pong ya CRC32 ya vidoka viwili kuhifadhi vipande vilivyokamilika, baada ya kukatishwa na kuanzishwa upya, pakua upya kwa utendaji, ukimaliza yote, katiza data ya mwisho na ubadilishe jina kwa atomiki hadi faili lengo.
- Uchafu wa kazi kwa misingi miwili: chanzo ambacho kwanza kumaliza kipindi chake kinavuma nusu ya kipindi kirefu zaidi kilichobaki chakamilika na kuchukua nusu ya mwisho, kubainisha mizani ya kazi ya kila chanzo cha ramani.
- Mshindano wa nje ya mwisho: ikiwa idadi ya krosi za kutumia zaidi ni zaidi ya vipande vilivyobaki vyasiyokamilika, tuma ombi la ziada kwa vyanzo vya ramani ambavyo havijajaraji kipande hicho; baada ya data ya kwanza kupita uthibitishaji kuandikwa kwenye diski, luka kwa haraka maombi mengine ya kipande hicho.
- Mepa ya kugawanya kwa utendaji: kwa urefu wa jumla wa faili na idadi ya uchangani, panga ukubwa wa kipande; wakati wa kupakua upya faili ya `.part` iliyoopo, tumia kwa utendaji ukubwa wa kipande uliohifadhiwa ndani yake.
- Reutiaji ya uunganisho na msaada wa protokoli: kiungo cha asili cha vyanzo vya asili kinareutia vyanzo vya HTTP/1.1 vya muda mrefu, kinatumia utajiri wa mteja, HTTPS ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` mabadiliko ya mazingira na vyanzo pamoja na udhibiti wa muda.

## Onyesho la matumizi

Msimbo wa mfano upo katika `examples/basic.rs` (uteleze `cargo run --example basic`):

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

/// Kichaguzi cha uthibitishaji wa kila kipande kwa CRC32
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

  // Chanzo 0 kipande 1 rinarudi data iliovunjika, chanzo 1 haraka na kawaida, chanzo 2 polepole zenye zaacha nene
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

  // Kupakua kwa vyanzo vingi kwa pamoja kwa sasa: panga ukubwa wa kipande kwa utendaji, andika faili ya nyepesi .part kisha katiza na ubadilishe jina
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Kupakua kwa vyanzo vingi wenye uthibitishaji wa CRC32 kwa kila kipande: kipande kilema kinabadilika chanzo kwa utendaji, chanzo haraka kinachafua kwa utendaji na kushindana na chanzo polepole mwisho
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

## Maelezo ya mnyingamo

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Tumia idadi ya uchangani ya msingi `CONC` (`8`) na muda wa kukata wa msingi `TIMEOUT` (`30s`) kupakua `url_li` hadi `path`, rudi idadi ya jumla ya baibiti za faili.

### `Get`

Mnyingamo wa kuzalisha `Get::new(url_li, path)`, inayotumia muundo wa mnyingina:

- `.size(size: u64)`: Weka kwa wazi idadi ya jumla ya baibiti za faili iliyojulikana, ruka uchunguzi wa urefu wa mbali.
- `.chunk_size(chunk_size: usize)`: Weka idadi ya baibiti za kipande moja (kukamata `0` rudi `Error::ChunkSize`).
- `.conc(conc: usize)`: Weka idadi ya krosi za kupakua kwa pamoja.
- `.timeout(timeout: Duration)`: Weka muda wa kukata wa ombi moja na kusoma.
- `.verify<V2: Verify>(verify: V2)`: Ingiza kichaguzi, tayari kwa `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` na `Arc<T>`.
- `.run().await`: Anzisha kupakua na rudi `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` ni namba ya kuanzia kwa kipande (0), `data` ni kagoshi ya baibiti za kipande hicho, kurudi `true` huthibitisha uthibitishaji uliopita.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Rudi idadi ya baibiti za kipande za nguvu ya 2 kati ya `MIN_CHUNK` (`64 KB`) na `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Ugaisha sifa ya `PART_EXT` (`".part"`), rudi njia ya faili ya muda ya kupakua upya.

### `Error` na `Result`

Aina ya kurudi moja kwa moja `Result<T> = std::result::Result<T, Error>`, vitufe vya kosa vya enum vinajumuisha: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
