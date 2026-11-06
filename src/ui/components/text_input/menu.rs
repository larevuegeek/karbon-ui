//! Menu contextuel intégré du champ de saisie : Couper, Copier, Coller, Tout
//! sélectionner. Ouvert par le clic droit dans le champ, ou depuis le clavier (Maj+F10,
//! touche Menu) quand le champ a le focus ; dessiné et piloté par le moteur comme la
//! liste du `Select`, donc identique sur toutes les plateformes. Une application qui pose
//! `context_menu` sur le champ ou l'un de ses parents le remplace par le sien.
//!
//! Les entrées impossibles sont grisées (Couper et Coller en lecture seule, Couper et
//! Copier sans sélection ou dans un champ masqué). Choisir une entrée envoie la commande
//! d'édition correspondante au champ, qui demande au besoin le presse-papiers au backend.
use super::*;
use crate::ui::{
    layers::AnchoredLayer,
    runtime::{Cursor, Key, UiEvent},
};

/// Libellés du menu contextuel d'un champ, dans l'ordre : couper, copier, coller, tout
/// sélectionner.
pub type EditMenuLabels = [&'static str; 4];
/// Libellés par défaut (anglais, comme l'API) ; `TextInput::menu_labels` les remplace.
pub const DEFAULT_MENU_LABELS: EditMenuLabels = ["Cut", "Copy", "Paste", "Select all"];

const ITEM_HEIGHT: f32 = 30.;
const PADDING: f32 = 4.;
const ITEMS: [EditCommand; 4] = [
    EditCommand::Cut,
    EditCommand::Copy,
    EditCommand::Paste,
    EditCommand::SelectAll,
];

/// Entrées possibles selon l'état du champ.
fn available(input: &TextInput, snapshot: &InputSnapshot) -> [bool; 4] {
    let selected = !snapshot.selection.is_empty();
    [
        selected && !input.read_only && !input.masked,
        selected && !input.masked,
        !input.read_only,
        !snapshot.value.is_empty() && snapshot.selection != (0..snapshot.value.len()),
    ]
}

struct OpenMenu {
    /// Champ visé.
    id: ElementId,
    /// Rectangle de contenu du champ à l'ouverture : s'il bouge, le menu se ferme.
    content: RectF,
    bounds: RectF,
    viewport: RectF,
    labels: EditMenuLabels,
    enabled: [bool; 4],
    active: Option<usize>,
    hover: Option<usize>,
    pressed: Option<usize>,
}
impl OpenMenu {
    fn row(&self, index: usize) -> RectF {
        RectF::new(
            self.bounds.left + PADDING,
            self.bounds.top + PADDING + index as f32 * ITEM_HEIGHT,
            (layout::width(self.bounds) - 2. * PADDING).max(0.),
            ITEM_HEIGHT,
        )
    }
    /// Entrée activable sous `p`.
    fn item(&self, p: PointF) -> Option<usize> {
        (0..ITEMS.len()).find(|&i| self.enabled[i] && layout::contains(self.row(i), p.x, p.y))
    }
    fn navigate(&mut self, key: Key) {
        let enabled: Vec<usize> = (0..ITEMS.len()).filter(|&i| self.enabled[i]).collect();
        if enabled.is_empty() {
            self.active = None;
            return;
        }
        let current = self
            .active
            .and_then(|i| enabled.iter().position(|v| *v == i));
        let index = match key {
            Key::Home => 0,
            Key::End => enabled.len() - 1,
            Key::Up => current.map_or(enabled.len() - 1, |i| {
                (i + enabled.len() - 1) % enabled.len()
            }),
            _ => current.map_or(0, |i| (i + 1) % enabled.len()),
        };
        self.active = Some(enabled[index]);
    }
}

/// Où ouvrir le menu d'un champ.
pub(crate) struct MenuAnchor {
    /// Rectangle de contenu du champ (le menu se ferme s'il change).
    pub content: RectF,
    /// Point du pointeur (rectangle d'un pixel) ou caret (clavier) : le menu se place
    /// dessous.
    pub anchor: RectF,
    /// Zone disponible pour le menu.
    pub viewport: RectF,
    /// Ouverture depuis le clavier : la première entrée possible est active.
    pub keyboard: bool,
}
#[derive(Default)]
pub(crate) struct MenuResponse {
    pub handled: bool,
    pub repaint: bool,
    pub capture: bool,
    pub release: bool,
    /// Entrée choisie : commande à envoyer au champ.
    pub command: Option<EditCommand>,
}
impl MenuResponse {
    fn handled() -> Self {
        Self {
            handled: true,
            ..Self::default()
        }
    }
    fn repaint() -> Self {
        Self {
            handled: true,
            repaint: true,
            ..Self::default()
        }
    }
}

/// Menu contextuel ouvert (au plus un par fenêtre) et capture du clic qui le ferme.
#[derive(Default)]
pub(crate) struct EditMenu {
    open: Option<OpenMenu>,
    /// Appui en cours dans le menu.
    captured: bool,
    /// Clic hors du menu : fermé, et le relâchement n'atteint pas ce qu'il y a dessous.
    dismiss_capture: bool,
}
impl EditMenu {
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }
    pub fn captured(&self) -> bool {
        self.captured || self.dismiss_capture
    }
    pub fn cursor(&self) -> Option<Cursor> {
        self.open
            .as_ref()
            .map(|o| {
                if o.hover.is_some() {
                    Cursor::Hand
                } else {
                    Cursor::Arrow
                }
            })
            .or(self.dismiss_capture.then_some(Cursor::Arrow))
    }
    /// Rangées du menu ouvert et leur disponibilité (tests).
    #[cfg(test)]
    pub fn rows(&self) -> Vec<(RectF, bool)> {
        self.open
            .as_ref()
            .map(|o| (0..ITEMS.len()).map(|i| (o.row(i), o.enabled[i])).collect())
            .unwrap_or_default()
    }
    pub fn close(&mut self) -> MenuResponse {
        let response = MenuResponse {
            handled: true,
            repaint: self.open.is_some() || self.captured(),
            release: self.captured(),
            ..MenuResponse::default()
        };
        self.open = None;
        self.captured = false;
        self.dismiss_capture = false;
        response
    }
    /// Ouvre le menu du champ `id` à l'emplacement `at`.
    pub fn open(
        &mut self,
        id: ElementId,
        input: &TextInput,
        snapshot: &InputSnapshot,
        at: MenuAnchor,
    ) -> MenuResponse {
        let MenuAnchor {
            content,
            anchor,
            viewport,
            keyboard,
        } = at;
        let enabled = available(input, snapshot);
        let longest = input
            .menu_labels
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0) as f32;
        let layer = AnchoredLayer::place(
            anchor,
            SizeF {
                width: (longest * 8. + 48.).clamp(160., 320.),
                height: ITEMS.len() as f32 * ITEM_HEIGHT + 2. * PADDING,
            },
            viewport,
        );
        if layout::height(layer.bounds) < ITEM_HEIGHT + 2. * PADDING
            || layout::width(layer.bounds) < 40.
        {
            return MenuResponse::handled();
        }
        let release = self.captured();
        self.captured = false;
        self.dismiss_capture = false;
        self.open = Some(OpenMenu {
            id,
            content,
            bounds: layer.bounds,
            viewport,
            labels: input.menu_labels,
            enabled,
            active: keyboard
                .then(|| (0..ITEMS.len()).find(|&i| enabled[i]))
                .flatten(),
            hover: None,
            pressed: None,
        });
        MenuResponse {
            handled: true,
            repaint: true,
            release,
            ..MenuResponse::default()
        }
    }
    /// Après layout : le menu ne survit pas à son champ (retiré, désactivé, déplacé) ni à
    /// la perte de son focus.
    pub fn sync(&mut self, root: &Element, focused: Option<ElementId>) -> MenuResponse {
        let valid = self.open.as_ref().is_none_or(|o| {
            focused == Some(o.id)
                && root.find(o.id).is_some_and(|n| {
                    n.widget
                        .as_ref()
                        .and_then(|w| w.text_input())
                        .is_some_and(|i| i.enabled)
                        && super::super::select::same_rect(layout::content_rect(n), o.content)
                })
        });
        if valid {
            MenuResponse::default()
        } else {
            self.close()
        }
    }
    fn choose(&mut self, index: usize) -> MenuResponse {
        let mut response = self.close();
        response.command = Some(ITEMS[index]);
        response
    }
    pub fn event(&mut self, event: &UiEvent) -> MenuResponse {
        if self.dismiss_capture {
            match event {
                UiEvent::PointerUp(_) | UiEvent::Cancel => return self.close(),
                UiEvent::FocusChanged(false) => {
                    let mut r = self.close();
                    r.handled = false;
                    return r;
                }
                UiEvent::PointerMove(_) | UiEvent::PointerDown(_) => {
                    return MenuResponse::handled();
                }
                _ => {}
            }
        }
        let Some(o) = self.open.as_mut() else {
            return MenuResponse::default();
        };
        match event {
            UiEvent::FocusChanged(false) | UiEvent::KeyDown { key: Key::Tab, .. } => {
                let mut r = self.close();
                r.handled = false;
                r
            }
            UiEvent::Cancel
            | UiEvent::KeyDown {
                key: Key::Escape, ..
            } => self.close(),
            // Un raccourci d'édition pendant que le menu est ouvert vaut pour le champ.
            UiEvent::Edit(_) => {
                let mut r = self.close();
                r.handled = false;
                r
            }
            UiEvent::Scroll { position, .. } => {
                if layout::contains(o.bounds, position.x, position.y) {
                    MenuResponse::handled()
                } else {
                    let mut r = self.close();
                    r.handled = false;
                    r
                }
            }
            UiEvent::PointerDown(p) => {
                if !layout::contains(o.bounds, p.x, p.y) {
                    let mut r = self.close();
                    self.dismiss_capture = true;
                    r.capture = true;
                    r.release = false;
                    return r;
                }
                self.captured = true;
                o.pressed = o.item(*p);
                if o.pressed.is_some() {
                    o.active = o.pressed;
                    o.hover = o.pressed;
                }
                MenuResponse {
                    capture: true,
                    ..MenuResponse::repaint()
                }
            }
            UiEvent::PointerMove(p) => {
                let before = (o.hover, o.active);
                o.hover = o.item(*p);
                if o.hover.is_some() {
                    o.active = o.hover;
                }
                MenuResponse {
                    repaint: before != (o.hover, o.active),
                    ..MenuResponse::handled()
                }
            }
            UiEvent::PointerUp(p) => {
                let hit = o.item(*p);
                if self.captured && o.pressed.is_some() && o.pressed == hit {
                    return self.choose(hit.unwrap());
                }
                if self.captured && !layout::contains(o.bounds, p.x, p.y) {
                    return self.close();
                }
                o.pressed = None;
                let release = self.captured;
                self.captured = false;
                MenuResponse {
                    release,
                    ..MenuResponse::repaint()
                }
            }
            UiEvent::PointerLeave => {
                o.hover = None;
                MenuResponse::repaint()
            }
            UiEvent::KeyDown { key, repeat, .. } => {
                match key {
                    Key::Enter | Key::Space if !repeat => {
                        if let Some(index) = o.active {
                            return self.choose(index);
                        }
                    }
                    Key::Up | Key::Down | Key::Home | Key::End => {
                        o.hover = None;
                        o.navigate(*key);
                    }
                    _ => {}
                }
                MenuResponse::repaint()
            }
            // Texte tapé, touches relâchées et composition : absorbés tant que le menu est
            // ouvert, comme pour la liste du `Select`.
            UiEvent::KeyUp(_)
            | UiEvent::Text(_)
            | UiEvent::CompositionStart
            | UiEvent::CompositionUpdate { .. }
            | UiEvent::CompositionCommit(_)
            | UiEvent::CompositionEnd => MenuResponse::handled(),
            _ => MenuResponse::default(),
        }
    }
    pub fn paint(&self, theme: Theme, scene: &mut Scene) {
        let Some(o) = &self.open else {
            return;
        };
        let r = o.bounds;
        scene.commands.push(DrawCommand::PushClip(o.viewport));
        scene.fill(
            RectF::new(r.left, r.top + 3., layout::width(r), layout::height(r)),
            8.,
            Color::rgba(0, 0, 0, 55),
        );
        scene.fill(r, 8., theme.surface);
        scene.commands.push(DrawCommand::Stroke {
            rect: r,
            radius: 8.,
            width: 1.,
            color: theme.border,
        });
        for (index, label) in o.labels.iter().enumerate() {
            let row = o.row(index);
            let enabled = o.enabled[index];
            let active = enabled && o.active == Some(index);
            if active {
                scene.fill(
                    row,
                    4.,
                    if o.pressed == Some(index) {
                        theme.pressed
                    } else {
                        theme.hover
                    },
                );
            }
            scene.text_single_line(
                RectF::new(
                    row.left + 12.,
                    row.top + 5.,
                    (layout::width(row) - 24.).max(0.),
                    22.,
                ),
                *label,
                14.,
                false,
                if enabled { theme.text } else { theme.muted },
            );
            if active {
                scene.commands.push(DrawCommand::Stroke {
                    rect: RectF::new(
                        row.left + 1.,
                        row.top + 1.,
                        (layout::width(row) - 2.).max(0.),
                        ITEM_HEIGHT - 2.,
                    ),
                    radius: 3.,
                    width: 1.,
                    color: theme.accent,
                });
            }
        }
        scene.commands.push(DrawCommand::PopClip);
    }
}
