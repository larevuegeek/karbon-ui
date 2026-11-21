//! Barre de menus dans la fenêtre (KUI-032), menu contextuel et entrées de menu serrées
//! (KUI-034) :
//! sous-menus, cases et choix exclusifs, raccourcis affichés, entrées grisées.
mod context;
mod model;
mod state;
#[cfg(test)]
mod tests;
mod view;
mod widgets;

pub use context::context_menu;
pub use model::{Check, Menu, MenuEntry};
pub use state::{MenuBarMsg, MenuBarState, MenuReply};
pub use view::MenuBar;
