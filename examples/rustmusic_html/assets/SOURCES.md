# Ressources de l’exemple

- `fonts/` : Figtree, Inter et JetBrains Mono, packages `@fontsource-variable`
  présents dans le projet RustMusic local. Les licences SIL Open Font License sont
  conservées dans les fichiers `*-LICENSE.txt` voisins.
- `covers/*.jpg` : pochettes de l’état de référence (captures de l’application
  Tauri), tirées du cache de RustMusic par `../tools/prepare-covers.py`, qui lit ce
  cache sans jamais l’écrire.
- `covers/*.png` : pochettes des sections sous la ligne de flottaison (playlists,
  genres, ajouts récents). Elles viennent de l’ancien exemple natif `rustmusic`
  (supprimé) ; leurs sources et liens d’albums figurent dans
  `covers/covers-sources.json`. Ce sont des illustrations originales des albums,
  utilisées ici comme données de la maquette.
- `covers/radio_logo.png` : logo transparent de la dernière radio écoutée.

Les icônes sont des SVG compilés, rangés dans `../icons/` (voir `../icons.rs`).
Les ressources incluses suffisent à compiler hors ligne.
