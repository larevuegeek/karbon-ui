# Ressources partagées des tests

Ressources tierces ou binaires, partagées par les tests d’intégration (`tests/`) et
les tests unitaires de `src/`. Elles ne sont pas chargées par la bibliothèque en
fonctionnement normal. Les feuilles CSS et les templates propres à un module de test
sont rangés à côté de lui (voir [docs/tests.md](../../docs/tests.md)).

- `bootstrap/bootstrap.css` : Bootstrap 5.3.3, compilé en mode framework par
  `tests/frameworks/bootstrap/` et mesuré par les outils de compatibilité de
  `crates/ui-macros`. Licence MIT : voir `bootstrap/LICENSE`.
- `icons/heart.svg` : icône chargée par `<svg src>` (`tests/templates/svg.rs`).
- `images/cover.png` : image PNG de référence conservée pour les tests de
  décodage, de miniature et de validation des limites.
- `fonts/figtree.woff2` : sous-ensemble latin variable normal de Figtree,
  issu de `@fontsource-variable/figtree`. Copyright The Figtree Project Authors.
  Voir `fonts/FIGTREE-LICENSE.txt` (SIL OFL 1.1).
- `fonts/inter.woff2` : sous-ensemble latin variable normal d’Inter,
  issu de `@fontsource-variable/inter`. Copyright The Inter Project Authors.
  Voir `fonts/INTER-LICENSE.txt` (SIL OFL 1.1).

Les ressources ont été déplacées des anciennes démonstrations vers ce dossier
pour que les tests du moteur restent indépendants des applications de démo.
