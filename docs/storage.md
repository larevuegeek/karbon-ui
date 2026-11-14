# Préférences et secrets (`karbon_ui::storage`, `karbon_ui::secrets`)

Deux magasins, comme sur le web : `storage` pour les petites préférences de
l'interface (l'équivalent de `localStorage`), `secrets` pour les mots de passe et
les jetons, chiffrés par le système. Aucun crate tiers.

## `storage` : les préférences

```rust
use karbon_ui::storage;

// Au lancement, une fois : le fichier du magasin du processus.
storage::open_local(dossier_de_données.join("local-storage.json"));

storage::local().get("sort");                  // Option<String>
storage::local().set("sort", "title")?;        // écrit le fichier
storage::local().remove("banner_dismissed")?;
```

`LocalStorage::open(chemin)` crée un magasin à part (un par fenêtre, un par
profil…) ; `LocalStorage::in_memory()`, un magasin sans fichier (tests). Avant
`open_local`, le magasin du processus vit en mémoire, comme un navigateur en mode
privé.

- **Fichier** : un objet JSON de chaînes (`{"clé": "valeur"}`), indenté, au format
  qu'écrivent `serde_json` et les navigateurs : un fichier existant se relit tel
  quel.
- **Écriture atomique** : le texte complet va dans un fichier temporaire voisin,
  synchronisé sur le disque, puis renommé par-dessus l'ancien. Une coupure en
  pleine écriture laisse l'ancien fichier ou le nouveau, jamais un mélange.
- **Fichier privé** : créé en `0600` sous macOS et Linux ; sous Windows, le dossier
  du profil est déjà réservé à l'utilisateur.
- **Limites** : clés de 1 à 256 octets, 5 Mio pour tout le magasin (comme les
  navigateurs) ; au-delà, `set` rend `Error::InvalidInput` et ne change rien.
- **Fichier abîmé** : renommé en `*.corrupt` (gardé pour l'examiner) ; le magasin
  repart vide.
- **Échec d'écriture** (disque plein, droits) : `set` rend `Error::Io` ; la valeur
  reste en mémoire pour la session.

Les valeurs ne sont **pas chiffrées** : rien de secret dans `storage`.

## `secrets` : mots de passe et jetons

```rust
use karbon_ui::secrets::Secrets;

let secrets = Secrets::new("dev.rustmusic.RustMusic");   // identifiant stable de l'application
secrets.set("jellyfin/maison", &mot_de_passe)?;
let mot_de_passe = secrets.get("jellyfin/maison")?;     // Option<String>
secrets.remove("jellyfin/maison")?;                     // absent : sans effet
```

| Système | Magasin | Protection |
| --- | --- | --- |
| Windows | Gestionnaire d'identification (identifiant générique `service/clé`) | DPAPI, par utilisateur |
| macOS | Trousseau de session (mot de passe générique : service, compte = clé) | Trousseau |
| Linux | Secret Service par libsecret (GNOME Keyring, KWallet…), attributs `service` et `key` | Trousseau du bureau |

- Illisibles pour un autre utilisateur ou depuis un disque copié ; ils survivent à
  une réinstallation de l'application et apparaissent dans l'outil du système
  (« Gestionnaire d'identification », « Trousseaux d'accès », « Seahorse »).
- **Limites** : service et clé de 1 à 256 octets, secret de 2 048 octets au plus
  (limite du Gestionnaire d'identification), sans caractère nul.
- **Erreurs** : `Error::Unsupported` sans magasin (Linux sans libsecret, autres
  systèmes) ; `Error::Platform` quand le magasin refuse (trousseau verrouillé,
  accès refusé). libsecret est chargée au lancement (`dlopen`) : un système sans
  elle démarre quand même. Une application qui doit fonctionner sans trousseau
  prévoit un repli (redemander le mot de passe, par exemple).
- Les appels sont synchrones : le trousseau peut demander son déverrouillage à
  l'utilisateur. Appelés depuis une tâche de fond, ils ne figent pas la fenêtre.
