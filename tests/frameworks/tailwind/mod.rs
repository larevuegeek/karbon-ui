//! Utilitaires Tailwind dans les templates (`@import "tailwindcss"` dans la feuille) :
//! mise en page, couleurs et variantes, effets, curseurs et défilement.
mod effects;
mod interaction;
mod placement;
mod variants;
css_file!(mod css, "tailwind.css");
mod fixture;
use fixture::*;
