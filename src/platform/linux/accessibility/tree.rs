//! Arbre exposé à AT-SPI : application (racine), une fenêtre (`frame`) par fenêtre du
//! moteur, et les éléments de l'instantané accessible de chaque fenêtre. Rôles, noms de
//! rôles, états, attributs et interfaces d'un objet.
use crate::{
    ui::{
        element::{CheckState, ChoiceState, ElementId, Role},
        runtime::{AccessibilityAction, AccessibleElement},
    },
    window::WindowId,
};
use std::collections::HashMap;

/// Chemin de l'application (racine imposée par AT-SPI).
pub(super) const ROOT: &str = "/org/a11y/atspi/accessible/root";
/// Référence nulle (aucun objet).
pub(super) const NULL: &str = "/org/a11y/atspi/null";
const PREFIX: &str = "/org/a11y/atspi/accessible/";

/// Objet AT-SPI.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Object {
    Root,
    Window(WindowId),
    Element(WindowId, ElementId),
}
impl Object {
    pub fn parse(path: &str) -> Option<Self> {
        if path == ROOT {
            return Some(Self::Root);
        }
        let rest = path.strip_prefix(PREFIX)?.strip_prefix('w')?;
        let (window, element) = match rest.split_once("/e") {
            Some((w, e)) => (w, Some(e)),
            None => (rest, None),
        };
        let window = WindowId(window.parse().ok()?);
        Some(match element {
            Some(e) => Self::Element(window, ElementId(e.parse().ok()?)),
            None => Self::Window(window),
        })
    }
    pub fn path(self) -> String {
        match self {
            Self::Root => ROOT.into(),
            Self::Window(w) => format!("{PREFIX}w{}", w.0),
            Self::Element(w, e) => format!("{PREFIX}w{}/e{}", w.0, e.0),
        }
    }
}
/// Instantané accessible d'une fenêtre.
pub(super) struct Tree {
    pub version: (u64, u64, u64, u64, Option<ElementId>),
    pub title: String,
    pub size: (i32, i32),
    pub elements: Vec<AccessibleElement>,
    index: HashMap<ElementId, usize>,
    /// Enfants de chaque élément (`None` : éléments de premier niveau), dans l'ordre.
    children: HashMap<Option<ElementId>, Vec<ElementId>>,
}
impl Tree {
    pub fn new(
        version: (u64, u64, u64, u64, Option<ElementId>),
        title: &str,
        size: (i32, i32),
        elements: Vec<AccessibleElement>,
    ) -> Self {
        let index = elements
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id, i))
            .collect::<HashMap<_, _>>();
        let mut children: HashMap<Option<ElementId>, Vec<ElementId>> = HashMap::new();
        for e in &elements {
            // Un parent absent de l'instantané rattache l'élément à la fenêtre.
            let parent = e.parent.filter(|p| index.contains_key(p));
            children.entry(parent).or_default().push(e.id);
        }
        Self {
            version,
            title: title.into(),
            size,
            elements,
            index,
            children,
        }
    }
    pub fn get(&self, id: ElementId) -> Option<&AccessibleElement> {
        self.index.get(&id).map(|i| &self.elements[*i])
    }
    pub fn children(&self, parent: Option<ElementId>) -> &[ElementId] {
        self.children.get(&parent).map_or(&[], Vec::as_slice)
    }
    /// Parent d'un élément dans l'arbre exposé (`None` : la fenêtre).
    pub fn parent(&self, id: ElementId) -> Option<ElementId> {
        self.get(id)?.parent.filter(|p| self.index.contains_key(p))
    }
    pub fn focused(&self) -> Option<ElementId> {
        self.elements.iter().find(|e| e.focused).map(|e| e.id)
    }
}
/// Rôle AT-SPI (`AtspiRole`) et son nom.
pub(super) fn role(e: &AccessibleElement) -> (u32, &'static str) {
    match e.role {
        Role::Text => (29, "label"),
        Role::Button => (43, "push button"),
        Role::Separator => (50, "separator"),
        Role::ScrollArea => (49, "scroll pane"),
        Role::List => (31, "list"),
        Role::TextInput if e.text.as_ref().is_some_and(|t| t.multiline) => (61, "text"),
        Role::TextInput => (79, "entry"),
        Role::Checkbox => (7, "check box"),
        // Pas de rôle « switch » dans at-spi2-core 2.52 : bouton bascule.
        Role::Switch => (62, "toggle button"),
        Role::RadioButton => (44, "radio button"),
        Role::RadioGroup => (99, "grouping"),
        Role::Slider => (51, "slider"),
        Role::Select => (11, "combo box"),
        Role::Dialog => (16, "dialog"),
        Role::Popover => (39, "panel"),
        Role::Menu => (41, "popup menu"),
        Role::MenuItem => (35, "menu item"),
        Role::Link => (88, "link"),
        Role::Image => (27, "image"),
        Role::Heading => (83, "heading"),
        Role::ListItem => (32, "list item"),
        Role::Group => (85, "section"),
        Role::Main | Role::Navigation | Role::Complementary => (110, "landmark"),
        Role::Banner => (71, "header"),
        Role::ContentInfo => (72, "footer"),
        Role::Custom => (39, "panel"),
    }
}
pub(super) const ROLE_APPLICATION: (u32, &str) = (75, "application");
pub(super) const ROLE_FRAME: (u32, &str) = (23, "frame");
/// États AT-SPI (`AtspiStateType`, numéros des bits).
pub(super) mod state {
    pub const ACTIVE: u32 = 1;
    pub const CHECKED: u32 = 4;
    pub const COLLAPSED: u32 = 5;
    pub const EDITABLE: u32 = 7;
    pub const ENABLED: u32 = 8;
    pub const EXPANDABLE: u32 = 9;
    pub const EXPANDED: u32 = 10;
    pub const FOCUSABLE: u32 = 11;
    pub const FOCUSED: u32 = 12;
    pub const HAS_TOOLTIP: u32 = 13;
    pub const HORIZONTAL: u32 = 14;
    pub const MODAL: u32 = 16;
    pub const MULTI_LINE: u32 = 17;
    pub const RESIZABLE: u32 = 21;
    pub const SENSITIVE: u32 = 24;
    pub const SHOWING: u32 = 25;
    pub const SINGLE_LINE: u32 = 26;
    pub const VERTICAL: u32 = 29;
    pub const VISIBLE: u32 = 30;
    pub const INDETERMINATE: u32 = 32;
    pub const SELECTABLE_TEXT: u32 = 38;
    pub const CHECKABLE: u32 = 41;
    pub const HAS_POPUP: u32 = 42;
    pub const READ_ONLY: u32 = 43;
}
/// Ensemble d'états en deux mots de 32 bits.
pub(super) fn states(bits: &[u32]) -> [u32; 2] {
    let mut set = [0u32; 2];
    for &bit in bits {
        set[(bit / 32) as usize] |= 1 << (bit % 32);
    }
    set
}
/// États d'un élément.
pub(super) fn element_states(e: &AccessibleElement) -> [u32; 2] {
    use state::*;
    let mut bits = Vec::new();
    if e.enabled {
        bits.extend([ENABLED, SENSITIVE]);
    }
    if !e.offscreen {
        bits.extend([VISIBLE, SHOWING]);
    }
    if e.focusable {
        bits.push(FOCUSABLE);
    }
    if e.focused {
        bits.push(FOCUSED);
    }
    if !e.help.is_empty() {
        bits.push(HAS_TOOLTIP);
    }
    match e.choice {
        Some(ChoiceState::Toggle(s)) => {
            bits.push(CHECKABLE);
            match s {
                CheckState::On => bits.push(CHECKED),
                CheckState::Mixed => bits.push(INDETERMINATE),
                CheckState::Off => {}
            }
        }
        Some(ChoiceState::Radio(on)) => {
            bits.push(CHECKABLE);
            if on {
                bits.push(CHECKED);
            }
        }
        None => {}
    }
    if let Some(text) = &e.text {
        bits.push(SELECTABLE_TEXT);
        bits.push(if text.multiline {
            MULTI_LINE
        } else {
            SINGLE_LINE
        });
        if !e.read_only && e.enabled {
            bits.push(EDITABLE);
        }
    }
    if e.read_only {
        bits.push(READ_ONLY);
    }
    if let Some(range) = &e.range {
        bits.push(if range.axis == crate::ui::element::Axis::Vertical {
            VERTICAL
        } else {
            HORIZONTAL
        });
    }
    let expanded = e.select.as_ref().map(|s| s.expanded).or(e.expanded);
    if let Some(expanded) = expanded {
        bits.extend([
            EXPANDABLE,
            HAS_POPUP,
            if expanded { EXPANDED } else { COLLAPSED },
        ]);
    }
    if e.role == Role::Dialog {
        bits.push(MODAL);
    }
    states(&bits)
}
/// Attributs d'un élément : repères de page (`xml-roles`, lus par Orca).
pub(super) fn attributes(e: &AccessibleElement) -> Vec<(&'static str, &'static str)> {
    let landmark = match e.role {
        Role::Main => Some("main"),
        Role::Navigation => Some("navigation"),
        Role::Complementary => Some("complementary"),
        Role::Banner => Some("banner"),
        Role::ContentInfo => Some("contentinfo"),
        _ => None,
    };
    let mut out = vec![("toolkit", "karbon-ui")];
    if let Some(role) = landmark {
        out.push(("xml-roles", role));
    }
    out
}
/// Action d'un appui selon le rôle, et son nom AT-SPI.
pub(super) fn press(e: &AccessibleElement) -> Option<(&'static str, AccessibilityAction)> {
    if !e.enabled {
        return None;
    }
    Some(match e.role {
        Role::Checkbox | Role::Switch => ("toggle", AccessibilityAction::Toggle(e.id)),
        Role::RadioButton => ("click", AccessibilityAction::SelectChoice(e.id)),
        Role::Select => (
            "press",
            AccessibilityAction::ExpandSelect {
                id: e.id,
                expanded: !e.select.as_ref().is_some_and(|s| s.expanded),
            },
        ),
        Role::TextInput => ("activate", AccessibilityAction::Focus(e.id)),
        _ if e.invokable => ("click", AccessibilityAction::Invoke(e.id)),
        _ if e.focusable => ("activate", AccessibilityAction::Focus(e.id)),
        _ => return None,
    })
}
/// Interfaces AT-SPI d'un élément.
pub(super) fn interfaces(e: &AccessibleElement) -> Vec<&'static str> {
    let mut out = vec!["org.a11y.atspi.Accessible", "org.a11y.atspi.Component"];
    if press(e).is_some() {
        out.push("org.a11y.atspi.Action");
    }
    if e.range.is_some() {
        out.push("org.a11y.atspi.Value");
    }
    if e.text.is_some() {
        out.push("org.a11y.atspi.Text");
        if !e.read_only {
            out.push("org.a11y.atspi.EditableText");
        }
    }
    out
}
