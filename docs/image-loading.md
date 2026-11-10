# Charger des images en arrière-plan

Le socle fournit un chargement PNG/JPEG borné, sans crate externe. Sous Windows,
GDI+ décode et réduit l’image sur un worker. Le thread UI reçoit un `Arc<Image>`
de pixels BGRA prémultipliés ; les objets natifs restent sur leur thread.

## API et chemin d’exécution

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
