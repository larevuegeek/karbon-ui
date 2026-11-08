# Tooltip : une aide au survol et au clavier

L’info-bulle est attachée à un élément. Elle ne crée pas un élément de layout
supplémentaire et ne prend ni le focus ni la capture de souris.

```rust
widgets::button(2, "Réglages", Msg::OpenSettings)
    .tooltip("Ouvrir les réglages rapides sans quitter cette page.")
```

Le délai par défaut est de 500 ms. Le survol fonctionne aussi sur les textes,
les conteneurs et les widgets désactivés. Un contrôle focalisé au clavier peut
afficher son aide sans mouvement de souris. Le texte d’aide complète le nom
accessible du contrôle ; il ne remplace pas ce nom.

## Personnaliser

Importer `Tooltip` et `TooltipStyle` depuis `ui::widgets` ou `ui::tooltip` :

```rust
widgets::button(2, "Réglages", Msg::OpenSettings).tooltip_with(
    Tooltip::new("Régler le volume et les notifications.")
        .delay(Duration::from_millis(350))
        .max_width(280.)
        .style(TooltipStyle {
            font_size: 13.,
            background: Some(Color::rgb(25, 30, 35)),
            foreground: Some(Color::rgb(245, 245, 245)),
            ..TooltipStyle::default()
        }),
)
```

`Duration` vient de `std::time`. `border` personnalise aussi le contour.
Les couleurs laissées à `None` suivent le thème. La taille du texte vaut 12
par défaut ; la largeur maximale vaut 320 unités. Le texte se replie sur
plusieurs lignes, avec un padding horizontal de 10 et vertical de 8 unités.

La validation refuse un texte vide, plus de 16 384 octets de texte, un délai
supérieur à 60 secondes, une largeur hors de 40–4 096 unités ou une taille de
police hors de 8–96. Les valeurs flottantes doivent être finies. Un délai nul
permet un affichage immédiat.
Dans un template, `title=""` (ou un `title={…}` calculé vide ou blanc) n’attache
aucune info-bulle, comme en HTML : la validation ne voit jamais ce texte vide.

## Affichage et fermeture

L’info-bulle apparaît près du rectangle visible de son élément. Elle préfère
le dessous, passe au-dessus si l’espace est meilleur et reste dans la zone
de contenu de la fenêtre. Le texte est découpé si l’espace disponible ne suffit
pas : cette première version ne fait pas défiler les aides longues.

Le clavier prend la priorité sur un pointeur immobile. Un vrai mouvement du
pointeur rend la priorité au survol. La liaison la plus proche dans l’arbre
fournit l’aide, avant celle d’un conteneur parent. Le pointeur doit être dans le
rectangle visible de l’élément (sa boîte limitée par son clip) : un voisin dont
seul le clip, hérité d’un ancêtre sans `overflow`, contient le pointeur ne prend
pas l’aide.

| Événement | Comportement |
| --- | --- |
| Rester sur un élément ou lui donner le focus clavier | Attendre le délai, puis afficher son aide. |
| Bouger dans le même élément | Conserver l’échéance ; ne pas recommencer le délai à chaque pixel. |
| Passer dans l’info-bulle ou dans le petit espace qui la sépare de l’élément | La garder visible pour permettre sa lecture. |
| Échap lorsqu’elle est visible | Fermer seulement l’info-bulle ; conserver le contrôle focalisé. |
| Clic, saisie, réglage au clavier ou scroll | Masquer l’aide pour laisser l’interaction se poursuivre. |
| Quitter l’élément après une fermeture explicite | Réarmer l’aide lors d’un nouveau survol ou focus pertinent. |
| Retrait de l’élément, changement de géométrie ou de contenu | Abandonner l’ancienne aide ; recalculer la cible et le délai. |
| Perte de focus de la fenêtre | Masquer et annuler le réveil en attente. |

L’aide n’ajoute pas de zone au hit-test. Si elle recouvre un bouton et qu’un
clic est reçu à cet endroit, le moteur traite le bouton situé dessous.
L’info-bulle reste visible pendant la lecture, sans fermeture automatique au
bout de quelques secondes. Elle peut être fermée avec Échap.

Les captures de souris et les listes Select ouvertes suspendent les aides.
Une modale ou un popover limite les cibles à son propre contenu : aucune aide
de la page masquée ne réapparaît au-dessus. Échap ferme l’aide avant de fermer
le dialogue ou le popover lors d’un second appui.

## Coût et réveil Windows

L’état transitoire appartient au runtime, pas à l’application : aucun booléen
ni callback métier n’est nécessaire pour ouvrir une info-bulle.
Le texte est partagé par `Arc`. Seule la cible courante est conservée ; retirer
l’élément libère ses références. La mesure du texte de l’aide se fait à son
affichage, puis reste en cache tant que sa configuration et sa géométrie restent
identiques. La page ne doit pas être remesurée pour afficher l’aide.

`UiRuntime::next_wake()` expose l’échéance monotone en attente. Le backend Windows
arme `SetTimer` pour cette échéance, puis le désarme à sa réception et envoie
`UiEvent::Timer(Instant::now())`. Aucun timer d’info-bulle ne tourne lorsqu’il
n’y a rien à attendre, ni lorsque l’aide est déjà affichée. Le clignotement
du caret du TextInput conserve son timer distinct.
Les délais Windows restent approximatifs et dépendent du traitement de la file
de messages ; voir [SetTimer](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-settimer).
Les futurs backends devront raccorder ce même contrat de réveil à leurs services natifs.

## Accessibilité

Un contrôle possédant un nœud accessible fournit son texte dans `HelpText` même
quand l’aide n’est pas dessinée. Pour un champ avec placeholder, les deux textes sont conservés,
séparés par un saut de ligne.

À l’affichage, Windows reçoit aussi un nœud `ToolTip` distinct, enfant du contrôle
ou de la fenêtre si l’ancre n’a pas de nœud accessible. Son identifiant est
`tooltip-{id}`. Il ne possède aucun pattern d’action, n’est pas focalisable et
n’ajoute pas d’arrêt à Tab. Il appartient à la vue de contrôle UI Automation ;
son contenu est fourni par le `HelpText` de l’ancre dans la vue de contenu.
Les notifications `ToolTipOpened` et `ToolTipClosed` suivent son apparition
et sa disparition. Voir le [contrat Microsoft ToolTip](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supporttooltipcontroltype).
Un conteneur purement visuel sans nœud accessible expose seulement l’aide
affichée ; il n’ajoute pas de HelpText permanent à l’arbre.

La recette d’annonces avec Narrateur/NVDA reste ouverte. Le client utilisé dans
les essais peut garder une indication de focus sur le premier champ malgré le
déplacement réel du focus ; les tests COM vérifient séparément que l’aide ne
change pas le fournisseur focalisé.

## Démo et vérifications

```sh
cargo test --offline --lib
```

Survoler « Ajouter +1 », « Réglages », « Actions » ou « Afficher une modale ».
Un premier Tab focalise le bouton d’en-tête et affiche son aide après le délai.
Échap ferme l’aide ; Tab puis Shift+Tab permettent de la réafficher. Une option
désactivée possède aussi son aide dans le panneau des choix. Le bouton Réglages
du formulaire illustre une cible dans une modale.

Dix tests communs couvrent délai, annulation, focus, pointeur immobile,
survol de l’aide, clics traversants, scopes, Select, cache, libération et validation.
Deux tests natifs couvrent identité, propriétés passives, coordonnées, focus,
durée de vie et événements UI Automation. La suite complète compte 144 tests,
y compris GPU et WARP ; formatage et Clippy passent.

Vérifié à l’écran : réveil natif après le focus par Tab, apparition du texte et
du nœud accessible, fermeture par Échap sans fermeture de la fenêtre, réarmement
au clavier et conservation après passage à WARP et au thème clair. L’instance
de recette a été fermée. Le survol et les clics traversants sont couverts par
les événements simulés ; leur recette manuelle reste à compléter.

| Fichier | Rôle |
| --- | --- |
| `src/ui/overlays/tooltip/mod.rs` | Configuration, délai, cible, mesure, dessin et suppression. |
| `src/ui/core/element/mod.rs` | `.tooltip` et `.tooltip_with`. |
| `src/ui/runtime/mod.rs` | Priorité clavier/souris, événements et intégration à la scène. |
| `src/platform/windows/window/timer.rs` | Armement et annulation du réveil ponctuel. |
| `src/platform/windows/accessibility/tooltip.rs` | Nœud ToolTip natif et tests COM. |

Une infobulle est acceptée sous un élément animé ou transformé (`transform` au
survol) : elle se place alors sur la géométrie non transformée.

Cette étape fournit des aides textuelles. Les panneaux interactifs avec boutons,
liens ou formulaires restent des [popovers](popover.md).

