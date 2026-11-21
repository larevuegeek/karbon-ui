# Composants d'éditeur (0.3.23, ajouts 0.3.25)

Le module `karbon_ui::ui::editor` réunit des composants serrés pour les outils d'édition
(éditeur de niveau, inspecteur) : un arbre virtualisé, des champs numériques qu'on fait
glisser, une grille de propriétés et une barre de menus dans la fenêtre. Ils suivent le
reste de la bibliothèque : l'application garde leur état (`TreeState`, `FieldState`,
`MenuBarState`), la vue les construit d'après lui, leurs messages reviennent à
`Application::update`. Couleurs et mesures : `EditorPalette` (`DARK`, `LIGHT` ; lignes
de 22, texte de 12, axes X, Y, Z en rouge, vert, bleu).

Exigences du cahier de l'éditeur de karbon-engine (`docs/editeur/06-karbon-ui.md`) :
KUI-029 (arbre), KUI-031 (champs), KUI-030 (grille), KUI-032 et KUI-034 (menus), et,
pour les entrées et la fenêtre, KUI-015, KUI-016, KUI-026, KUI-002 (plus bas).

Exemple qui les réunit, textes en français :

```powershell
cargo run --release --example editor_components
cargo run --release --example editor_components -- --bench 20    # défilement mesuré
cargo run --release --example editor_components -- --export x.png --menu 2
```

Tout est fait **par ajout** : modules neufs, fonctions et méthodes neuves ; un fichier
existant ne reçoit que des lignes ajoutées (`git diff --numstat`). Les gestes de la
souris passent par `Element::on_pointer` et les touches par `Application::key`, qui
n'arrivent encore que **sous Windows**.

## Arbre virtualisé (`TreeView`, KUI-029)

```rust
// L'application décrit les enfants de chaque nœud, par clé stable.
impl TreeSource for Level {
    fn children(&self, parent: Option<u64>) -> &[u64] { … }
}
let mut tree = TreeState::new(&level);                 // tout replié
// Vue : seules les lignes visibles sont construites.
TreeView::new("tree", &self.tree, Msg::Tree, |item| label(item.scope.child("name"), …, &p))
    .focused(self.panel == Panel::Tree)
    .element()
// Messages et touches.
Msg::Tree(m) => match self.tree.update(m, &self.level).event { … }
fn key(&mut self, press: KeyPress) -> bool { self.tree.key(press, &self.level).rebuild }
```

- `TreeState` aplatit les seules branches dépliées en lignes visibles (`rows()`), garde
  la sélection **par clé** (gardée après un tri : `refresh` relit la source), le curseur
  et l'ancre de Maj.
- Souris : clic pour choisir, Ctrl+clic pour ajouter ou retirer, Maj+clic pour une plage,
  clic sur la flèche pour plier, double-clic → `TreeEvent::Activated`, clic droit qui
  choisit la ligne avant le menu contextuel. Glisser des lignes sur une autre →
  `TreeEvent::Dropped(TreeDrop { keys, target, place })`, `place` valant `Into` au milieu
  de la ligne (rattacher à un parent), `Before` ou `After` près de ses bords. Une ligne
  ne se dépose ni sur elle-même ni dans ses descendants ; Échap abandonne le glisser.
- Clavier (`TreeState::key`) : flèches haut et bas, Début, Fin, pages ; Maj étend, Ctrl
  déplace le curseur seul, Ctrl+Espace ajoute la ligne du curseur, Ctrl+A choisit tout ;
  gauche plie ou remonte au parent, droite déplie ou descend au premier enfant ; Entrée
  → `Activated`. La vue suivante fait défiler la liste jusqu'au curseur
  (`VirtualList::reveal`, ajout), oublié dès que l'utilisateur fait défiler.
- Il n'y a pas encore de focus clavier par panneau (KUI-013) : l'application dit à quel
  panneau vont les touches (`focused`, et l'aiguillage dans `Application::key`).

## Champs numériques (`NumberField`, `VectorField`, KUI-031)

```rust
NumberField::new(4, p.intensity, &self.fields, Msg::Field)
    .range(0., 100.).step(0.5).unit(" lx").element()
VectorField::new(1, p.position, &self.fields, Msg::Field).step(0.01).unit(" m").element()
// Dans update :
let reply = self.fields.update(m);
if let Some(e) = reply.event { /* e.field.key, e.field.axis, e.value, e.change */ }
```

- Glisser horizontalement change la valeur : `step` par pixel, **Maj dix fois plus vite,
  Ctrl dix fois plus fin**, bornée, arrondie au pas et aux décimales. Pendant le geste des
  `FieldChange::Preview`, à la fin **une seule** `FieldChange::Commit` (pour l'historique
  d'annulation) ; Échap (donné à `FieldState::key`) rend la valeur d'avant
  (`FieldChange::Cancel`).
- Un clic sans glisser remplace le champ par une zone de saisie qui prend le clavier,
  valeur choisie : nombres à point ou virgule, `+ - * /`, parenthèses (« 2*3 »), unité
  tolérée. Entrée, Tab ou un clic ailleurs valident ; Échap abandonne ; une saisie
  illisible garde la valeur.
- `VectorField` : trois champs X, Y, Z, lettre et barre à la couleur de l'axe.
  `mixed` montre « — » quand plusieurs objets choisis diffèrent.
- Ajout fait pour cela : **`TextInput::autofocus()`**, un champ qui prend le clavier en
  apparaissant, tout son texte choisi (comme `autofocus` en HTML). Une application qui ne
  s'en sert pas n'en paie rien (même principe que les gestionnaires : aucun parcours de
  l'arbre sans champ `autofocus` construit).
- Le pointeur n'est pas verrouillé (KUI-014) : le glisser s'arrête au bord de l'écran.

## Grille de propriétés (`PropertyGrid`, KUI-030)

```rust
PropertyGrid::new("properties", Msg::Fold)
    .section(PropertySection::new(1, "Objet", open)
        .row(Property::new("Nom", text_field("name", &p.name, Msg::Name, &palette)))
        .row(Property::new("Visible", checkbox("visible", Some(p.visible), Msg::Visible, &palette))
            .tooltip("Montre l'objet dans la vue et dans le jeu.")))
    .element()
```

Sections qui se plient (clic, ou Entrée et Espace au clavier sur le titre), lignes
« libellé, contrôle » alignées sur une colonne (`label_width`, 110 par défaut), bulle
d'aide par ligne, panneau qui défile. Contrôles serrés : `checkbox` (trois états,
« — » pour mélangé), `text_field`, `choice` (la liste déroulante existante, à la
palette), `NumberField`, `VectorField`, `color_swatch` (pastille et valeur
hexadécimale ; un clic envoie un message, le sélecteur viendra plus tard), `read_only`.

## Barre de menus (`MenuBar`, KUI-032 et KUI-034)

```rust
fn menus(&self) -> Vec<Menu<Msg>> {
    vec![Menu::new("&Fichier")
        .entry(MenuEntry::new("Enregistrer", Msg::Save).shortcut("Ctrl+S"))
        .entry(MenuEntry::submenu("Récents", vec![…]))
        .separator()
        .entry(MenuEntry::new("Annuler", Msg::Undo).enabled(self.can_undo()))]
}
let (bar, overlays) = MenuBar::new("menus", &self.menus, Msg::Menus, self.menus()).build();
// update : le choix revient dans `reply.command`, à appliquer comme un message de la vue.
Msg::Menus(m) => { let r = self.menus.update(m); match r.command { Some(c) => self.update(c), None => r.rebuild } }
```

- Clic sur un titre : son menu s'ouvre dessous ; un clic sur un autre titre ouvre
  celui-ci tout de suite. Entrées serrées (22 de haut) : case (`checked`), choix exclusif
  (`radio`), raccourci affiché à droite, entrée grisée (`enabled(false)`), sous-menu à
  droite, ouvert quand le pointeur s'y pose ou par Entrée.
- Clavier (`MenuBarState::key`) : F10 ouvre le premier menu, Alt avec la lettre qui suit
  `&` dans le titre ouvre ce menu, gauche et droite passent d'un menu à l'autre ; dans un
  menu, flèches, Entrée et Échap comme dans tout menu (Échap ne ferme que le sous-menu).
- Bulles riches (KUI-034) : `tip(titre, description, raccourci, &palette)` donne une bulle
  « Titre (Ctrl+S) » suivie de sa description, aux couleurs de la palette.

Limites : le pointeur qui passe d'un titre à l'autre, menu ouvert, n'ouvre pas l'autre
menu (un clic le fait) ; il ne passe pas non plus d'une entrée du menu à l'autre tant qu'un
sous-menu est ouvert (Échap ou un clic le ferment). Pas de lettre soulignée dessinée.
Le registre de commandes (KUI-044) n'existe pas : les raccourcis affichés sont des textes.

## Entrées et fenêtre

- **Rang du clic** (KUI-015) : `PointerEvent::clicks` vaut 1, 2 (double-clic), 3… pour
  les appuis du même bouton à moins de 500 ms et 4 pixels logiques l'un de l'autre (les
  valeurs par défaut de Windows) ; un relâchement porte le rang de son appui, un
  mouvement 0.
- **Molette avec Maj, Ctrl et Alt** (KUI-016) : `Element::on_pointer_wheel`, une
  `PointerEvent` de phase `PointerPhase::Wheel`, le tour en crans dans `wheel`. Le premier
  gestionnaire qui rend un message prend la molette : ni `on_wheel` ni défilement. C'est
  un gestionnaire **à part** : les `on_pointer` existants ne voient jamais la molette (un
  gestionnaire qui rendait un message pour toute phase aurait sinon volé le zoom de
  l'éditeur de karbon-engine).
- **Fermeture annulable** (KUI-026) : `Application::close_requested(&mut self) -> bool`,
  `true` par défaut. Demandée quand l'utilisateur ferme (bouton de la barre de titre,
  Alt+F4 et barre des tâches par `WM_CLOSE`, bouton de fermeture par UI Automation) ; un
  refus garde la fenêtre et reconstruit la vue (l'application montre sa question), et
  `WindowCommand::Close` ou `Exit` ferment ensuite sans redemander. Sous Windows
  seulement ; `HeadlessApp::request_close` pour les tests.
- **Mode copie allégé** (KUI-002) : `Image::opaque_bgra(largeur, hauteur, pixels)` force
  l'alpha à 255 sans vérifier chaque pixel ; `LiveImage` est une source d'images
  renouvelées (comme `ImageSlot`, qu'elle contient) dont Direct2D garde le bitmap et le
  met à jour en place (`CopyFromMemory`) quand la taille ne change pas. Ailleurs (Metal,
  Vulkan, Cairo) elle se comporte comme un `ImageSlot`.

## Contrôles faits

Tests sans fenêtre (`HeadlessApp` et runtime) : arbre (pli, sélection Ctrl et Maj par
clé, tri, clavier, double-clic, glisser-déposer et ses refus, défilement jusqu'au
curseur, au plus 60 lignes construites en défilant 87 000 lignes) ; champs (pas,
bornes, vitesses, un seul `Commit`, Échap, saisie et expressions, vecteurs) ; grille
(plier à la souris et au clavier, contrôles, alignement, bulle) ; barre de menus (clic,
F10, Alt+lettre, flèches, entrée grisée, sous-menu) ; rang du clic ; molette ; fermeture
refusée ; `autofocus` ; `reveal`. Tests natifs sur fenêtre cachée : `WM_CLOSE` refusé ;
bitmap d'une `LiveImage` repris et pixels identiques à ceux d'une image neuve.
Contrôles négatifs faits pour la fermeture, `reveal` et la reprise du bitmap.

**Non vérifié** : rien n'a été fait à la souris ni au clavier dans une vraie fenêtre (la
consigne interdit de simuler clics et frappes) ; les tests passent par les mêmes entrées
du runtime que la fenêtre Windows. Lecteur d'écran des composants neufs (KUI-053) non
fait.

## Champs à onglet, à virgule, en chasse fixe (0.3.25)

Trois réglages ajoutés à `NumberField` et `VectorField`, pour l'inspecteur de
karbon-engine (maquette de son propriétaire) ; un champ qui ne s'en sert pas est peint
exactement comme avant :

```rust
VectorField::new(1, position, &self.fields, Msg::Field)
    .font(Font::new("Cascadia Mono", 500)) // la valeur dans cette police
    .decimal_comma(true)                   // « 1,50 » ; la saisie prend le point et la virgule
    .tagged(true)                          // la lettre de l'axe sur un onglet teinté
    .element()
```

- `font` : la valeur est dessinée par `DrawCommand::FontText` dans la police donnée (une
  police à chasse fixe aligne les chiffres) ;
- `decimal_comma` : le séparateur décimal montré est une virgule ; la zone de saisie
  ouverte par un clic garde le texte au point, et les deux sont acceptés ;
- `tagged` : la lettre X, Y ou Z sur un onglet de 18 pixels teinté de la couleur de
  l'axe, séparé de la valeur par un filet de cette couleur, au lieu du trait de 3 pixels ;
  coins de 6, filet de bord de la palette.

Le glisser, la saisie et Échap ne changent pas. Code : `src/ui/editor/field/view/tagged.rs`
(peinture et test) ; dans `field/view.rs`, des lignes ajoutées seulement (les trois
réglages et l'appel du peintre). Ce qui manque encore à karbon-ui pour l'éditeur de
karbon-engine (couleurs de la barre de titre, séparateurs réglables, bouton à icône, taille
du texte des listes, icônes dans les menus, police des autres composants) reste à faire :
l'éditeur peint lui-même ses boutons et ses textes en attendant.
