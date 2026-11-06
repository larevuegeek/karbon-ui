# TextInput : premier champ de saisie

Le champ est dessiné par le moteur, avec la pile de texte du backend (DirectWrite
sous Windows, Core Text sous macOS) pour la géométrie du texte. Il fonctionne sur
une ligne ou, avec `.multiline()`, sur plusieurs (voir
[Champ multiligne](#champ-multiligne)), et garde la valeur métier dans
l’application. Aucun navigateur et aucune crate externe ne sont utilisés.

## Construire et personnaliser le champ

Dans `Application::view`, avec le type `Msg` de l’application :

```rust
use karbon_ui::ui::{element::Element, widgets::{TextInput, InputStyle}};

let field = Element::custom(
    92,
    TextInput::new(self.name.clone(), Msg::NameChanged)
        .label("Nom de la playlist")
        .placeholder("Ex. Ma sélection du soir")
        .max_chars(256)
        .on_submit(Msg::SubmitName)
        .style(InputStyle {
            font_size: 16.,
            padding: 12.,
            radius: 10.,
            ..InputStyle::default()
        }),
);
```

Le champ remplit par défaut la largeur disponible. Les méthodes de layout de
`Element` restent utilisables : largeur, minima, maxima et insertion dans une
ligne, colonne ou grille. La hauteur naturelle dépend de la police et du padding.

`InputStyle` expose la taille de police, le padding intérieur sur les quatre
côtés, le rayon, le fond, la couleur du texte et les bordures normale/focalisée.
Une couleur `None` utilise le thème. `label` est le nom sémantique du champ ;
pour afficher aussi un label au-dessus, ajouter un widget `Text`.

`.read_only()` permet le focus, la sélection et la copie sans modification.
`.disabled()` retire le champ du parcours Tab et empêche l’interaction.
`max_chars` compte des valeurs scalaires Unicode (`char` Rust), pas des octets
ni des graphèmes. La valeur par défaut est 4 096, avec une limite de 65 536.

## Recevoir et conserver la valeur

Le second argument de `TextInput::new` est une fonction `Fn(String) -> Msg` :
chaque modification envoie le texte complet. Avec une variante
`NameChanged(String)`, la traiter dans `update` :

```rust
fn update(&mut self, message: Msg) -> bool {
    match message {
        Msg::NameChanged(name) => self.name = name,
        Msg::SubmitName => self.save_name(),
    }
    true
}
```

`true` demande une reconstruction de la vue. Le champ
retrouve son état sous son ID stable. Si la nouvelle valeur applicative est
identique au texte déjà édité, le curseur et la sélection sont conservés.
Une valeur différente fournie par l’application remplace le contenu et remet
l’historique à zéro. Il faut donc stocker les changements reçus, sinon une
reconstruction ultérieure peut rétablir la valeur initiale.

Entrée envoie à `Application::update` une copie du message donné à `on_submit`. La démo
présente un champ de nom de playlist, un bouton Valider et un bouton Effacer.
La composition IME provisoire n’émet pas de message `on_change` : seule sa
validation modifie la valeur applicative.

## Interactions disponibles

| Interaction | Effet |
| --- | --- |
| Clic et glissement | Positionner le curseur (la sélection se replie) puis sélectionner. |
| Double clic | Sélectionner le mot sous le pointeur, sans les espaces (ou la suite d'espaces visée). |
| Triple clic | Sélectionner la ligne entière (tout le texte, ou la ligne logique d'un champ multiligne). |
| Maj+clic, Maj+flèches | Étendre la sélection. |
| Gauche / Droite | Déplacer le curseur parmi les limites de clusters du texte natif. |
| Home / End | Début / fin du champ. |
| Retour arrière / Suppr | Supprimer la sélection ou un cluster voisin. |
| Ctrl+Gauche / Droite, avec Maj en option | Déplacer ou étendre par groupes séparés par des espaces. |
| Ctrl+A / C / X / V | Tout sélectionner, copier, couper, coller. |
| Ctrl+Z / Y, Ctrl+Maj+Z | Annuler / rétablir. |
| Entrée | Déclencher `on_submit`, sans répétition automatique. |
| Tab / Maj+Tab | Quitter le champ et déplacer le focus. |
| Échap | Annuler une composition ou réduire la sélection ; la démo reste ouverte. |
| Clic droit, Maj+F10, touche Menu | Menu contextuel intégré : Couper, Copier, Coller, Tout sélectionner. |

Le menu contextuel est dessiné et piloté par le moteur (`text_input/menu.rs`), comme la
liste du `Select`, donc identique sur toutes les plateformes : posé sous le pointeur
(ou sous le caret depuis le clavier) dans la fenêtre, entrées grisées selon l'état
(Couper et Coller en lecture seule, Couper et Copier sans sélection ou dans un champ
masqué), flèches et Entrée, Échap, clic extérieur qui le ferme sans atteindre ce qu'il y
a dessous. Choisir une entrée envoie la commande d'édition correspondante au champ, qui
passe par le presse-papiers du backend comme les raccourcis. Les libellés sont en anglais
par défaut ; `TextInput::menu_labels(["Couper", "Copier", "Coller", "Tout sélectionner"])`
les traduit. Un gestionnaire `context_menu` posé sur le champ ou l'un de ses parents a
priorité : l'application ouvre alors son propre menu.

Le défilement horizontal garde le curseur dans la zone intérieure. Les retours
à la ligne et tabulations collés sont remplacés par des espaces ; les autres
caractères de contrôle sont ignorés. La coupe ne supprime la sélection qu’après
la réussite de l’écriture dans le presse-papiers Windows.

## Champ multiligne

`TextInput::new(valeur, Msg::Notes).multiline()`, le constructeur
`textarea(id, valeur, Msg::Notes)` ou la balise `<textarea value={…} oninput={…}>`
donnent un champ de plusieurs lignes. `.rows(n)` (attribut `rows`, 2 par défaut)
fixe sa hauteur naturelle en lignes ; CSS `height` la remplace. Les styles, la
lecture seule, la désactivation, `max_chars` (un retour à la ligne compte pour un
caractère), le presse-papiers, l’annulation et la composition IME sont ceux du
champ monoligne.

| Interaction | Effet |
| --- | --- |
| Entrée | Insère un retour à la ligne (`on_submit` n’est pas émis). |
| Haut / Bas, Page préc. / Page suiv. | Ligne visuelle précédente ou suivante, ou d’une hauteur de champ, en gardant la colonne visée ; au-delà de la première ou de la dernière ligne, début ou fin du texte. |
| Home / End | Début / fin de la ligne visuelle ; en fin de ligne coupée automatiquement, le caret reste affiché en fin de ligne. |
| Clic, glissement, Maj+clic | Sur n’importe quelle ligne ; glisser au-delà du bord fait défiler. |
| Molette | Fait défiler le texte du champ survolé, puis le conteneur englobant une fois le bord atteint. |
| Tab / Maj+Tab | Navigation au focus, comme le champ monoligne. |

Le texte se coupe à la largeur intérieure du champ, après les espaces (les espaces
de coupure restent en fin de ligne) ; un mot trop long est coupé entre deux
clusters. `TextMeasurer::layout_wrapped_text` fournit les lignes : leurs plages
d’octets et, pour chacune, les carets et clusters de `layout_formatted_line`
appliqué à la ligne seule. Le champ dessine ensuite chaque ligne visible avec
cette même mise en page : carets, sélection et glyphes coïncident. Les coupures
sont natives sous macOS (`CTTypesetter`, la mise en page du dessin) et Windows
(`GetLineMetrics` de DirectWrite), et suivent les métriques fixes en headless ;
l’implémentation par défaut du trait coupe gloutonnement à partir de
`layout_formatted_line`. Le contenu défile verticalement ; le caret reste visible
après chaque frappe ou déplacement, mais la molette peut l’écarter sans le
déplacer. Un indicateur fin, sans interaction, montre la position du défilement.

Le texte collé ou fourni par l’application garde ses retours à la ligne (CR LF, CR,
U+2028 et U+2029 deviennent LF) ; les tabulations deviennent des espaces et les
autres caractères de contrôle sont ignorés. `AccessibleText` signale le champ
(`multiline`) et décrit ses lignes visuelles (`lines`) ; ses carets sont donnés
ligne par ligne, une position de coupure automatique apparaissant en fin d’une
ligne et au début de la suivante.

## Répartition du code

| Fichier | Responsabilité |
| --- | --- |
| `src/ui/components/text_input/mod.rs` | Configuration, édition, sélection, historique, composition et peinture. |
| `src/ui/components/text_input/wrap.rs`, `rows.rs` | Lignes coupées (`WrappedLine`, coupure par défaut) et géométrie des lignes visuelles. |
| `src/ui/components/text_input/multiline.rs`, `multiline_paint.rs` | Champ multiligne : synchronisation, défilement, touches, peinture. |
| `src/ui/core/element/mod.rs` | Extension du contrat `Widget` et service de géométrie du texte. |
| `src/ui/runtime/mod.rs` | Focus, capture, routage, notifications applicatives et snapshots. |
| `src/platform/windows/graphics/typography/line.rs` | Mesures de clusters et positions DirectWrite. |
| `src/platform/windows/events/text.rs`, `events/clipboard.rs` | UTF-16, presse-papiers, caret système, timer et IMM. |
| `src/platform/windows/events/keyboard.rs`, `events/text.rs` | Conversion des messages clavier et IME. |

Le moteur conserve des indices d’octets UTF-8 sur des limites valides. DirectWrite
fournit les clusters et leur géométrie ; le backend traduit ses positions UTF-16
vers ces indices UTF-8. Le curseur ne coupe donc pas un octet multibyte ou une
paire de surrogates. Le test natif vérifie aussi le cas `e` + accent combiné.
Les rectangles de sélection sont calculés à partir des clusters, avec plusieurs
rectangles possibles pour du texte bidirectionnel. Références :
[GetClusterMetrics](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-getclustermetrics) et
[HitTestTextPosition](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-hittesttextposition).

La composition utilise les messages IMM `WM_IME_STARTCOMPOSITION`,
`WM_IME_COMPOSITION` et `WM_IME_ENDCOMPOSITION`. Le texte provisoire est souligné,
le résultat est validé une fois et les fenêtres de candidats sont positionnées
près du caret en coordonnées physiques. Références :
[WM_IME_COMPOSITION](https://learn.microsoft.com/en-us/windows/win32/intl/wm-ime-composition) et
[valeurs de composition](https://learn.microsoft.com/en-us/windows/win32/intl/ime-composition-string-values).

## Ressources et limites

La géométrie du texte est réutilisée tant que le contenu et la police ne changent
pas. Un déplacement de curseur, une sélection ou un clignotement ne remesure pas
le texte. Le timer suit la période de clignotement du système et s’arrête quand
le champ n’est plus visible ou que la fenêtre perd le focus. L’historique est
borné à 100 états et à environ 1 Mio par pile. Retirer un champ retire son état.
Les caractères tapés ne sont pas encore regroupés automatiquement en une seule
opération d’annulation.

Cette version ne fournit pas de double-clic de sélection de mot, ni d’autoscroll
continu pendant une sélection immobile hors du champ. Le champ multiligne n’a pas
de barre de défilement interactive, ni de raccourcis début/fin de document
(Ctrl+Début, ⌘↑ : les touches ne portent pas ce modificateur jusqu’au moteur) ;
une ligne ne peut pas être plus large que le champ (pas de défilement horizontal). Les champs dans des lignes virtualisées sont explicitement refusés
pour éviter de perdre une édition lors du recyclage. La navigation gauche/droite
est logique ; l’affinité et la navigation visuelle complètes aux frontières bidi
restent à approfondir. Les couleurs d’emoji dépendent du rendu texte actuel.

Le rôle commun `TextInput`, son label et les snapshots de valeur/sélection/caret
sont disponibles. Le [premier fournisseur Windows UI Automation](accessibility.md)
expose désormais le champ et sa valeur via `ValuePattern`, ainsi que ses plages,
sa sélection et sa géométrie via `TextPattern`. `TextPattern2` et les
essais avec lecteurs d’écran restent à compléter ; les exigences d’accessibilité
du cahier des charges restent donc partiellement ouvertes.

## Vérifications

La suite complète comprend 144 tests. Les tests du champ couvrent édition Unicode,
focus, lecture seule, désactivation, clipboard différé, annulation/rétablissement,
limite de longueur, validation, défilement horizontal et composition simulée.
Le test natif contrôle les positions de clusters DirectWrite sur texte vide,
accents, emoji, accent combiné et texte mixte hébreu/arabe.

Vérifié à l’écran : saisie de `Café été 😃`, Ctrl+A/C, collage à la fin, Ctrl+Z,
validation par Entrée, conservation lors du passage GPU vers WARP et insertion
au début en mode logiciel. L’IME est raccordé et testé dans le cœur par simulation ;
**la recette réelle avec un IME japonais/chinois n’a pas été effectuée**. La
reconversion, les attributs détaillés de composition, TSF et les variantes de
fenêtres de candidats nécessitent encore des essais natifs dédiés.

```sh
cargo test --offline --lib
cargo test --offline text_input
cargo test --offline input_line
```



