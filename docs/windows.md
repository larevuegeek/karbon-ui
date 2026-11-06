# Plusieurs fenêtres et cycle de vie

Cette étape prépare Karbon UI avec un premier gestionnaire de fenêtres
applicatives. L’API est commune ; seul le backend Windows est implémenté.
Le nom Cargo est `karbon-ui`.

## Exemple minimal

```powershell
cargo run --offline --example multi_window --target-dir target/library-check
```

Cet exemple ouvre deux fenêtres indépendantes avec le même socle public.
L’application choisit ses commandes et sa politique de fermeture.

## Une identité et une application par fenêtre

`WindowId` est l’identité logique choisie par l’application. Elle ne contient
aucun handle Windows. Deux fenêtres vivantes doivent avoir des identifiants
différents ; un identifiant peut être réutilisé après sa fermeture.

`WindowDescriptor` rassemble cet ID, les options et l’application qui possède
le contenu. Voici le principe, avec un type applicatif `MyWindow` :

```rust,ignore
let first = WindowDescriptor::new(
    WindowId(1),
    WindowOptions::DEFAULT,
    MyWindow::default(),
);
let second = WindowDescriptor::new(
    WindowId(2),
    WindowOptions { x: 400, light: true, ..WindowOptions::DEFAULT },
    Inspector::default(),
);
platform::run_windows(vec![first, second], ExitPolicy::LastWindowClosed)?;
```

`WindowDescriptor::new` reçoit l’application par valeur. Chaque application
déclare son propre `type Message` et reçoit les messages de sa vue dans
`update` : ici, `MyWindow` et `Inspector` peuvent utiliser des types de message
différents. Elle construit sa vue avec `view`. Les modèles métier peuvent être partagés explicitement, par
exemple avec `Rc<RefCell<Model>>` sur le thread UI ; le moteur ne les partage
pas automatiquement.

L’entrée `platform::run(options, application)` reste disponible et reçoit elle
aussi l’application par valeur. Elle crée une fenêtre avec `WindowId::MAIN` et
utilise `LastWindowClosed`.

## Ouvrir et fermer depuis un bouton

Une application conserve ses demandes dans `Vec<WindowCommand>`. Dans sa
méthode `update`, elle ajoute une commande :

```rust,ignore
self.commands.push(WindowCommand::Open(WindowDescriptor::new(
    WindowId(2),
    options,
    MyWindow::default(),
)));
```

Elle rend puis vide sa file avec le nouveau callback :

```rust,ignore
fn take_window_commands(&mut self) -> Vec<WindowCommand> {
    std::mem::take(&mut self.commands)
}
```

Le moteur extrait les demandes après les callbacks et les exécute depuis la
boucle principale. Les appels natifs n’ont donc pas lieu pendant que le
callback applicatif manipule son état. Un message de réveil permet aussi de
traiter une demande provenant d’un message Windows envoyé pendant l’attente
de la boucle. `update` peut renvoyer `false` si la vue n’a pas changé : cela
n’empêche pas la commande de fenêtre d’être traitée.

Les commandes disponibles sont :

| Commande | Effet |
| --- | --- |
| `Open(descriptor)` | Créer, initialiser, peindre puis afficher une fenêtre. |
| `Action(id, action)` | Réduire, agrandir/restaurer ou fermer via `WindowAction`, notamment depuis une barre de titre personnalisée. |
| `Close(id)` | Détruire la fenêtre visée ; ne fait rien si elle est déjà fermée. |
| `Exit` | Fermer toutes les fenêtres et abandonner les demandes suivantes du lot. |

L’ordre des demandes d’une même file est conservé. Il n’y a pas de garantie
d’ordre global entre deux applications de fenêtre. Le callback doit rendre
les demandes une seule fois, et non recréer la même ouverture à chaque appel.

Une ouverture avec un ID déjà utilisé renvoie `Error::DuplicateWindowId`, des
options invalides `Error::InvalidWindowOptions`. Les erreurs de création, de
chargement et de rendu arrêtent actuellement la session entière ; le
`karbon_ui::Result` renvoyé par `run`/`run_windows` permet de les traiter :

```rust,ignore
match platform::run_windows(windows, ExitPolicy::LastWindowClosed) {
    Ok(()) => {}
    Err(karbon_ui::Error::DuplicateWindowId(id)) => eprintln!("ID réutilisé : {id:?}"),
    Err(karbon_ui::Error::Panicked { message }) => eprintln!("panique : {message}"),
    Err(error) => return Err(error),
}
```

Les fenêtres déjà créées sont nettoyées avant le retour.

## Choisir quand l’application s’arrête

| Politique | Comportement |
| --- | --- |
| `LastWindowClosed` | Une fenêtre peut survivre à celle qui l’a ouverte ; sortie après la dernière fermeture. |
| `MainWindowClosed(WindowId(1))` | Sortie à la fermeture de cette fenêtre, avec destruction des fenêtres restantes. |

La fenêtre désignée comme principale doit être présente dans la liste
initiale. Une liste initiale vide ou contenant des IDs en double est refusée.
Une demande `Exit` arrête la session quelle que soit la politique.

## État et ressources sous Windows

Une seule boucle attend les événements de toutes les fenêtres du thread UI.
Chaque fenêtre possède son runtime, son renderer, sa surface, son thème, ses
états de barre de titre, son ombre, ses timers, sa saisie UTF-16/IME et son pont
UI Automation. L’OS conserve ses services globaux, comme le presse-papiers.

`state.rs` associe le handle natif au contexte de la fenêtre. Un scope choisit
le contexte au début du message et restaure le précédent à la fin : si Windows
appelle B pendant un traitement de A, le retour reprend bien le contexte A.
Le registre n’est pas emprunté pendant les callbacks. Le contexte de création
couvre aussi les messages synchrones émis par `CreateWindowExW`.

`WM_DESTROY` désarme les timers et déconnecte l’accessibilité et l’ombre de la
fenêtre concernée. À `WM_NCDESTROY`, le contexte est retiré du registre et ses
ressources sont libérées après la fin des scopes qui l’utilisent encore. La
fermeture réveille la boucle sans déposer de `WM_QUIT` pour chaque fenêtre.
Cela permet à une deuxième fenêtre de continuer et évite un `WM_QUIT` résiduel
si l’application relance une session sur le même thread.

Le décodeur GDI+ et l’appartement COM restent vivants jusqu’au nettoyage de la
session. Une seconde boucle imbriquée sur le même thread est refusée. Appeler
l’entrée du moteur sur le thread principal prépare la portabilité future ;
les applications de fenêtre ne sont pas tenues d’implémenter `Send`.

Les règles natives utilisées ici sont documentées par Microsoft :
[destruction finale](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-ncdestroy),
[messages envoyés pendant GetMessage](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessagew)
et [réveil par PostMessageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postmessagew).

## Validation et limites

Neuf tests natifs créent des fenêtres cachées et vérifient l’isolation des
compteurs, des thèmes et du décodage UTF-16, la réentrance entre fenêtres, les
ouvertures/fermetures différées, les politiques de sortie, les erreurs et la
libération des applications. Deux scénarios envoient réellement un message
depuis un autre thread pendant la boucle : fermeture de la dernière fenêtre
et demande de sortie sans reconstruction de la vue. Un secours borné détecte
une régression du réveil au lieu de suspendre la suite.

Un test de la démo valide les arbres et les identifiants des fenêtres produites.
La suite compte 154 tests au total : 153 dans la bibliothèque et un dans la démo.
La recette visuelle simultanée des fenêtres, de leurs ombres, de leurs champs
de saisie et du lecteur d’écran reste à faire ; les tests cachés ne la remplacent
pas. Aucun gain de RAM ou de temps d’image global n’est annoncé à cette étape.

Le [service de tâches de fond](tasks.md) fournit maintenant des résultats
associés à chaque fenêtre, avec annulation à la fermeture. L’API de fenêtres
ne fournit pas encore les relations propriétaire/dialogue natif, l’annulation d’une fermeture pour
confirmer une sauvegarde, le docking ou les backends macOS/Linux. Chaque fenêtre
possède actuellement son renderer ; le partage de ressources graphiques entre
fenêtres reste une optimisation à mesurer.

Les tâches préparent la validation d’un premier écran RustMusic et du chargement
de ses ressources, avec des mesures de mémoire et de temps d’affichage.
