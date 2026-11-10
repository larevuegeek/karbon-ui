//! Mesures de texte gardées d'une image à l'autre.
//!
//! Pendant un redimensionnement ou une transition de largeur, la place offerte aux
//! textes change à chaque image : le moteur de texte du backend (Core Text,
//! DirectWrite) recrée alors une mise en page pour chacun, alors que la plupart tiennent
//! sur la même ligne qu'avant. Les coupures de ligne sont gloutonnes : une mesure faite
//! à la largeur `W`, dont la ligne la plus large mesure `w`, vaut pour toute largeur
//! entre `w` et `W` (chaque ligne y tient encore, et le mot suivant ne tenait déjà pas
//! dans `W`). Il en va de même pour l'ellipse d'une ligne seule (la partie gardée
//! tient, la suivante ne tenait pas). Une demi-unité de marge au-dessus de `w` écarte
//! les arrondis du moteur de texte. La justification, le centrage et `line-clamp` font
//! dépendre la taille de la largeur exacte : le centrage place chaque ligne au milieu
//! de `W` et la largeur rendue garde l'arrondi de ce calcul (DirectWrite, à un ULP
//! près) ; avec `line-clamp`, la dernière ligne gardée est mesurée après l'ellipse,
//! plus étroite que la ligne coupée qu'elle remplace, qui peut ne plus tenir un peu
//! sous `W`. Ces textes ne sont repris qu'à la même largeur.
mod cached;
#[cfg(test)]
mod tests;
pub(crate) use cached::*;
