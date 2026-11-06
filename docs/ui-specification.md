# Cahier des charges du socle UI Rust

Version 0.26 du 3 novembre 2026 — styles hérités, layout CSS étendu et effets natifs.

Créer une bibliothèque d’interface de bureau en Rust qui permette de composer,
styliser et étendre des interfaces comme RustMusic, sans navigateur intégré et
sans crate externe. RustMusic sert de première application de validation ; les
composants génériques doivent pouvoir servir à d’autres applications.

Cette étape définit les besoins. Elle ne lance pas la réécriture de RustMusic
et ne constitue pas une implémentation des fonctions décrites ci-dessous.

## 1 Objectifs et contraintes retenus

| Sujet | Exigence |
| --- | --- |
| Usage | Socle général, validé progressivement par les écrans de RustMusic. |
| Identité du projet | Nom Cargo : `karbon-ui`. Développement dans ce dépôt local. |
| Intégration Karbon | Bibliothèque utilisable seule ; raccordement facultatif aux services de `karbon-framework` et, plus tard, au CLI. Le cœur UI ne dépend pas du framework web. |
| Langage | API applicative en Rust ; noms des fonctions, types et propriétés en anglais. Documentation pédagogique en français. |
| Dépendances | Aucune crate externe. Appels directs aux services et bibliothèques natives des plateformes. La bibliothèque standard Rust reste utilisable. |
| Interface | Rendu personnalisé, sans HTML, CSS exécuté dans un navigateur, JavaScript ou WebView. |
| Overlays de bureau | Fenêtre sans bordure avec fond entièrement transparent, contenu visible et politique de clics configurable. |
| Construction | Composer des éléments courants avec peu de code ; spécialiser leur apparence et leur contenu. |
| Écriture des styles | Conserver les recettes Rust et permettre des fichiers CSS compilés ; même moteur natif. Étendre la couverture des propriétés par familles avec validation réelle du rendu, sans accepter silencieusement une propriété ignorée. |
| Extension | Créer un nouvel élément sans modifier le cœur du moteur ni écrire du Win32 dans le code de l’application. |
| Rendu | Mode accéléré GPU et mode logiciel CPU, avec les mêmes composants et événements. |
| Plateformes | Windows d’abord ; architecture prévue pour macOS et Linux, avec validation distincte sur chaque système. |
| Identité visuelle | Même composition, composants, thème et ressources sur chaque OS ; adaptations natives explicites pour les services système. |
| Performance | Viser une consommation RAM/CPU et une latence nettement inférieures à une interface Tauri comparable ; établir les résultats par mesure avant d’annoncer un gain. |
| Qualité | Inclure le clavier, la saisie internationale, l’accessibilité et la gestion de longues collections dans la conception. |

« Sans crate externe » ne signifie pas réécrire les polices ou les pilotes GPU.
Les services natifs restent utilisables par FFI, comme Win32 aujourd’hui.
Chaque backend devra documenter ses bibliothèques liées, ses prérequis de
compilation et ses dépendances à l’exécution. La pile Linux de texte et de
fenêtrage reste à définir avant son implémentation.

### 1.1 Direction validée : Karbon UI

Le produit cible est un framework d’interface native avec gestion des fenêtres
de l’application. Il doit permettre de créer aussi bien un petit outil autonome
qu’une application comme RustMusic, sans imposer un serveur HTTP ou les
dépendances web de Karbon. L’absence de crates externes concerne le cœur UI ;
une application reste libre d’utiliser d’autres bibliothèques pour ses services.

La même vue Rust et les mêmes composants doivent fonctionner sur les backends
pris en charge, sans conditions Win32/Cocoa/Linux dans les pages applicatives.
Les API publiques emploient des unités logiques, des identifiants et des
événements communs. Une capacité absente doit être signalée ou avoir une
alternative documentée ; un backend non implémenté n’est pas un OS supporté.

L’identité visuelle appartient à la bibliothèque et à l’application : composants
dessinés par le moteur, espacements, formes, icônes et palette partagés. Prévoir
un choix explicite de polices et de ressources communes. Les métriques et
l’anticrénelage natifs peuvent varier ; une identité cohérente ne signifie pas
une égalité de tous les pixels. Les raccourcis, la saisie, l’accessibilité et les
dialogues système respectent les conventions de chaque plateforme.

La simplicité de personnalisation se valide dans les exemples : composer une
page, appliquer un style commun, modifier un seul élément et créer un composant
spécialisé sans intervenir dans le backend. Les réglages usuels doivent être
accessibles par l’API publique ; les optimisations internes ne doivent pas
imposer de gérer des handles ou des durées de vie natives dans une page.

Les prochaines fondations, avant l’intégration à Karbon, sont :

1. Clarifier le contrat public application/fenêtre/événements en conservant la
   démo actuelle comme exemple de compatibilité.
2. Gérer plusieurs fenêtres avec identité, état, focus, ressources et fermeture
   indépendants, puis une politique explicite d’arrêt de l’application.
3. Fournir un canal de résultats et un réveil du thread UI pour les tâches de
   fond, avec annulation et rejet des réponses devenues obsolètes.
4. Valider ces contrats sur un écran RustMusic représentatif et mesurer les
   coûts avant de généraliser l’API ou de migrer le dépôt.

Le premier contrat multi-fenêtre est maintenant disponible sous Windows :
`WindowId`, `WindowDescriptor`, commandes différées et politique de sortie.
Voir le [guide des fenêtres](windows.md). Le premier service de [tâches de
fond](tasks.md) ajoute un pool borné, l’annulation et le rejet des réponses
obsolètes, avec livraison sur le thread UI. Seul le backend Windows est disponible.

## 2 Point de départ du projet

Les templates HTML `ui!` / `ui_file!` et recettes `styles!` produisent désormais
l'arbre natif. Un premier `css_file!` compile les classes, imports, variables,
dimensions, espacements, fonds, bordures et effets pris en charge. Les marges
et gaps par axe sont raccordés au layout. Le [guide CSS](css.md) distingue les
propriétés disponibles et les fonctions encore à implémenter. L’héritage
typographique, les états des contrôles, flex/grid, les positions, ombres/filtres,
unités calculées, media queries, variables typées et keyframes sont raccordés.
La couverture complète du CSS d’un navigateur reste hors du périmètre actuel.

Le prototype fournit une fenêtre Windows sans bordure, les déplacements,
le redimensionnement, trois boutons de fenêtre, des préférences DWM et une
ombre calculée en Rust. Le gestionnaire possède maintenant un état distinct
pour chaque fenêtre interactive, sur une boucle d’événements commune.

Le premier démonstrateur J1 ajoute une scène graphique commune, du texte
DirectWrite, une image PNG, des formes et chemins, du clipping, deux thèmes et
le rendu Direct2D sur GPU ou sur WARP logiciel. La présentation conserve l’alpha
jusqu’au bureau pour permettre un fond entièrement transparent. Le
[bilan J1](archive/rendering-j1.md) précise les validations et limites de cette version.

La géométrie de la barre de titre, le thème, les commandes de dessin et le
calcul d’ombre sont séparés des appels Windows. Il manque encore le moteur de
mise en page complet, les contrôles de saisie, le routage général des événements
entre composants, l’accessibilité native et les services généraux de ressources.

La première base J2 ajoute un contrat d’application indépendant du backend,
un arbre de composants conservé, des lignes et colonnes, une grille régulière
avec spans et positions explicites, des panneaux redimensionnables en option,
des boutons avec souris et clavier, ainsi que le focus. Le
[guide J2](layout-j2.md) présente l’API, ses validations et ses limites.

Une première base de [défilement](scroll.md) ajoute les viewports verticaux ou
horizontaux, les barres, la molette, le clavier et la conservation des positions.
La démonstration utilise maintenant une [liste virtualisée](virtual-list.md)
de 10 000 éléments à hauteur fixe : seules les lignes utiles sont construites.
La virtualisation à hauteurs variables, l’accessibilité native des éléments
virtuels et les autres exigences J4 restent à réaliser.

Le premier [TextInput](text-input.md) ajoute édition monoligne, sélection,
historique, presse-papiers, géométrie DirectWrite et raccordement IMM. Les tests
simulent la composition ; la recette native avec un IME asiatique reste ouverte.
Un [premier fournisseur UI Automation](accessibility.md) expose maintenant l’arbre,
le focus, ValuePattern, InvokePattern et TextPattern du champ monoligne. TextPattern2, les patterns avancés et les
essais avec lecteurs d’écran restent ouverts ; J2 et J3 ne sont pas considérés terminés.
Une première collection de [choix contrôlés par l’application](choices.md) est
disponible : Checkbox, Switch, RadioButton et RadioGroup, avec thème, souris,
clavier, état mixte et patterns accessibles Toggle/Selection.
Le [slider numérique](slider.md) ajoute les plages, les pas, l’aperçu et la
validation séparés, le clavier et RangeValuePattern sous Windows.
Le [Select](select.md) ajoute un choix non éditable, une liste défilante peinte
au-dessus des panneaux, le clavier et les patterns ExpandCollapse/Selection.
Son placement ancré constitue une première base aux menus généraux encore à réaliser.
Les [modales](modal.md) ajoutent une pile de dialogues dans la fenêtre, un fond
bloquant, le confinement et la restauration du focus, avec un contenu composé
des widgets existants. Les [popovers et menus contextuels](popover.md) ajoutent
les panneaux ancrés, le clic droit, Shift+F10, les actions et la navigation de
menu. Les menus en cascade, les animations et leur recette lecteur d’écran
restent ouverts.

Les [info-bulles](tooltip.md) ajoutent l’aide au survol et au focus clavier,
un délai configurable, un réveil ponctuel et HelpText/ToolTip sous Windows.
Leur recette d’annonces avec un lecteur d’écran reste à réaliser.

Les [styles réutilisables](styles.md) proposent maintenant des recettes partagées
pour panneaux, boutons et textes, avec des couleurs sémantiques résolues au dessin.
Les contrôles partagent désormais les propriétés CSS de typographie, d’apparence
et d’états. Les widgets personnalisés peuvent implémenter `Widget::apply_style`.
Les recettes Rust détaillées restent disponibles pour les sous-parties des contrôles.
Le [guide CSS](css.md) précise le contrat, les sélecteurs et les limites restantes.

Un ancien [prototype RustMusic](archive/rustmusic-demo.md) a permis de valider
navigation, recherche, favoris, pochettes, liste de 10 000 titres et lecteur
simulé dans un exécutable désormais retiré. Les illustrations passaient par
les tâches de fond ; `ImageCache<K>` borne les allocations de pixels retenues
par le cache CPU et évince selon la récence. Les références encore utilisées
par les scènes, les textures GPU et les résultats en transit sont hors de cette
borne. Le [chargement PNG/JPEG](image-loading.md) est maintenant disponible
sur worker avec limites de fichiers et de dimensions, miniature et erreurs
partielles. Un [premier relevé](archive/performance.md) mesure rendu hors écran,
relecture CPU et mémoire du processus. La composition jusqu’au moniteur et
la comparaison Tauri restent à mesurer ; J5 n’est pas entièrement validé.

Le [profilage de fenêtre](archive/presentation-profiling.md) ajoute les durées de
préparation, rendu/relecture, surface et soumission Windows en utilisation
normale, avec au maximum 1 024 échantillons et instantanés CPU/RAM du processus.
La recette native sur fenêtre cachée est validée ; les premières sessions
visibles sont consignées avec la file ci-dessous. La mesure du compositeur reste à réaliser.

Le chemin normal de peinture réutilise maintenant les pixels du DIB comme
destination de relecture, supprimant le buffer Vec et sa copie intermédiaire.
Les modes GPU/WARP et les pixels sont testés aux échelles 1/1,5/2. Le protocole
de profilage devient `window-submit-direct-dib-v2` ; la relecture CPU reste présente.

La [file de lecture](archive/playback-queue.md) valide maintenant l’ordre par clés
stables, sélection, doublons, retrait et liste virtualisée jusqu’à 10 000
occurrences. Le [drag/drop commun](drag-drop.md) transmet le dépôt à
l’application, avec seuil, capture, annulation, groupes et placement avant/après.
Des commandes hors liste offrent l’alternative clavier. La [recette et les
mesures](archive/queue-validation.md) couvrent GPU/WARP et distinguent les sessions
visibles des mesures répétables hors écran ; aucun gain Tauri n’est établi.

## 3 Lecture de la référence RustMusic

La capture fournie représente un écran d’accueil. Les comportements ne sont pas
tous déductibles d’une image : les menus ouverts, les modales, les erreurs et
les déplacements au clavier devront être décrits par des scénarios complémentaires.

| Zone visible | Éléments repérables | Besoins associés |
| --- | --- | --- |
| Barre supérieure | Logo, boutons précédent et suivant, recherche, raccourci affiché, icônes, thème, profil, commandes de fenêtre | Ligne flexible, champ composé, raccourcis, boutons à icône, menu ancré, intégration de la barre de titre. |
| Colonne gauche | Sources, import fichier et dossier, navigation, bibliothèques, playlists, profil et état du scan | Sections, navigation sélectionnée, listes défilantes, vignettes, menus, sélecteurs système, zones fixes et défilantes. |
| En-tête principal | Salutation, sous-titre, statistiques, action d’import | Hiérarchie typographique, alignement, adaptation à la largeur. |
| Carte du morceau en cours | Pochette, titre, métadonnées, progression, commandes et favoris | Carte composée, image arrondie, texte, dégradé, slider, bouton principal, bouton à bascule. |
| File suivante | Miniatures et titres tronqués dans un panneau | Liste compacte, troncature, sélection et navigation vers une vue détaillée. |
| Recommandations | Cartes illustrées avec textes superposés | Grille adaptative, découpage d’image, superpositions, dégradés et actions au survol. |
| Encadré de soutien | Message flottant, fermeture, action principale, report et préférence | Panneau non modal fermable ; sa persistance relève de l’application. |
| Lecteur inférieur | Pochette, commandes, temps, progression, favoris, file, périphérique et volume | Zone fixe, alignement, boutons, sliders, indicateurs et popovers. |
| Barre d’état | Format audio, chemin tronqué, DSP, sortie audio et réseau | Texte compact, badges, icônes, états et menus éventuels. |

Le menu de profil, le choix des sources et les popovers sont des comportements
à confirmer dans RustMusic. Les modales et formulaires font partie du besoin
général du socle, même s’ils ne sont pas ouverts dans cette capture.

## 4 Priorités

| Niveau | Signification |
| --- | --- |
| P0 | Fondation nécessaire aux composants ; à construire et tester progressivement. |
| P1 | Nécessaire à la première version utilisable pour RustMusic et pour une application de bureau courante. |
| P2 | Extension inventoriée, à développer sur un scénario concret après le premier socle. |
| Hors périmètre | Fonction applicative ou moteur spécialisé qui ne relève pas de cette bibliothèque. |

P0 ne signifie pas que toute la fondation doit être terminée avant une première
démonstration. Les jalons de la section 17 livrent des ensembles utilisables.

## 5 Catalogue des éléments

Chaque composant interactif devra définir ses propriétés, ses événements, son
comportement au clavier, ses états visuels et sa représentation accessible.

### 5.1 Primitives et conteneurs

| Élément | Besoin | Priorité |
| --- | --- | --- |
| `Text`, `RichText` | Texte simple puis portions de texte de styles différents, liens et sélection facultative. | Texte simple P0 ; texte enrichi P1 |
| `Icon` | Icônes vectorielles, dimensions, couleur et épaisseur contrôlables. Un bouton à icône possède un nom accessible. | P0 |
| `Image` | Images raster, ratio, ajustement au cadre, recadrage, arrondis, remplacement en chargement ou en erreur. | P0 |
| `Surface`, `Container`, `Card` | Fond, bordure, rayon, ombre, padding et contenu composé. | P0 |
| `Row`, `Column`, `Stack`, `Grid` | Composition horizontale, verticale, superposition et grille. | P0 |
| `Spacer`, `Divider` | Espace flexible, séparateur horizontal ou vertical. | P0 |
| `ScrollView`, `Scrollbar` | Défilement vertical et horizontal, molette, pavé tactile, clavier et contenu découpé. | P0 |
| `SplitPane` | Panneaux séparés par une poignée déplaçable, limites et restauration des dimensions. | P1 |
| `AspectRatio`, `Wrap` | Ratio réservé pour une pochette ; retour des éléments à la ligne. | P1 |

### 5.2 Actions et navigation

| Élément | Besoin | Priorité |
| --- | --- | --- |
| `Button`, `IconButton` | Texte, icône, contenu composé, tailles et variantes ; activation souris et clavier. | P1 |
| `ToggleButton`, `Switch` | Favori, répétition ou préférence ; distinguer valeur active et bouton momentanément pressé. | P1 |
| `Checkbox`, `RadioGroup` | Choix multiples, choix exclusif, état intermédiaire pour une sélection partielle. | P1 |
| `Link` | Action de navigation distincte d’une commande ; ouverture externe via un service de plateforme. | P1 |
| `NavigationItem`, `Tabs` | Page courante, changement au clavier, contenu associé, onglet désactivé. | P1 |
| `Menu`, `ContextMenu`, `MenuItem` | Groupes, séparateurs, raccourcis, coches, sous-menus, commandes désactivées et navigation clavier. | P1 |
| `Breadcrumb`, `Toolbar` | Chemin de navigation et groupes d’actions, débordement lorsque la largeur manque. | P2 |

### 5.3 Saisie et sélection de valeurs

| Élément | Besoin | Priorité |
| --- | --- | --- |
| `TextInput` | Champ simple, label, placeholder, valeur, lecture seule, sélection, validation et bouton d’effacement optionnel. | P1 |
| `SearchInput` | Champ de recherche composé avec icône, raccourci, effacement, état de recherche et suggestions optionnelles. | P1 |
| `TextArea` | Texte multiligne, retour à la ligne, défilement et limites configurables. | P1 |
| `NumberInput` | Valeur numérique, minimum, maximum, pas, format localisé et valeur temporairement incomplète. | P1 |
| `Select` | Liste de choix sans saisie libre ; valeur, ouverture, sélection clavier et fermeture. | P1 |
| `ComboBox` | Saisie et choix dans des suggestions ; valeur libre ou contrainte selon le contrat du composant. | P1 |
| `MultiSelect`, `TagInput` | Choix multiples, puces supprimables et recherche. | P2 |
| `Slider` | Volume et position : plage, pas, orientation, clavier, valeur en déplacement et valeur validée. | P1 |
| `RangeSlider`, `Knob` | Intervalle à deux poignées et potentiomètre ; cas d’usage à confirmer. | P2 |
| `PasswordInput` | Masquage, affichage explicite et comportement de copie défini. | P2 |
| `FilePicker`, `FolderPicker` | Ouvrir un sélecteur système ; filtres et annulation ; résultat transmis à l’application. | P1 |
| `KeyBindingInput` | Capturer un raccourci en distinguant la saisie des commandes de l’application. | P2 |
| `ColorPicker`, `DatePicker`, `TimePicker` | Choix spécialisés pour d’autres applications ; pas de développement sans scénario. | P2 |
| `Form`, `Field`, `ValidationMessage` | Label associé, aide, erreurs, validation synchrone ou asynchrone, soumission et annulation. | P1 |

### 5.4 Collections et présentation de données

| Élément | Besoin | Priorité |
| --- | --- | --- |
| `List`, `ListItem` | Lignes composées, sélection simple ou multiple, activation, menu contextuel. | P1 |
| `VirtualList` | Créer et dessiner les lignes utiles à la zone visible ; conserver identité, sélection et focus. | P1 |
| `Table` | Colonnes, tri, sélection, redimensionnement, entêtes et lignes virtualisées. | P1 |
| `VirtualGrid` | Grille de pochettes avec chargement progressif et dimensions adaptatives. | P1 |
| `TreeView` | Dossiers imbriqués, ouverture, fermeture, navigation clavier et chargement différé. | P2 |
| `Badge`, `Chip`, `Avatar`, `StatusIndicator` | Petites informations, filtres, profil et états ; sens fourni aussi autrement que par la couleur. | P1 |
| `ProgressBar`, `Spinner`, `Skeleton` | Progression déterminée, attente indéterminée et structure de remplacement. | P1 |
| `EmptyState`, `ErrorState` | Collection vide, aucune recherche trouvée, erreur et possibilité de réessayer. | P1 |
| `Pagination` | Découpage de résultats quand l’application le souhaite ; distinct du chargement virtuel. | P2 |

### 5.5 Éléments au-dessus du contenu

| Élément | Besoin | Priorité |
| --- | --- | --- |
| `Tooltip` | Aide au survol et au focus, délai configurable, position adaptée à l’espace disponible. | P1 |
| `Popover` | Contenu ancré à un élément, repositionnement, fermeture définie et navigation clavier. | P1 |
| `Dialog`, `Modal` | Titre, contenu, actions, fermeture, focus contenu dans la modale et retour au déclencheur. | P1 |
| `Drawer` | Panneau latéral, version modale ou non modale explicitement choisie. | P1 |
| `Toast`, `NotificationPanel`, `Banner` | Message temporaire ou persistant, action, fermeture et annonce accessible adaptée. | P1 |
| `Overlay` | Couche de composition ; découpage, opacité et interception des événements explicites. | P0 |
| `OverlayWindow` | Fenêtre de bureau à fond transparent, avec zones interactives ou transmission des clics aux applications derrière. | P1 |

L’encadré de soutien de la capture n’est pas à traiter automatiquement comme une
modale : le reste de l’interface semble rester visible et aucun voile bloquant
n’est montré. Son comportement doit être choisi explicitement.

## 6 Contrat commun des composants

### 6.1 Propriétés et états

Un composant expose des propriétés typées : contenu, identifiant stable,
dimensions ou contraintes, style, état activé, rôle accessible et commandes.
L’identité doit survivre à une mise à jour des données ou à un déplacement dans
une liste ; l’index de la ligne ne suffit pas comme identité persistante.

Les états utiles sont `normal`, `hovered`, `pressed`, `focused`,
`focus_visible`, `disabled`, `selected`, `checked`, `indeterminate`,
`expanded`, `loading`, `read_only` et `invalid`, selon le composant.
Plusieurs états peuvent coexister : un champ peut être à la fois focalisé et
invalide. Le style doit traiter ces combinaisons de façon déterministe.

Un composant désactivé ne déclenche pas son action ; la lecture seule interdit
la modification tout en permettant les usages prévus, comme sélectionner et
copier le texte. Un élément caché ne reçoit pas de clic. Masquer en conservant
l’espace et retirer de la mise en page sont deux opérations distinctes.

### 6.2 Événements et commandes

Les contrôles exposent des événements sémantiques, par exemple `on_activate`,
`on_change`, `on_submit`, `on_cancel`, `on_open` et `on_close`.
Ils doivent être déclenchables par une interaction clavier ou accessible,
sans simuler un clic à une coordonnée.

L’application reste propriétaire des données métier. Le socle gère les états
transitoires d’interaction ; une API claire synchronise les valeurs, la
sélection et le focus avec les données de l’application. Éviter deux valeurs
concurrentes pour un même champ et les boucles de notification.

Une commande peut être réutilisée par un bouton, un menu et un raccourci avec
le même état activé. Le socle ne connaît pas les commandes métier `PlayTrack`,
`ScanLibrary` ou `AddToPlaylist` : RustMusic les définit.

## 7 Mise en page et adaptation

| Référence | Exigence P0 |
| --- | --- |
| LAY-01 | Utiliser des unités logiques indépendantes des pixels physiques ; convertir au niveau du rendu et des services natifs. |
| LAY-02 | Gérer taille fixe, taille selon contenu, espace restant, pourcentage dans un parent défini, minimum et maximum. |
| LAY-03 | Prévoir marge, padding, gap, alignement, distribution et alignement sur la ligne de base du texte. |
| LAY-04 | Mesurer le texte et les enfants avant de répartir l’espace ; traiter explicitement les contraintes impossibles et les dépassements. |
| LAY-05 | Découper le dessin et les zones de clic selon les conteneurs et le défilement ; conserver la transformation vers les coordonnées locales. |
| LAY-06 | Séparer les zones fixes, les zones défilantes et les couches flottantes ; supporter les défilements imbriqués. |
| LAY-07 | Adapter lignes, colonnes, grilles et panneaux à la largeur disponible, au zoom de l’interface et à la taille des textes. |
| LAY-08 | Recalculer après redimensionnement, changement de police, traduction, arrivée d’une image ou changement de facteur d’échelle. |
| LAY-09 | Proposer une grille décrite par un nombre de colonnes, avec spans, positions explicites et placement automatique faciles à lire en Rust. |
| LAY-10 | Permettre d’activer le redimensionnement de panneaux internes, d’en borner les dimensions et de modifier leur ordre depuis l’application ; prévoir une évolution vers la réorganisation par glisser-déposer. |

Pour RustMusic, la barre supérieure et le lecteur inférieur restent accessibles,
le contenu central peut défiler et la colonne gauche peut posséder son propre
défilement. À faible largeur, une règle explicite replie ou réduit les panneaux
et redistribue les commandes. Les seuils exacts restent à définir avec les écrans.

Les modes `fit`, `fill`, `contain` et `cover`, les ratios et l’espace réservé
aux images évitent les déplacements brusques au chargement. Les noms publics
exacts seront stabilisés pendant la conception de l’API.

## 8 Styles et personnalisation

### 8.1 Système de thème

Le thème fournit des valeurs nommées : couleurs sémantiques, familles et tailles
de police, graisses, hauteurs de ligne, espacements, dimensions de contrôles,
rayons, bordures, ombres, opacités, durées et courbes d’animation.

Il doit permettre les thèmes clair et sombre, une couleur d’accent, une densité
compacte ou confortable et un changement à l’exécution. Les couleurs de surface,
texte, bordure, accent, danger et sélection sont distinctes ; éviter les valeurs
copiées dans chaque composant.

Chaque composant possède des variantes nommées, par exemple `primary`,
`secondary`, `ghost` et `danger` pour un bouton. L’application peut ajouter
ses propres variantes sans dupliquer les interactions.

### 8.2 Propriétés de style

Le socle devra couvrir les fonds unis et dégradés, bordures et rayons par coin,
ombres internes et externes, typographie, opacité, tailles, espacements,
curseurs, contours de focus et transformations utiles. Le flou d’une surface
ou du contenu derrière elle est un effet optionnel P2 ; une ombre douce reste
un besoin P1. Les possibilités d’effets dépendent des capacités du backend.

Les états des contrôles stylisent aussi leurs parties internes : fond du champ,
bordure, texte, placeholder, sélection, curseur de saisie, icônes et message
d’erreur. Un slider expose le rail, la portion remplie et la poignée. Un menu
expose ses items et son indicateur de sélection.

Ordre de résolution proposé : valeurs par défaut, thème et héritage autorisé,
variante, règles d’état, puis propriétés locales explicites. L’API doit préciser
comment une surcharge locale interagit avec un état, pour éviter de faire
disparaître involontairement le focus ou une erreur. Seules les propriétés
prévues, notamment certaines propriétés de texte, sont héritées.

### 8.3 Ergonomie inspirée de Tailwind

Le besoin est une composition rapide de propriétés cohérentes et réutilisables,
en Rust typé. Il n’est pas nécessaire d’implémenter un parseur CSS ni la totalité
du modèle CSS. Des méthodes chaînables et des styles réutilisables peuvent
remplir ce rôle.

Exemple d’intention d’API, illustratif et non compilable dans le prototype actuel :

```rust
button("Lecture")
    .icon(Icon::Play)
    .variant(ButtonVariant::Primary)
    .size(ControlSize::Large)
    .on_activate(Command::Play);

text_input(search_query)
    .placeholder("Rechercher un titre, un artiste, un album…")
    .style(styles.search_field())
    .on_change(Command::SearchChanged);
```

Créer un nouveau style ne doit pas obliger à réécrire la capture souris, le
clavier ou l’accessibilité du composant.

## 9 Texte et saisie

### 9.1 Affichage du texte

Le rendu doit prendre en charge la sélection de police, les polices de repli,
les tailles, graisses, italiques, soulignements, couleurs, alignements,
retours à la ligne et troncatures avec points de suspension. La mesure et le
dessin doivent utiliser des règles cohérentes pour que le curseur et la
sélection correspondent au texte affiché.

Prévoir Unicode, accents combinés, emoji et textes de droite à gauche. Un
caractère visible peut contenir plusieurs points de code : déplacer le curseur
et supprimer du texte ne doit pas découper arbitrairement les octets UTF-8.
L’arabe et les ligatures demandent aussi une mise en forme des glyphes ; cette
responsabilité peut être déléguée à la pile native. Sous Windows, DirectWrite
fournit mesure, mise en page et détection de positions dans le texte.
[Documentation DirectWrite](https://learn.microsoft.com/en-us/windows/win32/directwrite/direct-write-portal).

### 9.2 Édition

| Référence | Exigence P1 |
| --- | --- |
| TXT-01 | Curseur, sélection souris et clavier, extension de sélection, double clic par mot et déplacement par mot ou ligne selon la plateforme. |
| TXT-02 | Couper, copier, coller, tout sélectionner, annuler et rétablir ; conserver un historique d’édition cohérent. |
| TXT-03 | Touches mortes, caractères accentués, disposition de clavier et entrée de texte distinguée des touches physiques. |
| TXT-04 | Composition IME, texte temporaire, validation ou annulation et position correcte de la fenêtre de candidats. |
| TXT-05 | Retour à la ligne et défilement pour garder le curseur visible ; cas de texte long dans un champ court. |
| TXT-06 | Placeholder distinct de la valeur ; valeur vide, lecture seule, désactivation et validation sans destruction d’une saisie en cours. |
| TXT-07 | Labels, aide et erreurs associés au champ ; aucune erreur indiquée seulement par une couleur. |

La saisie internationale nécessite l’intégration des services de texte de
l’OS ; la traduction de `WM_KEYDOWN` en caractères ne suffit pas comme stratégie.
Le service Windows TSF fait partie des pistes à étudier pour les contrôles
personnalisés. [Documentation TSF](https://learn.microsoft.com/en-us/windows/win32/tsf/text-services-framework).

Pour la recherche, l’application choisit son délai de déclenchement, annule
les demandes dépassées et ne remplace pas les résultats récents par une réponse
ancienne. Une composition IME en cours ne doit pas déclencher une validation
prématurée du champ.

## 10 Interactions et couches

### 10.1 Souris et clavier

Le moteur doit distribuer les mouvements, entrées et sorties du pointeur,
pressions, relâchements, doubles clics, molette et défilement fin. Le routage
définit l’élément cible, la propagation dans les parents et la possibilité de
consommer un événement. Il doit permettre de capturer une interaction de
glissement, et de l’annuler proprement en cas de perte de focus ou de capture.

Le gestionnaire de focus définit l’ordre de `Tab` et `Shift+Tab`, l’indication
visuelle, les raccourcis avec modificateurs et les touches propres à chaque
contrôle. Les menus, listes, onglets et sliders ont leurs propres règles.
L’édition d’un champ a priorité sur un raccourci global incompatible.

`Escape` ferme d’abord l’élément transitoire concerné selon sa politique ; il
ne doit pas fermer l’application systématiquement comme dans la démonstration
actuelle. Les raccourcis de l’application doivent être configurables.

Le tactile et le stylet sont P2. Les coordonnées et types d’événements doivent
permettre de les ajouter sans refaire tous les composants.

### 10.2 Superposition

Les couches internes de l’UI et l’ordre des fenêtres du bureau sont deux
responsabilités distinctes. Le socle gère l’ordre de dessin et de ciblage de ses
éléments ; le backend demande au système les propriétés de ses fenêtres.

Ordre par défaut à définir : contenu, panneaux flottants, menus et popovers,
barrière de modale et modale, puis aides autorisées dans ce contexte. Les
couches doivent être organisées par contexte ; un tooltip d’arrière-plan ne
peut pas réapparaître au-dessus d’une modale.

Une couche transparente peut être traversable ou capter les événements : ce
choix doit être explicite. Un contrôle masqué par un panneau ne reçoit pas
accidentellement le clic à sa place.

Une modale bloque les interactions avec l’arrière-plan, contient le focus,
définit si `Escape` ou un clic extérieur la ferme, puis rend le focus à son
déclencheur s’il existe encore. Un menu ou popover se repositionne au défilement,
au redimensionnement et au changement d’échelle, ou se ferme selon son contrat.

### 10.3 Glisser et déposer

Prévoir en P1 le dépôt de fichiers et dossiers depuis le système, et la
réorganisation d’items dans une liste. Définir l’aperçu, l’emplacement d’insertion,
le défilement automatique au bord et l’annulation. Le déplacement de fichiers
ou la modification de la playlist est décidé par l’application après le dépôt.
Les échanges complexes entre applications sont P2.

### 10.4 Fenêtres transparentes et overlays de bureau

Le socle doit permettre une fenêtre dont le fond est **totalement transparent**,
pour afficher directement des éléments au-dessus du bureau ou d’une autre
application : mini-lecteur, commandes flottantes, indicateur ou visualisation.
Ce besoin P1 concerne la transparence de la fenêtre native, au-delà des
superpositions internes à une fenêtre opaque.

| Référence | Exigence |
| --- | --- |
| OVL-01 | Autoriser un fond avec alpha nul et une transparence par pixel, tout en conservant du texte, des images et des contrôles opaques ou semi-transparents. |
| OVL-02 | Distinguer l’opacité du fond de l’opacité globale de la fenêtre : rendre le fond transparent ne doit pas rendre les enfants invisibles. |
| OVL-03 | Désactiver indépendamment les décorations, la bordure, l’ombre et les surfaces de fond automatiques ; aucun rectangle opaque ne doit apparaître derrière l’overlay. |
| OVL-04 | Proposer un mode interactif, un mode entièrement traversable par les clics et un mode où seules des zones déclarées captent les interactions. Les autres zones transmettent les clics aux fenêtres derrière. |
| OVL-05 | Définir les zones interactives séparément de leur alpha : un élément invisible ne doit pas intercepter les clics par accident ; une zone transparente peut être interactive si elle est déclarée comme telle. |
| OVL-06 | Rendre configurable le maintien au-dessus des fenêtres ordinaires, indépendamment de la transparence. Respecter les capacités et règles du système. |
| OVL-07 | Prévoir l’affichage sans prise de focus pour un overlay passif ; un overlay avec champ de saisie doit pouvoir recevoir explicitement le focus. La présence dans la barre des tâches ou le sélecteur de fenêtres est une option distincte, selon la plateforme. |
| OVL-08 | Permettre à l’application de changer de politique d’interaction, par exemple passer d’un affichage traversable à un mode de déplacement ou d’édition, avec une commande de sortie définie. |
| OVL-09 | Conserver la transparence et des contours corrects lors du redimensionnement, du changement d’échelle et des mises à jour, en mode GPU comme en mode logiciel. |

Les modes traversables portent sur la distribution des événements aux autres
fenêtres du bureau ; ignorer un événement à l’intérieur de l’arbre UI ne suffit
pas à remplir ce contrat. Le backend expose ses capacités pour la transparence,
les régions interactives, l’activation et l’ordre des fenêtres. Une demande
non prise en charge produit un résultat explicite ; elle ne doit pas créer
silencieusement une fenêtre opaque.

Le prototype de validation affichera du texte, un bouton et une image sur fond
transparent, puis vérifiera les trois politiques de clic au-dessus d’une
fenêtre de test. L’overlay passif devra pouvoir rester affiché pendant que
l’utilisateur continue à saisir dans la fenêtre derrière.

## 11 Rendu et ressources

### 11.1 Contrat graphique commun

Le moteur de rendu reçoit des opérations indépendantes des OS : rectangles
arrondis, traits, chemins vectoriels, images, texte mis en page, dégradés,
ombres, transformations, masques et zones de découpage. Les composants ne
doivent pas manipuler directement un `Hwnd`, un contexte GDI ou un objet GPU.

Les icônes utilisent un ensemble de chemins vectoriels ou un format choisi
explicitement. Le support complet de SVG, notamment filtres et texte, est P2 ;
il ne doit pas être présumé fourni par un simple dessin de chemins.

Prévoir un rendu sans scintillement, une composition alpha cohérente et une
gestion documentée des couleurs. Le rendu et le ciblage doivent utiliser les
mêmes transformations et règles de découpage. Les différences d’anticrénelage
entre backends ne doivent pas changer les dimensions ni les interactions.

La présentation native doit préserver l’alpha du contenu jusqu’à la composition
avec le bureau lorsque la transparence est demandée. Effacer une surface avec
un alpha nul ne suffit pas si le chemin de présentation la rend ensuite opaque.
Les contours de texte et d’image doivent être validés sur fonds clairs et
sombres, sans halo dû à une composition alpha incorrecte.

### 11.2 Modes de rendu

| Mode | Comportement attendu |
| --- | --- |
| `Auto` | Choisir l’accélération disponible, signaler le backend effectif et utiliser une solution logicielle si l’initialisation matérielle échoue. |
| `Gpu` | Demander explicitement le GPU ; renvoyer une erreur exploitable si indisponible, afin que l’application choisisse la suite. |
| `Software` | Produire le contenu UI sur CPU ; mode utilisable sans accélération de ce contenu. |

En mode logiciel, le système peut encore utiliser son GPU pour composer le
bureau. L’exigence porte sur le rendu de notre contenu, pas sur l’ensemble du PC.

Les composants restent utilisables dans les deux modes. Texte, images,
géométrie, clipping et interactions appartiennent au socle obligatoire. Les
effets facultatifs peuvent avoir une variante plus simple, déclarée par les
capacités du backend et documentée. Une perte du périphérique graphique doit
permettre de recréer les ressources ou d’effectuer le repli prévu sans perdre
les valeurs des champs et l’état de navigation.

Sous Windows, Direct2D constitue une piste pour partager une API entre rendu
accéléré et rendu logiciel ; le choix précis des interfaces et de la présentation
reste à valider par un prototype. La documentation décrit ses deux possibilités
de rendu. [Présentation de Direct2D](https://learn.microsoft.com/en-us/windows/win32/direct2d/direct2d-overview).

### 11.3 Ressources et mises à jour

Charger et décoder les images hors du chemin critique de l’interface ; créer
les ressources graphiques sur le contexte approprié. Prévoir cache de
miniatures, cache de texte et de glyphes, limites mémoire, éviction et
libération des ressources à la fermeture d’une vue. Les formats raster
initiaux à viser sont PNG et JPEG ; les formats supplémentaires sont explicites.

Une image manquante ou invalide utilise un remplacement. Un chargement terminé
après la destruction d’un composant ne doit pas écrire dans un composant
réutilisé pour une autre donnée.

Redessiner sur changement, défilement ou animation active. Distinguer ce qui
demande une nouvelle mesure, une mise en page ou seulement une peinture.
Le clignotement d’un curseur ou la progression d’un morceau ne doit pas
entraîner la reconstruction de toute la bibliothèque musicale.

Les animations P1 couvrent survol, ouverture, opacité et transitions simples.
Elles doivent être annulables, utiliser le temps écoulé et respecter une
préférence de réduction des animations. Une fenêtre masquée ou minimisée
réduit les travaux UI inutiles.

## 12 Création de composants personnalisés

### 12.1 Composition avant dessin spécialisé

Un composant comme `AlbumCard` assemble une image, un conteneur, du texte et
des boutons. Il expose ses données, ses événements et son style sans recoder
la gestion de focus ou le rendu des caractères. Cette voie doit être la plus
simple et la mieux documentée.

Les emplacements de contenu personnalisable permettent de changer l’icône,
le contenu d’un bouton, la ligne d’une liste ou le corps d’un popover sans
modifier la bibliothèque.

### 12.2 Composant à dessin propre

Un composant spécialisé peut définir sa mesure, sa disposition, sa peinture,
son traitement des événements et sa sémantique accessible. Les noms de ces
points d’extension restent à stabiliser ; des comportements par défaut et des
helpers évitent d’écrire chaque étape lorsqu’elle n’est pas utile.

Il doit pouvoir :

- dessiner dans un contexte commun aux backends, avec découpage et transformation ;
- contenir ou réutiliser des composants standard ;
- recevoir le focus et les interactions, capturer un glissement, puis l’annuler ;
- demander une nouvelle mesure ou un redessin local ;
- lire le thème et exposer des propriétés stylables ;
- décrire un rôle, un nom, une valeur et des actions accessibles ;
- libérer ses ressources lors de sa destruction.

Le premier test d’extension sera une barre de progression spécialisée ; un
second pourra être un égaliseur ou une forme d’onde factice. Les données audio
sont fournies par l’application. Une extension avec shader GPU propriétaire
est P2 et doit déclarer son comportement en mode logiciel.

**Critère central :** ajouter un nouveau composant composé ou dessiné ne doit
exiger ni modification de `platform/windows/`, ni ajout d’un `match` central
recensant tous les types de composants de toutes les applications.

## 13 Accessibilité et localisation

Les éléments interactifs exposent rôle, nom, description utile, valeur,
états, ordre logique, focus et actions. Les modifications pertinentes sont
annoncées sans annoncer chaque mise à jour du temps de lecture.

Les opérations accessibles doivent réutiliser les mêmes commandes que la
souris et le clavier. Une liste virtualisée doit permettre la navigation et
l’accès aux items hors écran par une stratégie prévue à cet effet.

Les contrôles dessinés sur mesure n’héritent pas automatiquement de
l’accessibilité des contrôles standard Windows : le backend doit exposer les
fournisseurs UI Automation appropriés.
[Fournisseurs UI Automation](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-providersoverview).

La conception doit prévoir l’adaptation au contraste renforcé, un focus visible,
les labels des boutons sans texte, la mise à l’échelle des textes et l’usage
sans souris. Pour le premier ensemble de contrôles Windows, vérifier ces
besoins avec le Narrateur et la navigation clavier.

Les chaînes affichées appartiennent à l’application ou à ses ressources de
traduction. Prévoir des textes plus longs, les pluriels, les formats de nombres
et de durées, et la direction du texte. Les règles de mise en page ne doivent
pas dépendre uniquement des libellés français de la capture.

## 14 Architecture et intégration des plateformes

Répartition de responsabilités proposée, à préciser après ce cahier des charges :

```text
Application RustMusic
  données et commandes métier, pages, thème, composants métier
                    ↓
Bibliothèque de composants
  boutons, champs, listes, menus, modales et composants personnalisés
                    ↓
Services communs du moteur
  identité, arbre UI, état, layout, événements, focus, styles,
  commandes de rendu, ressources et sémantique accessible
                    ↓
Backends et services de plateforme
  Windows                 macOS                  Linux
  fenêtres, rendu, texte, saisie, accessibilité et dialogues système
```

L’arbre UI est ici une organisation logique des éléments ; le choix entre API
déclarative, objets persistants ou combinaison des deux reste une décision
d’architecture. Le besoin ferme est de conserver l’identité et l’état, de
composer facilement et de mettre à jour efficacement.

| Service | Partie commune | Adaptation native |
| --- | --- | --- |
| Fenêtres | Options, événements, commandes, capacités | Création, affichage, déplacement, redimensionnement, échelle, plein écran, transparence, politiques de clic, activation et fenêtres secondaires. |
| Dessin | Opérations et ressources abstraites | Ressources CPU/GPU, présentation, perte du périphérique. |
| Texte | Propriétés, valeur, sélection et contrat de mesure | Polices, mise en forme, rasterisation et conversion des indices si nécessaire. |
| Saisie | Événements normalisés, focus logique | Touches, texte, IME, curseurs, presse-papiers, drag and drop. |
| Accessibilité | Rôles, valeurs, structure et actions | API d’accessibilité de chaque OS. |
| Intégration | Résultats typés et annulation | Dialogues fichier et dossier, ouverture d’un lien, préférences système pertinentes. |

Les FFI et contrats mémoire restent confinés dans les backends. L’API publique
courante vise un Rust sûr. Les handles natifs, l’appartenance au thread UI,
la réentrance des callbacks et la durée de vie des ressources doivent être
documentés et protégés dans les adaptateurs.

Les opérations longues utilisent des tâches de fond et renvoient des résultats
par un mécanisme explicite au thread UI. Il faut un moyen d’annuler les tâches,
de réveiller ce thread et d’ignorer les réponses obsolètes. Aucune lecture de
disque, requête réseau ou analyse de bibliothèque ne doit bloquer la saisie.

Une première implémentation fournit `Task`, `TaskCommand` et `TaskResult`,
avec les callbacks facultatifs `take_task_commands` et `task_completed`.
Le pool est créé à la première demande ; il possède au plus quatre workers
et 64 travaux en attente. Les scopes de fenêtre isolent les IDs, annulent à
la fermeture et rejettent les anciennes versions. Le réveil est raccordé
à Windows. L’annulation est coopérative ; les caches et chargements d’images
ne sont pas encore intégrés à ce service. Voir le [guide](tasks.md).

Le backend Windows est développé en premier. macOS et Linux auront leurs
propres implémentations et tests, sans partager artificiellement les handles
ou hypothèses de Win32. Pour Linux, Wayland et X11 sont deux périmètres à
traiter explicitement. Le protocole Wayland de base ne donne pas aux clients
la position globale de leurs surfaces ; le placement libre d’une fenêtre
d’ombre séparée ne peut donc pas être transposé tel quel depuis le prototype.
[Modèle du protocole Wayland](https://wayland.freedesktop.org/docs/book/Protocol.html).

Prévoir les capacités natives et leurs alternatives pour les décorations,
ombres, menus, positionnements et fonctions de fenêtre. Le mode « toujours
au-dessus » est une option de l’application, pas une obligation de toute UI.
La gestion de plusieurs fenêtres indépendantes est une première fondation
du projet Karbon UI ; son implémentation Windows est disponible via
`platform::run_windows` et les commandes d’ouverture/fermeture différées.
Le docking libre et les vues détachables restent P2 ; ils ne sont pas
nécessaires pour valider ce premier contrat multi-fenêtre.

## 15 Performance et critères mesurables

La cible est une interface réactive et prévisible, nettement plus économe et
rapide qu’une interface Tauri comparable. Il s’agit d’un objectif de produit,
pas d’un résultat établi : aucun gain chiffré n’est promis sans comparaison sur
les mêmes écrans, données et machines.

La comparaison doit utiliser des builds release, le même contenu, la même
taille et échelle de fenêtre, et une qualité visuelle comparable. Documenter
l’OS, le matériel, le backend graphique et la version Tauri/WebView. Mesurer
l’ensemble des processus de chaque application pour éviter une comparaison
partielle ; distinguer le coût UI des tâches métier communes.

Conserver les résultats bruts et les conditions de chaque campagne : démarrage
à froid/à chaud, mémoire CPU et GPU, CPU au repos, défilement, ouverture de
menus et redimensionnement. Rapporter les temps d’image et latences avec leurs
percentiles, les saccades et les évolutions après ouvertures/fermetures répétées.
La galerie et les compteurs actuels ne constituent pas encore ce benchmark.

Les seuils ci-dessous sont des **objectifs initiaux proposés**, à calibrer sur
une machine de référence documentée avant d’en faire des conditions de livraison.

| Mesure | Objectif proposé | Conditions de mesure |
| --- | --- | --- |
| Fluidité GPU | Viser 60 images/s pendant défilement et animations ; budget indicatif de 16,7 ms par image. | Vue test 1920 × 1080, échelle 100 %, ressources déjà en cache, matériel et pilote documentés. |
| Fluidité CPU | Viser 30 images/s sur la même scène, avec les éventuelles simplifications d’effets déclarées. | Même jeu de données ; mode logiciel forcé et backend effectif vérifié. |
| Réponse d’une action locale | Retour visuel dans les 100 ms au 95e percentile. | Bouton, focus, ouverture d’un menu ; exclure le temps de réponse d’un service externe. |
| Collections | Naviguer dans 10 000 titres sans créer 10 000 composants de ligne simultanément. | Comparer le nombre d’éléments matérialisés à la zone visible et à la marge de préchargement. |
| Repos | Pas de boucle de dessin continue sans changement ni animation ; thread UI en attente. | Fenêtre statique et données sans changement ; mesurer séparément les activités métier. |
| Mémoire | Caches d’images et de texte bornés, limites configurables et libération vérifiable. | Mesurer mémoire CPU et GPU, à froid, à chaud, puis après fermeture répétée des vues. |
| Ressources | Pas de croissance non bornée des handles, surfaces et textures. | Répéter ouvertures, fermetures, changements de thème et redimensionnements. |
| Démarrage | Mesurer le temps jusqu’au premier écran interactif, à froid et à chaud. | Bibliothèque disponible ou indisponible ; ne pas attendre le scan pour afficher l’UI. |

La bibliothèque doit offrir des compteurs de temps de mesure, layout, peinture,
images décodées, mémoire de cache et éléments visibles. Les listes à hauteur
variable et une épreuve à 100 000 items sont P2 ; le socle doit d’abord valider
la virtualisation de lignes à hauteur fixe et de grilles régulières.

## 16 Scénarios de recette

| Référence | Scénario | Résultat attendu |
| --- | --- | --- |
| REC-01 | Redimensionner une page composée avec barre, colonne, contenu et lecteur fixe. | Pas de chevauchement involontaire ; zones critiques accessibles ; règles de repli documentées. |
| REC-02 | Passer la fenêtre entre des écrans à 100 %, 150 % et 200 %. | Texte et dessins recalculés ; clics, curseur de saisie et popovers correctement placés. |
| REC-03 | Tester un bouton à la souris, au clavier et via son action accessible ; relâcher la souris à l’extérieur. | Activation unique dans les cas valides, annulation du clic extérieur, état désactivé respecté. |
| REC-04 | Éditer `Été`, un accent combiné, un emoji composé et du texte arabe ; copier, coller, annuler, rétablir. | Valeur intacte, sélection et déplacements cohérents avec les caractères affichés. |
| REC-05 | Saisir avec un IME et confirmer une suggestion. | Composition distincte de la valeur validée, candidats près du curseur, aucune commande globale déclenchée par erreur. |
| REC-06 | Ouvrir et parcourir un select ou menu près d’un bord, puis défiler ou redimensionner. | Choix au clavier, contenu lisible, repositionnement ou fermeture conforme à la politique. |
| REC-07 | Ouvrir une modale au clavier, naviguer avec Tab, la fermer. | Aucun focus ou clic dans l’arrière-plan ; retour du focus au déclencheur valide. |
| REC-08 | Parcourir 10 000 titres, sélectionner une ligne, trier et revenir à la vue. | Sélection attachée à l’identité, défilement maîtrisé, nombre borné de lignes matérialisées. |
| REC-09 | Recevoir deux résultats de recherche dans l’ordre inverse des demandes. | Les données obsolètes ne remplacent pas les plus récentes ; champ toujours utilisable. |
| REC-10 | Changer de thème, taille de texte et réduction des animations à l’exécution. | Propagation cohérente, focus visible, aucun contrôle perdu ou devenu illisible. |
| REC-11 | Forcer CPU puis GPU sur la galerie ; provoquer l’indisponibilité du GPU dans un test contrôlé. | Même fonctionnalité ; `Auto` utilise le repli prévu ; `Gpu` renvoie une erreur explicite si nécessaire. |
| REC-12 | Créer une carte d’album puis une barre de progression spécialisée depuis le code d’exemple. | Aucune modification du backend ; thème, focus, événements et accessibilité disponibles. |
| REC-13 | Utiliser le Narrateur et parcourir la page sans souris. | Rôles, noms, valeurs et actions pertinentes accessibles ; ordre de navigation utilisable. |
| REC-14 | Annuler un sélecteur de fichiers ; déposer des fichiers, puis annuler un réordonnancement. | Annulation distincte d’une erreur ; aucune action métier prématurée. |
| REC-15 | Charger une image absente, quitter une vue pendant un chargement et rouvrir des panneaux. | Remplacement visible, aucune écriture dans une vue détruite, pas de fuite progressive. |
| REC-16 | Cacher, minimiser, restaurer puis fermer la fenêtre. | Reprise correcte du rendu et de la saisie ; ressources et fenêtres auxiliaires libérées. |
| REC-17 | Afficher texte, image et bouton sur un fond de fenêtre d’alpha nul, au-dessus de fonds clairs et sombres, en CPU puis GPU. | Fond entièrement transparent, contenu visible, bords sans halo et aucune surface opaque résiduelle ; vérifier aussi après redimensionnement et changement d’échelle. |
| REC-18 | Passer entre les modes interactif, entièrement traversable et interactif par zones au-dessus d’une fenêtre de test. | Les clics atteignent exactement la fenêtre ou le contrôle prévu ; aucune interception accidentelle par les zones transparentes. |
| REC-19 | Afficher un overlay passif pendant une saisie dans une autre fenêtre, puis activer volontairement un champ de l’overlay. | Pas de vol de focus à l’affichage ; saisie possible après activation explicite ; maintien au-dessus et décorations conformes aux options et capacités déclarées. |

Les tests unitaires porteront notamment sur contraintes de mise en page,
priorités de style, focus, conversion des coordonnées et édition. La galerie
permettra les tests visuels ; les scénarios natifs compléteront les tests
communs pour la saisie, l’accessibilité et les fenêtres.

Une comparaison d’images doit tolérer les différences de rendu des polices
entre plateformes et conserver des références propres à chaque backend.

## 17 Ordre de réalisation proposé

| Jalon | Livrable | Condition de passage |
| --- | --- | --- |
| J0 | Cahier des charges et choix de périmètre. | Priorités retenues, premières inconnues identifiées ; aucun moteur supplémentaire implémenté à cette étape. |
| J1 | Démonstrateur graphique Windows : texte, images, chemins, clipping, thème minimal, fond de fenêtre transparent et essais CPU/GPU. | Valider la faisabilité du texte, de la transparence de bout en bout et des deux modes avant de construire tout le catalogue ; comparaison visuelle et mesures initiales. |
| J2 | Composition, layout, identité, routage d’événements et focus ; première galerie avec bouton et champ. | Adapter la page à la fenêtre ; clavier, texte simple et structure accessible prouvés de bout en bout. |
| J3 | Saisie complète et première collection de contrôles. | TextInput, boutons, choix, sliders, formulaires ; recette Unicode, IME, clavier et accessibilité du premier ensemble. |
| J4 | Couches, menus, popovers, modales, défilement et collections virtualisées. | Recette des couches, focus, listes de 10 000 titres et chargements asynchrones. |
| J5 | Écran RustMusic représentatif avec données factices et composants personnalisés. | En-tête, navigation, cartes, lecteur, file, menu, formulaire et modale ; style réutilisable et mesures CPU/GPU. |
| J6 | Intégration progressive dans RustMusic. | Remplacement par vue et branchement des commandes existantes ; pas de migration totale en une étape. |
| J7 | Backends macOS et Linux. | Même galerie et contrats, recette native de chaque plateforme, capacités et différences documentées. |

L’accessibilité et la saisie internationale sont validées dès les premiers
contrôles. Le matériel GPU, les piles de texte et la contrainte sans crate sont
des sujets de faisabilité à tester tôt, avant la création de nombreux composants.
Les fonctionnalités P2 ne retardent pas automatiquement ces jalons.

## 18 Livrables du socle

- Une API publique documentée avec des exemples courts de composition et de style.
- Une galerie locale montrant chaque composant dans ses états et variantes, utilisable sans RustMusic.
- Un exemple de page RustMusic avec données factices, sans moteur audio requis.
- Un guide de création d’un composant composé et d’un composant à dessin propre.
- Une matrice des capacités par OS et mode de rendu, avec alternatives explicites.
- Les scénarios de recette et un relevé reproductible des performances.

La galerie inclura une page de formulaires, une page de listes et de tableaux,
une page de menus et modales, une page de texte international, une page de
thèmes et une page d’éléments personnalisés. Elle proposera aussi un exemple
d’overlay de bureau à fond entièrement transparent avec les trois politiques
de clic et les options d’activation.

## 19 Limites du périmètre et décisions restantes

Le socle expose les commandes et les états nécessaires à RustMusic. Le décodage
audio, le DSP, les pilotes de sortie audio, l’indexation musicale, la base de
données, les comptes, le réseau et les recommandations restent des services
de l’application. Une onde ou un égaliseur UI consomme leurs données.

Un navigateur, un moteur CSS complet, un traitement de texte riche, un moteur
3D général et une compatibilité totale avec toutes les bibliothèques d’UI
existantes sont hors périmètre.

Avant l’implémentation des parties concernées, préciser :

| Décision | Moment utile |
| --- | --- |
| Versions minimales de Windows et architecture processeur initiale | Avant J1. |
| Pile native de rendu, de texte et de saisie Windows, stratégie de repli et effets communs CPU/GPU | Prototypes J1 puis saisie J2 à J3. |
| API de construction, stockage de l’état et mécanisme d’extension | Essais sur quelques composants J2, avant stabilisation de l’API. |
| Polices, icônes, ressources graphiques et variantes exactes de RustMusic | Avant validation visuelle J5. |
| Largeur minimale et règles d’adaptation de chaque écran RustMusic | Pendant J2 puis validation J5. |
| Machine de référence et budgets de performances définitifs | Mesures J1, puis confirmation J5. |
| Ordre macOS et Linux, Wayland et X11, bibliothèques natives Linux autorisées et déployables | Avant J7. |
| Premiers besoins P2 réellement nécessaires | À partir d’un écran ou d’un composant concret. |

Le premier socle sera considéré utilisable lorsque les composants P1 retenus
fonctionneront avec thème, clavier, texte, accessibilité et rendu CPU/GPU selon
les contrats ci-dessus, et qu’un écran RustMusic pourra être composé sans
ajouter de code spécifique à RustMusic dans le cœur ou le backend Windows.
