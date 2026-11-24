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

## Sélecteur de couleur (`ColorPicker`, 0.3.26)

Pour la carte Matière de l'éditeur de karbon-engine (couleur de base, couleur d'une
lumière), et pour toute couleur d'un inspecteur :

```rust
ColorPicker::new(1, rgb, &self.colors, Msg::Color)
    .alpha(opacite)      // une barre d'opacité, si on la veut
    .height(110.)        // hauteur du carré (120 par défaut)
    .palette(PALETTE)
    .element()
// Dans update :
let reply = self.colors.update(m);
if let Some(e) = reply.event { /* e.key, e.rgb, e.alpha, e.change */ }
// Dans Application::key : self.colors.key(press) (Échap pendant un glisser).
```

- Un carré (saturation en travers, valeur en hauteur) sur la teinte choisie, une barre
  de teinte, une barre d'opacité au besoin (sur un damier), puis une pastille de la
  couleur et sa valeur hexadécimale à taper (`#RRGGBB` ou `#RGB`, avec ou sans `#`).
- Un appui dans le carré ou une barre prend la couleur à cet endroit, le glisser la
  change : des `FieldChange::Preview` pendant le geste, **un seul** `FieldChange::Commit`
  au relâchement (pour l'historique), Échap rend la couleur d'avant
  (`FieldChange::Cancel`). Le geste suit le pointeur hors du carré (bornes tenues).
- La valeur tapée compte à Entrée, Tab ou quand la zone perd le clavier ; une valeur
  illisible ne change rien ; Échap l'abandonne.
- Un gris ou un noir n'a pas de teinte : l'état retient la teinte et la saturation du
  dernier geste, tant que la couleur reçue est bien celle qu'elles donnent ; la barre de
  teinte ne saute pas au rouge quand on désature.
- Rouge, vert, bleu de 0 à 1, dans l'espace que choisit l'application (sRGB pour une
  couleur montrée telle quelle ; l'éditeur de karbon-engine convertit en lumière
  linéaire). Conversions publiques : `Hsv::from_rgb`, `Hsv::to_rgb`, `hex_of`,
  `parse_hex`.
- La pipette (prendre une couleur à l'écran) n'est pas faite.

Code : `src/ui/editor/color/` (`convert.rs`, `state.rs`, `view.rs`, `tests.rs`), tout neuf ;
dans `src/ui/editor/mod.rs`, des lignes ajoutées seulement. Aucun composant existant n'est
changé. Dans l'exemple `editor_components`, une section « Couleur » de la grille de
propriétés montre le sélecteur (lignes ajoutées). Tests sans fenêtre : aller-retour des
couleurs et de l'hexadécimal, glisser dans le carré (un seul `Commit`), teinte gardée d'un
gris, Échap, barre d'opacité, valeur tapée (contrôle négatif fait sur la teinte gardée).
Le pointeur arrive par `Element::on_pointer`, donc **sous Windows seulement** pour l'instant.

## Frise de temps (`Sequencer`, 0.3.28)

Pour la page Animation de l'éditeur de karbon-engine (clés d'un objet, animation d'un
personnage), et pour toute frise : une règle du temps, des pistes, leurs clés en losanges,
une tête de lecture. Nommée `Sequencer` pour ne pas la confondre avec
`ui::animation::timeline`, qui anime l'interface elle-même.

```rust
let pistes = vec![SequencerTrack { name: "Position".into(), keys: vec![0.0, 1.0], ..Default::default() }];
Sequencer::new(1, pistes, temps, &self.frise, Msg::Frise)
    .length(2.0)          // la fin de l'animation : au-delà, ombré
    .snap(1.0 / 30.0)     // aimant (Alt : libre)
    .decimal_comma(true)  // « 0,5 s » sur la règle
    .palette(PALETTE)
    .element()
// Dans update :
let reply = self.frise.update(m);
match reply.event {
    Some(SequencerEvent::Seek(t)) => { /* la tête de lecture */ }
    Some(SequencerEvent::Move { keys, by, change }) => { /* Preview, un Commit, Cancel */ }
    Some(SequencerEvent::Add { track, time }) => { /* double clic : une clé */ }
    Some(SequencerEvent::Menu { track, time, at }) => { /* clic droit */ }
    _ => {}
}
// Dans Application::key : self.frise.key(press) (Échap pendant un glisser de clés).
```

- Sur la règle : appui et glisser déplacent la tête de lecture (`Seek`), aimantée.
- Sur une clé : appui la choisit (Maj : l'ajoute au choix ou l'en retire) ; glisser
  déplace les clés choisies, aimanté (Alt libre) : des `FieldChange::Preview`, **un seul**
  `FieldChange::Commit` au relâchement, Échap : `Cancel` (de 0).
- Ailleurs dans les pistes : un clic met la tête de lecture et vide le choix, un glisser
  choisit les clés d'un rectangle (Maj : en plus) ; un double clic demande une clé
  (`Add`) ; le clic droit un menu (`Menu`) ; le bouton du milieu glisse la vue.
- La molette fait défiler les pistes, Maj et la molette le temps, Ctrl et la molette
  zoome autour du pointeur (l'instant sous lui ne bouge pas).
- L'état (`SequencerState`) garde la vue (`start`, `zoom`, `set_view`, `frame`) et le
  choix (`selection`, `set_selection` : à refaire quand l'application change les rangs).
- Seules les pistes et les clés visibles sont peintes (recherche dichotomique dans des
  clés rangées par instant) : 200 pistes et 50 000 clés ne coûtent que ce qui se voit.

Code : `src/ui/editor/sequencer/` (`state.rs`, `view.rs`, `tests.rs`), tout neuf ; dans
`src/ui/editor/mod.rs`, des lignes ajoutées seulement (et une phrase de sa documentation).
Aucun composant existant n'est changé, la version reste 0.3.28. Tests sans fenêtre : tête
de lecture aimantée, glisser de clés (un seul `Commit`, Échap), rectangle de choix et Maj,
double clic et clic droit, zoom autour du pointeur, peinture des seules clés visibles
(contrôle négatif fait : sans la recherche, l'essai échoue). Le pointeur arrive par
`Element::on_pointer`, donc **sous Windows seulement** pour l'instant.

## Ajouts de la vague « terminer l'outil » (version inchangée, 0.3.28)

Tout par ajout ; aucun composant existant ne change d'aspect ni de comportement.

### Bulle d'une entrée de menu

`MenuEntry::tip(texte)` : une bulle au survol, aux couleurs de la palette, dans la barre
de menus comme dans un menu contextuel (même grisée). Sans elle, rien ne change ; un
texte vide n'en fait pas. Code : `menu_bar/model.rs`, `view.rs`, `context.rs`. Le menu
contextuel passe par une fonction interne neuve, `anchored_menu` (le même menu, contre un
point ou contre un élément), qui sert aussi au débordement de la barre d'outils.

### Barre d'outils (`Toolbar`, KUI-033)

```rust
let items = vec![
    ToolItem::choice("Déplacer", tool == Move, Msg::Move).icon(arrows).shortcut("G"),
    ToolItem::separator(),
    ToolItem::toggle("Grille", grid, Msg::Grid).text("#").shortcut("Ctrl+G"),
];
let (bar, overlays) = Toolbar::new("tools", &self.bar, Msg::Bar, items).build(largeur);
```

- Boutons, bascules (`toggle`), choix exclusifs (`choice`), séparateurs ; icône
  (`VectorIcon`) ou court texte ; bulle « Nom (raccourci) » et sa description ; grisé.
- Ce qui ne tient pas dans la largeur donnée part, **dans l'ordre**, dans un menu ouvert
  par « » » au bout de la barre (cases et points des bascules et des choix, raccourcis).
  Les largeurs sont connues sans mise en page : un bouton à icône est carré, un bouton à
  texte a la largeur de son texte, ou celle que donne `width`.
- Au clavier : chaque bouton prend le focus (Tab), Entrée l'active.

### Toile 2D et tracés épais (`editor::canvas`, KUI-036 et KUI-037)

- `CanvasView` : coin du monde en haut à gauche, pixels par unité en x et en y ;
  `to_screen`, `to_world`, `zoom_at` (autour du pointeur, de 0,01 à 100 000 pixels par
  unité), `pan`, `frame` (cadrer un rectangle avec une marge).
- `bezier_points` (une courbe de Bézier en segments d'au plus `step`) et `stroke` (une
  ligne brisée d'épaisseur donnée, en tirets au besoin), faits de `DrawCommand::Line` :
  aucune commande de dessin neuve. 
- Pas de composant « toile » générique : l'éditeur de courbes s'en sert ; le graphe de
  nœuds (KUI-040) n'est pas fait.

### Éditeur de courbes (`CurveEditor`, KUI-038)

```rust
CurveEditor::new(1, &self.curves, &self.state, Msg::Curves).palette(PALETTE).element()
// update : let reply = self.state.update(m, &self.curves); appliquer reply.event
// key    : self.state.key(press, &self.curves)
```

- Données de l'application : `Curve { name, color, keys }`, `CurveKey { time, value,
  in_slope, out_slope, mode }` (`Smooth` d'Hermite, `Linear`, `Constant`),
  `Curve::value_at`.
- Gestes (`CurveEvent`) : clés glissées (`Move` : des `Preview`, **un** `Commit`, Échap
  `Cancel`), pentes tirées par leurs poignées (`Slope`), double clic sur une courbe
  (`Add`, à sa valeur), Suppr (`Delete`), flèches (dix pixels, un `Commit` chacune),
  F ou Origine (tout cadrer). Choix : clic, Maj+clic, rectangle. Bouton du milieu :
  glisser la vue ; molette : zoom autour du pointeur (Ctrl : le temps seul, Maj : les
  valeurs seules).
- Grille aux pas ronds avec ses nombres ; la courbe est tracée sur la largeur visible
  (un point tous les deux pixels).

### Thème d'éditeur (KUI-049, KUI-050)

- `EditorPalette::kit_theme()` : le thème du kit dans les teintes de l'éditeur (fonds,
  textes, accent de `EditorPalette::DARK` et `LIGHT`, rayons de 4) ; le thème du kit par
  défaut ne change pas. Test : texte et texte discret à 4,5:1 au moins sur les fonds.
- `EditorPalette::with_json(texte)` : une palette lue à l'exécution (couleurs
  `"#RRGGBB"` sous leurs noms, `row_height` de 16 à 48, `font_size` de 9 à 24) ; ce qui
  ne se lit pas est refusé et dit.

### Contrôles

Tests sans fenêtre : bulle d'une entrée (témoin : sans `tip`, aucune ; contrôle négatif :
sans la ligne de la vue, l'essai échoue), barre d'outils (ce qui tient, ordre du
débordement, menu, bulles, clavier), toile (aller-retour, zoom de 1 % à 1 000 % autour du
pointeur, cadrage, Bézier et tirets), courbes (valeurs, glisser et Échap, double clic,
rectangle et Maj, Suppr et flèches, pente, molette et cadrage), thème. Le pointeur des
courbes arrive par `on_pointer` : sous Windows seulement pour l'instant.

## Toile de graphe de nœuds (`GraphEditor`, KUI-040, version inchangée)

```rust
GraphEditor::new(1, &self.graph, &self.state, Msg::Graph).palette(PALETTE).element()
// update : let reply = self.state.update(m, &self.graph); appliquer reply.event
// key    : self.state.key(press, &self.graph)   (par Application::key ou on_key)
```

- Rangée sous `ui::editor` comme les courbes et la frise (le cahier proposait
  `ui::graph` ; la toile partage `editor::canvas` et `EditorPalette`).
- Données de l'application (`Graph`) : `GraphNode { id, title, subtitle, position,
  width, color, mark, inputs, outputs }`, `GraphPort { name, kind, color }`,
  `GraphWire { from, to }` (une sortie vers une entrée, `PortRef { node, port }`),
  `GraphComment { id, rect, text, color }`. Règles de branchement : même sorte, sorte
  `0` (tout), paires `Graph::compatible` ; une entrée prend un fil (le neuf remplace
  l'ancien) sauf `many_per_input`. Le sens des nœuds reste à l'application.
- Gestes (`GraphEvent`) : nœuds glissés (`Move` : des `Preview`, un `Commit`, Échap
  `Cancel`) ; un fil tiré d'une sortie vers une entrée ou l'inverse (`Connect`, avec les
  fils à retirer) ; un fil repris de son entrée, posé ailleurs (`Connect`) ou lâché dans
  le vide (`Disconnect`) ; Alt+clic sur un fil ou un port (`Disconnect`) ; clic sur un
  fil (`ChooseWire`, Suppr le retire) ; Suppr (`Delete`), Ctrl+A, Ctrl+C (`Copy`),
  Ctrl+V (`Paste`), Ctrl+D (`Duplicate`), C (`Comment` autour du choix) ; le titre d'un
  commentaire emporte ses nœuds ; clic droit (`Menu`, pour la recherche des nœuds à
  créer) ; double clic (`Open`). Choix : clic, Maj+clic, rectangle. Bouton du milieu :
  glisser la vue ; molette : zoom autour du pointeur ; F : cadrer ; mini-carte en bas à
  droite (clic, glisser) dès 360 × 260 pixels.
- Peinture, du fond vers le dessus : grille, commentaires, fils, nœuds, fil tiré,
  rectangle de choix, mini-carte. Ce qui sort de la toile n'est pas peint ; les textes
  disparaissent sous un zoom de 0,45.
- Contrôles (tests sans fenêtre, visés par la place des ports et des nœuds dans la
  fenêtre simulée) : brancher (témoins : autre sorte refusée, propre nœud refusé),
  reprendre et débrancher, annuler par l'historique de l'application, glisser en une
  étape et Échap, rectangle, Maj, Suppr avec les fils, copier, coller, dupliquer,
  commentaire qui emporte ses nœuds, clic droit, double clic, fil choisi et retiré,
  mini-carte, ordre de peinture (témoin : fils peints après les nœuds, l'essai échoue).
  500 nœuds et 1 000 fils tout montrés : 4,9 ms par image en mise au point (construction
  de la vue et peinture de la scène, processeur).

## Noms et valeurs lus par le Narrateur (ajout, version inchangée)

- `Element::aria_label(nom)` : le nom accessible que lit le Narrateur à la place de
  celui du composant (« Text field », « Choice ») ; rangé avec les gestionnaires, rien
  n'est ajouté à un élément qui n'en a pas.
- `Widget::accessible_value()` : la valeur lue d'un composant qui n'est pas une zone de
  texte (`None` par défaut). Les champs numériques (`NumberField`, `VectorField`) sont
  désormais exposés en curseurs, avec leur valeur montrée (« 2,5 m », « — ») ;
  `NumberField::aria_label` et `VectorField::aria_label` (« Position » donne
  « Position X », « Position Y », « Position Z »).
- Essai : `editor/field/view/labels.rs` (témoin : un champ sans nom est lu sans nom).
