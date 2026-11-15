# msrcget

Toetub [compio](https://crates.io/crates/compio) asünkroonse lõpetuse mudelile; mitme allika järgudega HTTP/HTTPS allalaadimiskirjastik, mis lahendab ühe peegli kiirusepiirangud, järgude andmekahjustuse ja aeglase sõlme viivituse.

## Sooritusvõime hindamine

Võrdlus [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) ja [ureq](https://crates.io/crates/ureq) allalaadimise läbijooksu puhul avalikes HTTPS peegelallikates:

### Stsenaarium 1: mitme avaliku peegli koondallalaadimine

Node.js v22.14.0 jaotuspaki allalaadimine (`node-v22.14.0-linux-x64.tar.xz`, 28,5 MB); peeglid järjekorras: Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (kaasa arvatud 302 ristallika suunamine).

| Kirjastik / lahendus | Läbijooks (MB/s) |
| --- | ---: |
| msrcget (5 allikat, 8 samaaegset) | 13,9 |
| msrcget + CRC32 | 13,4 |
| down (5 allikat) | 1,2 |
| cyper (üks allikas) | 14,4 |
| reqwest (üks allikas) | 14,4 |
| ureq (üks allikas) | 14,3 |

### Stsenaarium 2: kiire ja aeglase allika segatud allalaadimine

Node.js v22.14.0 pärapaketi allalaadimine (`node-v22.14.0-headers.tar.gz`, 8,4 MB); peeglid järjekorras: Alibaba Cloud (ühe ühenduse kiirusepiirang), Tsinghua, Huawei Cloud, Tencent Cloud, Node.js ametlik allikas (ristriigiline aeglane sõlm).

| Kirjastik / lahendus | Läbijooks (MB/s) |
| --- | ---: |
| msrcget (5 allikat, 8 samaaegset) | 13,5 |
| msrcget + CRC32 | 13,0 |
| down (5 allikat) | 0,2 |
| cyper (üks allikas) | 6,8 |
| reqwest (üks allikas) | 6,6 |
| ureq (üks allikas) | 6,7 |

### Testimismeetod

Hindamiskood asub `examples/bench.rs` (käita `cargo run --profile bench --example bench`):

- Baasvalmistus: enne ajamõõtmist allalaaditakse sihtfail ühe korra ette, kontrollitakse baitside pikkust ja arvutatakse kogu faili ning iga järgu CRC32 baasväärtus; see etapp ei arvestata ajamõõtmisse.
- Ajamõõtmise piirid: iga lahendus käitatakse järjestikku 3 vooru; iga vooru alguses kustutatakse väljundfail ja `.part` vahefail; üks voor algab võrgupalga esitamisel ja lõpetub faili salvestumisel (`cyper`, `reqwest` ja `ureq` käitavad ekspliitselt `sync_data` salvestamist; `msrcget` hõlmab kauguse pikkuse tuvastamist, hõreda faili loomist, samaaegsete järgude allalaadimist, `sync_data` salvestamist, lõppmetaandmete lõikamist ja aatomilist ümbernimetamist).
- Tulemuse kontroll: iga vooru ajamõõtmise peatamise järel loetakse salvestatud faili vooguna ja kontrollitakse faili baitside pikkust ning kogu CRC32-d; kontrollimisprotsess ei arvestata allalaadimisaega.
- Läbijooksu teisendus: võetakse 3 vooru kestuste mediaan ja arvutatakse läbijooks valemiga `faili baitside arv / 1048576 / mediaanisekundid` (`1 MB = 1048576 baityt`).
- Alliku jaotamisreegel: mitme allika lahendused (`msrcget`, `down`) saavad stsenaariumi kõigi 5 peegel-aadressi; ühe allika lahendused (`cyper`, `reqwest`, `ureq`) ei toeta mitme allika järgude allalaadimist ja kasutavad palgalist esimest peegel-aadressi.

## Omadused

- Järgude kaalumine ja rikkevahetus: kutsuja edastab `Verify` kontrolliliidese iga järgu kontrollimiseks; kui kontroll ei läbi või võrgupalg nurjub, vahetatakse automaatselt järgmisele proovimata peegelallikale uuesti proovile; kui kõik allikad on proovitud ja ebaõnnestunud, tagastatakse `Error::Exhausted`.
- Hõred fail ja katkestuspunktist jätkamine: faili kogupikkuse järgi eelnevalt loodud hõred `.part` vahefail koos faililukuga (luku konflikt tagastab `Error::Locked`); faili lõppu kirjutatakse kahe sloti ping-pong CRC32 bitmaap, mis salvestab lõpule viidud järgud; katkestuse ja taastumise järel jätkatakse automaatselt; pärast kõigi järgude lõpetamist lõigatakse lõppmetaandmed ja faili aatomiliselt ümbernimetatakse sihina.
- Binäärne töö vargus: esimesena oma vahealaga allalaadimise lõpetanud allikas lõigab järel olevatest lõpetamata vahealadest suurima kaheks ja võtab omale teise poole, tasakaalustamaks peegelallikate koormust.
- Lõpus mitmesteede võitlus: kui vabade samaaegsete ülesannete arv ületab lõpetamata järgude arvu, edastatakse lisapalgad allikale, kes seda järgu veel ei ole proovinud; esimene kontrollimise läbinud andmeseisund salvestatakse kohe ja sama järgu ülejäänud palgad tühistatakse.
- Automaatne järgude planeerimine: järgu suurus planeeritakse faili kogupikkuse ja samaaegsuse põhjal; olemasoleva `.part` faili jätkamisel kasutatakse automaatselt seal salvestatud järgu suurust.
- Ühenduste taaskasutamine ja protokoli tugi: sisseehitatud sama allika ühenduste pool taaskasutab HTTP/1.1 pikaühendusi, toetab jagatud ülekandekodeerimist, HTTPS-i ([rustls](https://crates.io/crates/rustls)), `301`/`302`/`303`/`307`/`308` ristteelise ja ristallika suunamisi ning ajaajastuse kontrolli.

## Kasutuse näide

Näidiskood asub `examples/basic.rs` (käita `cargo run --example basic`):

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

/// CRC32-põhine järgude kaaluri
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

  // Allikas 0 esmas järg tagastab kahjustatud andme, allikas 1 kiire ja korrektne, allikas 2 kõrge latentsusega aeglane sõlm
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

  // Vaikimisi mitme allika samaaegne allalaadimine: automaatne järgude suuruse planeerimine, kirjutab hõreda .part faili ja lõikab ümbernimega
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() lõpetas: {size_a} baityt, automaatne soovitatav järg {} baityt",
    plan_chunk_size(size_a, 8)
  );

  // Mitme allika allalaadimine järgude CRC32-kaalumisega: rikutud järgudel vahetatakse automaatselt allikat, kiired allikad varuvad automaatselt tööd ja lõpus käib aeglase allikaga võitlus
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
  println!("Get::verify() kontrollis ja lõpetas: {size_b} baityt");

  let _ = remove_file(&out_a);
  let _ = remove_file(&out_b);
  Ok(())
}
```

## Liidese kirjeldus

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>`

Kasutab vaikimisi samaaegsust `CONC` (`8`) ja vaikimisi üksiku ajaajastust `TIMEOUT` (`30 s`) `url_li` allalaadimiseks `path`-ile; tagastab faili kogubaitside arvu.

### `Get`

Ehitaja sisend `Get::new(url_li, path)`, toetab keti seadmisi:

- `.size(size: u64)`: ekspliitselt määrab teadaoleva faili kogubaitside arvu, vahele jätab kauguse pikkuse tuvastamise.
- `.chunk_size(chunk_size: usize)`: määrab ühe järgu baitside arvu (edastamine `0` tagastab `Error::ChunkSize`).
- `.conc(conc: usize)`: määrab samaaegsete allalaadimisülesannete arvu.
- `.timeout(timeout: Duration)`: määrab üksiku palga ja lugemise ajaajastuse.
- `.verify<V2: Verify>(verify: V2)`: sisestab kaaluri, toetab `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` ja `Arc<T>`.
- `.run().await`: käivitab allalaadimise ja tagastab `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` on 0-põhine järgu järjestusnumber, `data` on selle järgu baitsilõik; tagastamine `true` tähendab kontrollimise läbimist.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Tagastab 2 võimu järgu baitside arvu vahemikus `MIN_CHUNK` (`64 KB`) ja `MAX_CHUNK` (`4 MB`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Liidab sufikskonstanti `PART_EXT` (`".part"`), tagastab katkestuspunkti vahefaili tee.

### `Error` ja `Result`

Ühtne tagastustüüp `Result<T> = std::result::Result<T, Error>`; veaenumeeriku variandid hõlmavad: `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
