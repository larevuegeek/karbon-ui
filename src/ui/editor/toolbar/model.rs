//! Ce que montre une barre d'outils : boutons, bascules, choix exclusifs, séparateurs ;
//! décrits à chaque vue par l'application.
use crate::ui::vector::VectorIcon;
use std::sync::Arc;

/// Ce qu'est un élément de la barre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// Un bouton qui fait une chose.
    Button,
    /// Une option qu'on allume et qu'on éteint (`true` : allumée).
    Toggle(bool),
    /// Un choix parmi les choix voisins (déplacer, tourner, agrandir).
    Choice(bool),
    /// Un trait entre deux groupes.
    Separator,
}

/// An item of a [`Toolbar`](super::Toolbar): a button, an option turned on or off, one
/// choice among its neighbours, or a separator. A button shows its icon (or its short
/// text without icon); its tooltip gives its name, its shortcut and its description.
/// When the bar is too narrow, the last items go, in order, to the overflow menu (« » »),
/// where they show their name, check mark or dot, and shortcut.
pub struct ToolItem<M> {
    pub(super) kind: Kind,
    pub(super) name: Arc<str>,
    pub(super) text: Arc<str>,
    pub(super) icon: Option<Arc<VectorIcon>>,
    pub(super) shortcut: Arc<str>,
    pub(super) description: Arc<str>,
    pub(super) message: Option<M>,
    pub(super) enabled: bool,
    pub(super) width: Option<f32>,
}

impl<M> ToolItem<M> {
    fn new(kind: Kind, name: Arc<str>, message: Option<M>) -> Self {
        Self {
            kind,
            text: Arc::clone(&name),
            name,
            icon: None,
            shortcut: "".into(),
            description: "".into(),
            message,
            enabled: true,
            width: None,
        }
    }

    /// A button named `name` that sends `message`.
    pub fn button(name: impl Into<Arc<str>>, message: M) -> Self {
        Self::new(Kind::Button, name.into(), Some(message))
    }

    /// An option named `name`, turned on when `on`; a click sends `message`.
    pub fn toggle(name: impl Into<Arc<str>>, on: bool, message: M) -> Self {
        Self::new(Kind::Toggle(on), name.into(), Some(message))
    }

    /// One choice among the choices next to it, `selected` or not; a click sends
    /// `message`.
    pub fn choice(name: impl Into<Arc<str>>, selected: bool, message: M) -> Self {
        Self::new(Kind::Choice(selected), name.into(), Some(message))
    }

    /// A line between two groups of items.
    pub fn separator() -> Self {
        Self::new(Kind::Separator, "".into(), None)
    }

    /// Show `icon` instead of the text.
    pub fn icon(mut self, icon: Arc<VectorIcon>) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The short text shown on the button when it has no icon (its name by default).
    pub fn text(mut self, text: impl Into<Arc<str>>) -> Self {
        self.text = text.into();
        self
    }

    /// The shortcut, shown in the tooltip and in the overflow menu. Display only.
    pub fn shortcut(mut self, shortcut: impl Into<Arc<str>>) -> Self {
        self.shortcut = shortcut.into();
        self
    }

    /// What the item does, on the second line of its tooltip.
    pub fn description(mut self, text: impl Into<Arc<str>>) -> Self {
        self.description = text.into();
        self
    }

    /// Grey the item out when `enabled` is `false`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Width of the item in logical pixels: square buttons with an icon are as wide as
    /// they are high; a text button is given the width of its text in the font of the
    /// bar (`8 + 0.6 × size` per character, rounded), unless this says otherwise.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width.max(1.));
        self
    }
}
