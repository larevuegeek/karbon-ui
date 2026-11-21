# Surface partagée sur la carte graphique (0.3.24)

Une fenêtre peut montrer, à la place d'un élément, l'image qu'un autre moteur de rendu
dessine sur la carte graphique : la vue 3D d'un éditeur. L'image ne passe pas par la
mémoire de l'ordinateur, et l'interface n'est pas repeinte à chaque image du moteur.

Windows seulement (Direct3D 11 et DirectComposition). Ailleurs,
`SharedSurface::new` répond `Error::Unsupported` : l'application garde le mode copie
(`LiveImage`, voir [performance.md](performance.md), « Mode copie allégé »).

## Pour l'application

```rust
use karbon_ui::ui::shared_surface::SharedSurface;

// Sur le fil du moteur. `luid` : la carte du moteur (Vulkan :
// `VkPhysicalDeviceIDProperties::deviceLUID`).
let surface = SharedSurface::new(Some(luid), 1740, 1176)?;
// Le moteur importe la texture : `surface.texture_handle()` (poignée NT d'une
// `ID3D11Texture2D`, `DXGI_FORMAT_B8G8R8A8_UNORM_SRGB`) et `surface.adapter()`.
// Puis, pour chaque image :
//   1. le moteur prend le verrou de la texture avec la clé 0, dessine, le rend avec la clé 1 ;
//   2. `surface.present()?` montre l'image et rend la texture (clé 0).
```

Dans la vue, l'élément peint le **gabarit** de la surface :

```rust
fn paint(&self, context: &PaintContext, scene: &mut Scene) {
    scene.commands.push(DrawCommand::Image {
        rect: context.rect,
        image: self.surface.placeholder(),
    });
}
```

- `surface.placement()` rend la place de l'élément en pixels dès que la fenêtre montre
  la surface ; sa taille est celle que le moteur doit dessiner. `None` : la fenêtre ne
  peut pas la montrer (chemin « layered », autre système) et l'application copie.
- Changement de taille : le moteur lâche la texture, `surface.resize(l, h)`, puis il
  importe la nouvelle `texture_handle()`. La fenêtre garde la dernière image, étirée,
  jusqu'à la première image présentée à la nouvelle taille.
- `SharedSurface` se clone et se partage entre fils : la vue peint le gabarit sur le fil
  de l'interface, le moteur appelle `present` et `resize` sur le sien.
- `surface.read_pixels()` relit l'image au lieu de la montrer (contrôles).
- Tout ce que l'interface peint après l'élément (menu, bulle, texte posé sur la vue)
  reste par-dessus la surface.

Limite : l'élément ne doit pas être dans un groupe d'opacité, une découpe arrondie ni un
masque. Le trou serait fait dans le calque du groupe, pas dans l'image de la fenêtre.

L'essai complet, avec un vrai moteur, est dans le dépôt de karbon-engine :
`tools/shared-view`.

## Les choix

### Partage : Direct3D crée, l'autre importe

La texture est créée par Direct3D 11 avec `D3D11_RESOURCE_MISC_SHARED_NTHANDLE |
D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX`, et le moteur l'importe (Vulkan :
`VK_KHR_external_memory_win32`, type `D3D11_TEXTURE`). C'est le sens que tous les pilotes
acceptent ; l'inverse (Vulkan exporte, Direct3D ouvre) n'est pas porté partout.

### Synchronisation : le verrou à clé

Le verrou à clé (`IDXGIKeyedMutex`, côté Vulkan `VK_KHR_win32_keyed_mutex`) plutôt qu'une
barrière partagée (`ID3D11Fence` importée en sémaphore) :

- il est attaché à la texture : rien d'autre à créer, exporter ni importer ;
- les deux clés (0 : libre pour le moteur, 1 : image dessinée) font du passage de main un
  protocole que chaque côté vérifie. `present` sans image dessinée échoue au bout d'une
  seconde (`Error::TimedOut`) au lieu de montrer une image à moitié faite ;
- les deux cartes de la machine de développement (Radeon intégré, RTX 4080) le portent.

`AcquireSync` rend `WAIT_TIMEOUT` comme un succès de `HRESULT` : le code brut est lu
(`shared_surface/texture.rs`).

### Une seule texture, une copie sur la carte

DWM ne lit jamais la texture partagée. `present` la recopie (`CopyResource`, sur la
carte) dans le tampon d'une chaîne d'échange de composition, sous le verrou, puis
présente. Un anneau de textures montrées telles quelles éviterait cette copie, mais
demande l'interface de présentation de Windows 11 (`IPresentationManager`). `present`
tout entier (prise du verrou, copie, remise, présentation) coûte 0,13 à 0,15 ms au fil du
moteur pour 1656 × 1218 pixels sur la RTX 4080 : l'anneau attendra une mesure qui le
réclame.

La chaîne d'échange a **quatre tampons** et le modèle « flip discard » (la dernière image
présentée remplace celles qui attendaient l'écran). Le nombre vient de la mesure, sur une
RTX 4080 et un écran à 60 Hz :

| Tampons | Producteur à 60 images par seconde | Producteur qui suit une souris à 125 Hz |
| --- | --- | --- |
| 2 | par moments 5 à 15 ms d'attente par image : 1,2 % des images sur huit tours de 15 s, et dans une autre série un tour presque entier sur trois (médiane 14,5 ms) | non mesuré |
| 3 | 0,15 % des images (11 sur 7 470) | une image sur deux attend l'écran (5 à 13 ms) |
| 4 | aucun tour lent sur trois (95ᵉ centile du dessin : 3,1 à 4,0 ms) | aucune attente vue (95ᵉ centile du dessin : 1,3 ms) |

L'attente n'est pas dans `present`, qui rend la main en 0,13 ms : elle est sur la carte.
La copie vers un tampon que Windows tient encore attend l'écran, et le dessin suivant du
moteur, qui reprend la texture, attend la copie (`vkQueueWaitIdle` long, `vkQueueSubmit`
court). Lecture des mesures, non vérifiée autrement : Windows garde le tampon affiché et
celui d'avant, il reste donc « tampons − 2 » présentations libres par période d'écran. Un
producteur qui présente plus de deux fois par période (souris à 1 000 Hz) attendra de
nouveau l'écran : il doit regrouper ses images, ou la chaîne recevoir plus de tampons
(8 Mo chacun pour 1740 × 1176 pixels).

### Latence

Le chemin de présentation est celui de la surface, pas celui de la fenêtre : une image
est montrée dès que le moteur l'a finie, sans attendre la peinture de l'interface, qui
suit la cadence de l'écran. Mesuré dans `tools/shared-view` de karbon-engine (protocole
de son éditeur : du mouvement du pointeur reçu par la fenêtre à l'image remise à
Windows), RTX 4080, vue de 1740 × 1176 : **16,8 ms en mode copie, 2,7 ms en mode
partage** (médianes ; 95ᵉ centiles 17,0 et 3,7 ms). Relevé complet : `docs/mesures.md`
de karbon-engine.

### La carte : un périphérique à part, sur le LUID demandé

La surface a son propre périphérique Direct3D 11, créé sur la carte que l'application
désigne par son LUID (`IDXGIFactory4::EnumAdapterByLuid`). Le périphérique du renderer de
la fenêtre n'est pas touché : l'interface reste dessinée sur la carte par défaut, la
surface vit sur celle du moteur, et DWM compose les deux chaînes d'échange. Le partage
marche donc quelle que soit la carte du moteur (vérifié sur les deux). Autre avantage :
`present` se fait sur le fil du moteur, sans le contexte Direct3D de l'interface.

### Composition : une seconde cible, derrière l'interface

Une fenêtre peut avoir deux cibles DirectComposition. Celle de l'interface
(`present/composition.rs`) est créée au-dessus (`topmost = true`) et n'est pas changée.
Quand un gabarit de surface est peint, `present/surface_visual.rs` crée la seconde,
derrière (`topmost = false`), avec un visuel par surface dont le contenu est la chaîne
d'échange de la surface.

Le trou : en dessinant le gabarit, Direct2D efface son rectangle en transparent
(`graphics/surface_hole.rs`). La fenêtre prend alors l'alpha prémultiplié même agrandie.
Aucune commande de dessin nouvelle : le gabarit est un `DrawCommand::Image` ordinaire,
reconnu par son adresse (`ui/shared_surface/registry.rs`). Un backend qui ne le reconnaît
pas peint l'image, un rectangle noir.

Ce que les fichiers existants ont reçu, en lignes ajoutées seulement :

| Fichier | Ajout |
| --- | --- |
| `graphics/canvas.rs` | dans le dessin d'une image : le gabarit d'une surface devient un trou |
| `present/paint.rs` | avant la peinture, rattachement d'une chaîne changée et alpha forcé ; après la présentation, mise en place des visuels |
| `events/procedure.rs` | au démontage de la fenêtre, sa couche de surfaces est défaite (sans cela, une application qui lâchait sa surface après sa fenêtre sortait avec le code 0x87A) |
| `graphics/mod.rs`, `present/mod.rs`, `platform/windows/mod.rs`, `platform/mod.rs`, `ui/mod.rs`, `tests/mod.rs` | déclaration des modules neufs |
| `Cargo.toml` | fonction `Win32_Security` du crate `windows` (`CreateSharedHandle`) |

Sans surface partagée vivante dans le processus, le seul coût est la lecture d'un compteur
atomique par image dessinée, et la consultation d'une table vide par peinture.

## Contrôles

- `src/platform/windows/tests/shared_surface.rs` : fenêtre composée (couche créée, place de
  l'élément en pixels, `present` sans image refusé, couche défaite à la fermeture) et
  fenêtre « layered » (gabarit peint) ; témoin négatif fait : sans la reconnaissance du
  gabarit dans `canvas.rs`, le premier test échoue ;
- `src/platform/windows/graphics/surface_hole.rs` : rectangle de pixels d'un élément ;
- `src/ui/shared_surface/surface.rs` : seul le gabarit d'une surface vivante est reconnu ;
- dans karbon-engine, `src/app/run/shared_check.rs` : l'image relue de la texture est
  identique octet pour octet à celle du mode copie, sur les deux cartes, sous la couche de
  validation de Khronos.

## Ce qui reste

- Linux (KUI-007) et macOS : non écrits.
- Une seule texture : pas d'anneau, donc une copie sur la carte par image.
- Perte du périphérique de la surface : `present` rend `Error::DeviceLost` ;
  l'application doit refaire la surface. Jamais provoquée.
- Plusieurs surfaces dans une fenêtre : écrit (un visuel par surface), jamais essayé.
- Élément dans un groupe d'opacité, une découpe arrondie ou un masque : non géré.
- La mise en place des visuels et la présentation de l'interface ne sont pas une seule
  transaction : pendant un redimensionnement, le trou et la surface peuvent différer d'une
  image.
