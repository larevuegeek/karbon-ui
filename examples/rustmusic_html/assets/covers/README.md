# Pochettes de la démonstration

Images générées (disque sur un dégradé dont les teintes dérivent du nom du fichier),
une par album ou podcast de l'accueil, aux noms et dimensions des pochettes d'origine :
la mise en page de la démonstration est celle de RustMusic, les visuels ne sont pas
ceux des albums. Les vraies pochettes, sous droit d'auteur, ont été retirées avant la
publication du dépôt ; les exports de référence ont été rebasés à cette occasion.

`radio_logo.png` est décodé à l'exécution (`assets.rs`), les autres sont embarquées par
`include_bytes!` selon leur extension (`.png`, `.jpg`).
