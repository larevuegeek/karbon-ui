# Ressources de l’exemple

- `fonts/` : Figtree, Inter et JetBrains Mono, packages `@fontsource-variable`
  présents dans le projet RustMusic local. Les licences SIL Open Font License sont
  conservées dans les fichiers `*-LICENSE.txt` voisins.
- `covers/` : images générées (un disque sur un dégradé), aux noms et dimensions des
  pochettes de la maquette ; aucune pochette d’album réelle n’est distribuée, voir
  `covers/README.md`. `../tools/prepare-covers.py` sert seulement à refaire, en local, un
  état de comparaison avec l’application d’origine : il lit le cache de RustMusic sans
  jamais l’écrire, et ses sorties ne sont pas suivies par git.

Les icônes sont des SVG compilés, rangés dans `../icons/` (voir `../icons.rs`) ; elles
viennent de jeux d’icônes tiers, dont les licences sont dans `../icons/LICENSES.md`.
Les ressources incluses suffisent à compiler hors ligne.
