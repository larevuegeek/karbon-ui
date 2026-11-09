//! Événements d'élément : survol, appui, focus et double clic (`onhover`, `onpress`,
//! `onfocus`, `ondblclick`). Un gestionnaire voit aussi ce qui arrive à ses descendants,
//! comme `mouseenter` et `focusin` : survoler un bouton, c'est survoler sa carte.
use super::*;
use crate::application::Message;
use std::time::Duration;

/// Écart maximal entre les deux appuis d'un double clic.
const DOUBLE_CLICK: Duration = Duration::from_millis(500);

#[derive(Clone, Copy)]
enum Kind {
    Hover,
    Press,
    Focus,
}
const KINDS: [Kind; 3] = [Kind::Hover, Kind::Press, Kind::Focus];

/// Éléments dont les gestionnaires ont été prévenus : survolés, appuyés, focalisés.
#[derive(Default)]
pub(super) struct ListenerState {
    targets: [Option<ElementId>; 3],
    active: [Vec<ElementId>; 3],
    last_press: Option<(ElementId, Instant)>,
}

/// Gestionnaire de `node` pour `kind`.
fn handler(node: &Element, kind: Kind) -> Option<&crate::application::Handler<bool>> {
    let listeners = node.listeners.as_deref()?;
    match kind {
        Kind::Hover => listeners.hover.as_ref(),
        Kind::Press => listeners.press.as_ref(),
        Kind::Focus => listeners.focus.as_ref(),
    }
}
/// Chemin de la racine à `id`, inclus ; vide si l'élément n'est plus dans l'arbre.
fn path<'a>(node: &'a Element, id: ElementId, out: &mut Vec<&'a Element>) -> bool {
    out.push(node);
    if node.id == id || node.children.iter().any(|child| path(child, id, out)) {
        return true;
    }
    out.pop();
    false
}
fn chain(root: &Element, id: Option<ElementId>) -> Vec<&Element> {
    let mut out = Vec::new();
    if let Some(id) = id {
        path(root, id, &mut out);
    }
    out
}

impl UiRuntime {
    /// Prévient les gestionnaires de ce que le dernier événement a changé (`down` : un appui) ; `true` si la vue est à refaire.
    pub(super) fn notify_listeners(&mut self, down: bool) -> bool {
        self.invalidation.has_listeners |= crate::ui::element::take_listeners_built();
        if !self.invalidation.has_listeners {
            return false;
        }
        let Some(root) = self.root.as_ref() else {
            return false;
        };
        let hovered = self.interaction.hovered.map(Hit::id);
        let targets = [hovered, self.interaction.pressed, self.interaction.focused];
        let state = &mut self.invalidation.listeners;
        let mut messages: Vec<Message> = Vec::new();
        if targets != state.targets {
            state.targets = targets;
            for (index, kind) in KINDS.into_iter().enumerate() {
                let path = chain(root, targets[index]);
                let current: Vec<ElementId> = path
                    .iter()
                    .filter(|node| handler(node, kind).is_some())
                    .map(|node| node.id)
                    .collect();
                for id in &state.active[index] {
                    if !current.contains(id)
                        && let Some(handler) = root.find(*id).and_then(|n| handler(n, kind))
                    {
                        messages.push(handler.emit(false));
                    }
                }
                for node in &path {
                    if !state.active[index].contains(&node.id)
                        && let Some(handler) = handler(node, kind)
                    {
                        messages.push(handler.emit(true));
                    }
                }
                state.active[index] = current;
            }
        }
        if down {
            let listener = chain(root, hovered).into_iter().rev().find_map(|node| {
                let action = node.listeners.as_deref()?.double_click.as_ref()?;
                Some((node.id, action))
            });
            let now = Instant::now();
            match listener {
                Some((id, action))
                    if state.last_press.is_some_and(|(last, at)| {
                        last == id && now.saturating_duration_since(at) < DOUBLE_CLICK
                    }) =>
                {
                    messages.push(action.emit());
                    state.last_press = None;
                }
                Some((id, _)) => state.last_press = Some((id, now)),
                None => state.last_press = None,
            }
        }
        let mut rebuild = false;
        for message in messages {
            rebuild |= self.app.update(message);
        }
        if rebuild {
            self.invalidate();
        }
        rebuild
    }
}
