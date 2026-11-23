# Presse-papiers de l'application (`karbon_ui::clipboard`)

KUI-048. Les champs de saisie gardent leur chemin ; ces fonctions sont pour le reste :
copier des objets d'un niveau à l'autre ou d'une instance de l'éditeur à une autre, le
texte du journal.

```rust
use karbon_ui::clipboard::{self, ClipboardContent};

clipboard::write(&ClipboardContent::text("Caisse, Tonneau").with("karbon-editor/objets", bytes))?;
let mine = clipboard::read_format("karbon-editor/objets")?; // Some(octets) si c'est le nôtre
let text = clipboard::read_text()?;                        // le texte, pour tout programme
```

- Le texte (`CF_UNICODETEXT`) et des formats nommés par l'application
  (`RegisterClipboardFormatW`) sont écrits **ensemble** et remplacent le contenu. Un
  bloc d'un format garde sa longueur exacte (en-tête de huit octets).
- Le presse-papiers est ouvert au nom d'une fenêtre « message seulement » créée pour
  l'appel (sans propriétaire, `SetClipboardData` peut échouer), avec dix essais espacés
  de 10 ms s'il est tenu par un autre programme.
- Limites : 64 formats, 256 Mio. Windows seulement : ailleurs, `Error::Unsupported`.
- Code : `src/clipboard/`, `src/platform/windows/app_clipboard.rs`.

Contrôles : contenu vérifié (témoins : nom vide, caractère nul, trop de formats) ; bloc
relu à sa longueur exacte (témoins : trop court, longueur impossible) ; à la demande
(`--ignored` : il écrit le presse-papiers de la session puis y remet le texte d'avant),
texte et format propre relus, un texte seul efface le format. L'éditeur de karbon-engine
s'en sert pour copier le journal et la valeur d'un champ (`editor/src/clipboard.rs`).
