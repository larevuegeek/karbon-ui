# Charger des images en arrière-plan

Le socle fournit un chargement PNG/JPEG borné, sans crate externe. Sous Windows,
GDI+ décode et réduit l’image sur un worker. Le thread UI reçoit un `Arc<Image>`
de pixels BGRA prémultipliés ; les objets natifs restent sur leur thread.

## Le plus simple : `ImageLoader`

Pour afficher des images de fichiers dans la vue (pochettes, photos, vignettes),
`resources::ImageLoader` fait tout le travail décrit plus bas, comme `<img src>`
dans un navigateur : l’application le crée, le garde et le déclare ; le runtime
ordonnance ses tâches et lui livre leurs résultats, sans passer par
`take_task_commands` ni `task_completed`.

```rust
use karbon_ui::{application::Resources, resources::ImageLoader, ui::prelude::*};

struct Gallery {
    images: Option<ImageLoader>,
    photos: Vec<String>,
}
impl Application for Gallery {
    type Message = ();
    fn load(&mut self, resources: &dyn Resources) -> karbon_ui::Result<()> {
        self.images = Some(
            ImageLoader::new(resources.image_decoder())
                .budget(128 << 20)       // octets de pixels décodés en cache
                .thumbnail_edge(448)     // plus grand côté décodé par défaut
                .max_in_flight(12),      // décodages simultanés
        );
        Ok(())
    }
    fn image_loader(&self) -> Option<ImageLoader> {
        self.images.clone()
    }
    // Dans la vue : <img src={images.load(&photo)}> pour chaque photo.
}
```

- `load(source)` rend une `ImageSlot` : l’image si elle est en cache, sinon
  l’image d’attente (`placeholder`, transparente par défaut), remplacée et
  repeinte à l’arrivée **sans reconstruire la vue**. `load_or(source, côté,
  repli)` choisit l’image d’attente.
- `get(source)` rend l’image ou `None` ; son arrivée **reconstruit la vue**, pour
  les écrans qui choisissent leur mise en page d’après sa présence.
- `peek` lit le cache sans rien demander ; `preload(sources)` demande d’avance
  (données arrivées, page suivante). Chaque méthode a sa variante `_sized` pour une
  autre taille : une vignette de 160 px et une pochette de 448 px sont deux entrées.
- Une source est un chemin de fichier ou une adresse `data:image/…;base64,…`.
  `resolve(|source, côté| chemin)` choisit le fichier lu, sur le thread de travail :
  une miniature déjà calculée sur disque, un dossier de cache…
- Une source illisible garde son image d’attente et n’est plus redemandée, comme
  une image cassée, jusqu’à `forget_failures()`.
- Les identifiants de ses tâches ont le bit de poids fort posé : ceux de
  l’application doivent rester en dessous de 2⁶³. Un `TaskCommand::CancelAll` de
  l’application emporte aussi ses décodages, qu’il redemande aussitôt.

`Image::dominant_hue()` donne la teinte dominante d’une image et son intensité,
de quoi teinter un fond d’après une pochette (`style:--h={…} style:--s={…}`).

## API et chemin d’exécution

Ce que fait `ImageLoader`, pour une application qui veut piloter elle-même ses
chargements :

1. `Application::load` obtient un service partageable avec `Resources::image_decoder()`.
2. L’application place une `Task` dans sa file, avec le chemin et ce service.
3. Le worker appelle `load_image_file(decoder, path, options, &cancel)`.
4. Le chargeur vérifie le fichier, lit par blocs et inspecte les dimensions
   PNG/JPEG avant de demander le décodage natif.
5. Le backend produit une miniature puis libère ses ressources natives.
6. `task_completed` reçoit le résultat sur le thread UI ; l’application met son
   cache à jour et demande une nouvelle vue.
7. Le renderer transforme les commandes `Image` de la scène en bitmaps natifs.

`ImageDecoder: Send + Sync` décrit le service, pas un objet GDI+ partagé.
Le service Windows crée son contexte COM/GDI+ à chaque décodage sur le thread
appelant. `Resources` peut renvoyer `None` sur un backend sans décodeur.
`LoadedImage` contient l’image, le format et les dimensions source, les octets
lus et les deux durées. Le résultat d’une tâche annulée ou remplacée est rejeté
par le service de tâches avant de modifier l’application.

## Budgets et limites

| Option `ImageLoadOptions` | Défaut |
| --- | --- |
| `max_file_bytes` | 8 Mio encodés par fichier |
| `max_source_pixels` | 16 millions de pixels source |
| `max_source_edge` | 8 192 pixels par côté source |
| `thumbnail_edge` | 192 pixels au maximum par côté de sortie |

La miniature conserve les proportions et n’agrandit pas les petites sources.
Les limites configurables sont elles-mêmes plafonnées : 64 Mio encodés,
64 millions de pixels, 16 384 par côté source et 2 048 par côté de miniature.
Les dimensions sont vérifiées avant le décodage et à nouveau par le backend.
L’inspection d’en-tête ne remplace pas la validation complète du codec natif.

Ces limites ne garantissent pas une borne de RAM du processus : GDI+ peut
allouer des surfaces source et des buffers temporaires. L’application choisit séparément le budget de son `ImageCache<K>`. Les scènes,
les résultats en transit et les textures GPU sont hors du budget de ce cache.

Les échecs sont typés : `Error::Io` pour l’ouverture ou la lecture du fichier,
`Error::Cancelled` pour un jeton annulé, et `Error::Image(ImageError::…)` pour
un format non pris en charge (`UnsupportedFormat`), un en-tête tronqué
(`Malformed`), un dépassement de budget (`TooLarge`), des limites invalides
(`InvalidLimits`) ou un refus du décodeur natif (`Decode`) :

```rust,ignore
match load_image_file(decoder.as_ref(), &path, options, &cancel) {
    Ok(loaded) => Some(loaded.image),
    Err(Error::Cancelled) => None,
    Err(Error::Image(ImageError::TooLarge { reason })) => { eprintln!("{reason}"); None }
    Err(error) => { eprintln!("{error}"); None }
}
```

L’annulation est vérifiée entre les lectures, avant et après l’appel natif.
Elle ne peut pas interrompre un décodage GDI+ déjà en cours. Les sources de
plusieurs fichiers d’un même lot sont décodées successivement.

L’orientation EXIF automatique, les images animées, les autres formats,
l’extraction de pochettes audio et la surveillance du dossier restent à ajouter.

## Fichiers et validation

- `src/resources/image_loader/loader.rs` : limites, inspection et lecture commune.
- `src/platform/windows/imaging.rs` : service natif et miniature.

Les tests couvrent PNG, JPEG natif, troncature, budgets, annulation, confinement
du service sur un autre thread, erreurs partielles, alias de cache et recadrage.
Le mode fichiers a aussi tourné dans le [protocole hors écran](archive/performance.md).
Sa recette visible GPU/WARP est maintenant consignée dans le
[relevé avec la file](archive/queue-validation.md).
