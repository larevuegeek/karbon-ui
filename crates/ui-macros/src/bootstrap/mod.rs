//! Comportements Bootstrap déclaratifs (`data-bs-toggle`, `data-bs-dismiss`) réécrits
//! en composants natifs avant la génération d'un template.
//!
//! - `collapse` : le déclencheur émet `BootstrapAction::Toggle` ; la cible reçoit la
//!   classe `show` (et le déclencheur `collapsed`) selon l'état ; `data-bs-parent`
//!   forme un accordéon. La cible porte `data-bs-reveal="collapsing"` : sa hauteur
//!   s'anime entre 0 et sa hauteur naturelle au minutage de `.collapsing`
//!   (`Element::reveal`).
//! - `tab` / `pill` / `list` : `Show` ; le déclencheur et le volet reçoivent `active`
//!   (et `show` pour le volet), les autres volets du même `.nav` les perdent.
//! - `dropdown` : le `.dropdown-menu` qui suit le déclencheur s'ouvre dans un
//!   `<k-popover menu>` ancré au déclencheur (fermeture au clic extérieur, Échap).
//! - `modal` : la `.modal` devient un `<dialog>` natif (focus piégé, Échap, voile).
//! - `offcanvas` : panneau `.offcanvas.show` et voile `.offcanvas-backdrop` qui ferme.
//! - `tooltip` / `popover` : `data-bs-title` devient l'info-bulle native (`title`).
//! - `data-bs-dismiss="alert" | "modal" | "offcanvas"` : ferme l'ancêtre concerné.
mod plan;
mod query;
mod rewriter;
pub use plan::*;
