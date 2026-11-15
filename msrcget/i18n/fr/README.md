# msrcget

Bibliothèque de téléchargement HTTP/HTTPS multi-source par blocs, fondée sur le modèle de complétion asynchrone [compio](https://crates.io/crates/compio), conçue pour pallier la limitation de débit d'un miroir unique, la corruption de données par blocs et le freinage dû aux nœuds lents.

## Évaluation des performances

Comparaison du débit de téléchargement sur des miroirs HTTPS publics entre [down](https://crates.io/crates/down), [cyper](https://crates.io/crates/cyper), [reqwest](https://crates.io/crates/reqwest) et [ureq](https://crates.io/crates/ureq) :

### Scénario 1 : téléchargement agrégé depuis plusieurs miroirs publics

Téléchargement du paquet de distribution Node.js v22.14.0 (`node-v22.14.0-linux-x64.tar.xz`, 28,5 Mo). Ordre des miroirs : Tsinghua, USTC, Huawei Cloud, Tencent Cloud, npmmirror (avec redirection 302 inter-source).

| Bibliothèque / Solution | Débit (MB/s) |
| --------------------- | ----------: |
| msrcget (5 sources, 8 concurrents) |        13.9 |
| msrcget + CRC32       |        13.4 |
| down (5 sources)      |         1.2 |
| cyper (source unique) |        14.4 |
| reqwest (source unique) |        14.4 |
| ureq (source unique)  |        14.3 |

### Scénario 2 : téléchargement mixte de sources rapides et lentes

Téléchargement du paquet d'en-têtes Node.js v22.14.0 (`node-v22.14.0-headers.tar.gz`, 8,4 Mo). Ordre des miroirs : Alibaba Cloud (limitation de débit par connexion unique), Tsinghua, Huawei Cloud, Tencent Cloud, source officielle Node.js (nœud lent transfrontalier).

| Bibliothèque / Solution | Débit (MB/s) |
| --------------------- | ----------: |
| msrcget (5 sources, 8 concurrents) |        13.5 |
| msrcget + CRC32       |        13.0 |
| down (5 sources)      |         0.2 |
| cyper (source unique) |         6.8 |
| reqwest (source unique) |         6.6 |
| ureq (source unique)  |         6.7 |

### Méthodologie de test

Le code de benchmark se trouve dans `examples/bench.rs` (exécution via `cargo run --profile bench --example bench`) :

- Préparation de la référence : avant tout chronométrage, un pré-téléchargement du fichier cible est effectué pour vérifier la longueur en octets et calculer les valeurs de référence CRC32 (paquet complet et blocs individuels). Cette phase n'est pas comptée dans le temps mesuré.
- Bornes de chronométrage : chaque solution est exécutée de manière indépendante et continue sur 3 tours ; avant chaque tour, le fichier de sortie et le fichier temporaire `.part` sont supprimés. Un tour est chronométré depuis l'initiation de la requête réseau jusqu'à l'achèvement de l'écriture du fichier sur disque (`cyper`, `reqwest` et `ureq` exécutent explicitement `sync_data` pour forcer l'écriture sur disque ; `msrcget` couvre l'ensemble du flux : détection de la longueur distante, création d'un fichier creux, téléchargement parallèle par blocs, troncature des métadonnées de fin, `sync_data` pour forcer l'écriture et renommage atomique).
- Vérification du produit : après l'arrêt du chronomètre de chaque tour, le fichier écrit est lu en flux pour vérifier sa longueur en octets et le CRC32 du paquet complet. Cette vérification n'est pas comptée dans le temps de téléchargement.
- Conversion en débit : la médiane des durées des 3 tours est prise, et le débit est calculé selon la formule `octets du fichier / 1048576 / secondes médianes` (`1 Mo = 1048576 octets`).
- Règle de répartition des sources : les solutions multi-source (`msrcget`, `down`) reçoivent les 5 adresses de miroirs du scénario ; les solutions à source unique (`cyper`, `reqwest`, `ureq`) ne prenant pas en charge le téléchargement multi-source par blocs, seul le 1er adresse de la liste est utilisée.

## Fonctionnalités

- Vérification bloc par bloc et basculement en cas de panne : l'appelant fournit une implémentation de l'interface `Verify` pour valider chaque bloc. En cas d'échec de vérification ou de requête réseau, la bascule est automatique vers un autre miroir non encore essayé ; si toutes les sources ont été épuisées en vain, `Error::Exhausted` est renvoyé.
- Fichiers creux et reprise en cas d'interruption : un fichier temporaire `.part` creux est créé à l'avance selon la longueur totale du fichier, avec un verrou de fichier (en cas de conflit de verrou, `Error::Locked` est retourné) ; un bitmap est ajouté à la fin du fichier pour marquer les blocs terminés, permettant une reprise automatique après une interruption et un redémarrage ; une fois tous les blocs terminés, les métadonnées de fin sont tronquées et le fichier est renommé de manière atomique sous le nom de destination.
- Vol de tâches par dichotomie : une source ayant achevé son intervalle de téléchargement divise en deux l'intervalle restant le plus long et prend en charge la seconde moitié, afin d'équilibrer la charge entre les miroirs.
- Course multi-voix de fin de course : lorsque le nombre de coroutines libres excède le nombre de blocs restants, des demandes redondantes sont envoyées vers les miroirs n'ayant pas encore tenté ce bloc ; dès que les premières données validées sont écrites sur disque, les autres requêtes pour ce bloc sont immédiatement annulées.
- Planification automatique des blocs : la taille des blocs est déterminée en fonction de la longueur totale du fichier et du degré de parallélisme ; lors d'une reprise sur un fichier `.part` existant, la taille de bloc enregistrée y est réutilisée automatiquement.
- Réutilisation de connexions et support des protocoles : un pool de connexions intégré réutilise les connexions longue durée HTTP/1.1 vers une même source. Prise en charge du codage de transfert par blocs, de HTTPS ([rustls](https://crates.io/crates/rustls)), des redirections `301`/`302`/`303`/`307`/`308` (inter-chemin et inter-source) et de la gestion des dépassements de délai.

## Démonstration d'utilisation

Le code d'exemple se trouve dans `examples/basic.rs` (exécution via `cargo run --example basic`) :

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

:/// Vérificateur de bloc par bloc basé sur le CRC32
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

  // Source 0 : le 1er bloc renvoie des données corrompues, source 1 : rapide et correcte, source 2 : nœud lent à haute latence
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

  // Téléchargement concurrentiel multi-source par défaut : planification automatique de la taille des blocs, écriture d'un fichier .part creux puis troncature et renommage
  let size_a = get([&u0, &u1, &u2], &out_a).await?;
  println!(
    "get() 完成: {size_a} 字节，自动建议分块 {} 字节",
    plan_chunk_size(size_a, 8)
  );

  // Téléchargement multi-source avec vérification CRC32 bloc par bloc : bascule automatique de source sur bloc corrompu, vol de tâches et course de fin entre sources rapides et lentes
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

## Documentation de l'API

### `get`

`pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64> where I: IntoIterator<Item = S>, S: AsRef<str>`

Télécharge `url_li` vers `path` en utilisant le nombre de concurrents par défaut `CONC` (`8`) et le délai d'attente par défaut par requête `TIMEOUT` (`30 secondes`), et renvoie la taille totale du fichier en octets.

### `Get`

Point d'entrée du constructeur `Get::new(url_li, path)`, configurant par chaînage :

- `.size(size: u64)` : spécifier explicitement la taille totale connue du fichier en octets, en sautant la détection de la longueur distante (passer `0` termine comme un fichier vide).
- `.chunk_size(chunk_size: usize)` : définir la taille d'un bloc en octets (passer `0` renvoie `Error::ChunkSize`).
- `.conc(conc: usize)` : spécifier le nombre de coroutines de téléchargement parallèle (passer `0` est traité comme `1`).
- `.timeout(timeout: Duration)` : définir le délai d'attente par requête et par lecture.
- `.verify<V2: Verify>(verify: V2)` : injecter un vérificateur ; des implémentations sont fournies pour `()`, `Fn(usize, &[u8]) -> bool`, `Rc<T>` et `Arc<T>`.
- `.run().await` : lancer le téléchargement et renvoyer `Result<u64>`.

### `Verify`

`pub trait Verify { fn verify(&self, idx: usize, data: &[u8]) -> bool; }`

`idx` est l'indice de bloc en base 0, `data` est la tranche d'octets du bloc concerné ; retourner `true` indique que la vérification est passée.

### `plan_chunk_size`

`pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize`

Renvoie une taille de bloc (puissance de 2) comprise entre `MIN_CHUNK` (`64 Ko`) et `MAX_CHUNK` (`4 Mo`).

### `part_path`

`pub fn part_path(path: impl AsRef<Path>) -> PathBuf`

Concatène le suffixe constant `PART_EXT` (`".part"`) et renvoie le chemin du fichier temporaire de reprise.

### `Error` et `Result`

Type de renvoi unifié `Result<T> = std::result::Result<T, Error>` ; les variantes de l'enum d'erreurs comprennent : `EmptySrc`, `ChunkSize`, `Url`, `Scheme`, `Io`, `Tls`, `Timeout`, `Redirect`, `Status`, `Proto`, `Probe`, `Locked`, `Exhausted { idx }`.
