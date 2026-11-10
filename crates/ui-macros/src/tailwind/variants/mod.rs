//! Variantes Tailwind (`hover:`, `md:`, `@3xl:`, `group-hover:`, `dark:`…) traduites en
//! prédicats `StyleState`, requêtes `@media`/`@container` et pseudo-éléments.
mod apply;
mod names;
mod selectors;
pub use apply::*;
