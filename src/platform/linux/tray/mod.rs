//! Icône de la zone de notification sous Linux : protocole `StatusNotifierItem` sur le bus
//! de session (KDE Plasma, Xfce, Cinnamon, MATE, LXQt, Budgie ; GNOME avec l'extension
//! AppIndicator, présente par défaut sous Ubuntu), menu exporté par `com.canonical.dbusmenu`.
//! Icône de la fenêtre sous X11 (`_NET_WM_ICON`, tirée de `WindowOptions::icon`).
//!
//! Chaque icône a son thread et sa connexion privée au bus de session : elle prend un nom
//! `org.kde.StatusNotifierItem-<pid>-<n>`, sert ses deux objets (`/StatusNotifierItem`,
//! `/MenuBar`) et s'annonce au `org.kde.StatusNotifierWatcher` (de nouveau quand il
//! apparaît). Clic et choix de menu reviennent à la fenêtre par un canal et le réveil de la
//! boucle ; `Session::tasks` les remet à `Application::tray_event`. Fermer la connexion
//! (fenêtre fermée, icône retirée) libère le nom : l'hôte retire l'icône.
mod dbusmenu;
mod item;
mod pixmap;
mod service;

pub(in crate::platform::linux) use pixmap::window_icon;
pub(in crate::platform::linux) use service::Tray;

#[cfg(test)]
mod tests;
