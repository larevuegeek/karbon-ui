//! Fichiers tirés depuis l'Explorateur et déposés sur un élément
//! (`Element::on_file_drop`) : le survol, la sortie et le dépôt, avec les chemins
//! complets. Un élément qui accepte le survol rend un message ; le système montre alors
//! le curseur « copier », sinon le curseur interdit.
use super::super::hit_test::fixed_result;
use super::super::*;
use super::pointer::target;
use std::{path::PathBuf, rc::Rc, sync::Arc};

/// What happens to files dragged from another program over the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileDropPhase {
    /// The files are over the element (repeated while the pointer moves).
    Over,
    /// They left the element, or the drag was cancelled.
    Leave,
    /// They were dropped on the element.
    Drop,
}

/// Files dragged from another program (the Explorer) over an element with an
/// `on_file_drop` handler ([`Element::on_file_drop`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FileDropEvent {
    /// Over, left, or dropped.
    pub phase: FileDropPhase,
    /// The full paths of the files and folders dragged.
    pub paths: Arc<[PathBuf]>,
    /// Pointer position in window coordinates, logical pixels.
    pub x: f32,
    /// Pointer position in window coordinates, logical pixels.
    pub y: f32,
    /// Pointer position from the top left corner of the element, logical pixels.
    pub local_x: f32,
    /// Pointer position from the top left corner of the element, logical pixels.
    pub local_y: f32,
}

/// Gestionnaire `on_file_drop` : `None` refuse les fichiers (ils passent aux ancêtres).
#[derive(Clone)]
pub(crate) struct FileDropHandler(Rc<DropFn>);

/// Ce que fait un gestionnaire `on_file_drop`.
type DropFn = dyn Fn(&FileDropEvent) -> Option<Message>;

impl FileDropHandler {
    pub(crate) fn new<M: 'static>(handler: impl Fn(&FileDropEvent) -> Option<M> + 'static) -> Self {
        Self(Rc::new(move |event| handler(event).map(Message::new)))
    }
}

thread_local! {
    /// Un gestionnaire `on_file_drop` a été posé sur ce fil : la fenêtre s'inscrit alors
    /// comme cible de dépôt (une fois).
    pub(crate) static FILE_DROP_BUILT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Gestionnaires `on_file_drop` de `start` et de ses ancêtres, avec l'élément qui le porte.
fn drop_handlers(
    node: &Element,
    start: ElementId,
    out: &mut Vec<(ElementId, RectF, FileDropHandler)>,
) -> bool {
    if node.id != start && !node.children.iter().any(|c| drop_handlers(c, start, out)) {
        return false;
    }
    if let Some(handler) = node.listeners.as_ref().and_then(|l| l.file_drop.clone()) {
        out.push((node.id, node.rect, handler));
    }
    true
}

impl UiRuntime {
    /// Des fichiers survolent la fenêtre, la quittent ou y sont déposés à `position` :
    /// donnés aux gestionnaires `on_file_drop` sous le pointeur, du plus profond à la
    /// racine, jusqu'au premier qui les accepte. `true` quand l'un les accepte (le
    /// système montre « copier »). L'élément quitté reçoit `Leave`.
    pub(crate) fn file_drag(
        &mut self,
        phase: FileDropPhase,
        paths: &Arc<[PathBuf]>,
        position: PointF,
    ) -> bool {
        let previous = self.raw_pointer.file_target.take();
        let mut accepted = None;
        if phase != FileDropPhase::Leave {
            for (id, rect, handler) in self.file_drop_path(position) {
                let event = drop_event(phase, paths, position, rect);
                if let Some(message) = (handler.0)(&event) {
                    self.send(message);
                    accepted = Some(id);
                    break;
                }
            }
        }
        if previous.is_some() && previous != accepted {
            self.leave_file_target(previous, paths, position);
        }
        if phase == FileDropPhase::Over {
            self.raw_pointer.file_target = accepted;
        }
        if accepted.is_some() || previous.is_some() {
            self.invalidation.repaint = true;
        }
        accepted.is_some()
    }

    fn file_drop_path(&self, position: PointF) -> Vec<(ElementId, RectF, FileDropHandler)> {
        let Some(root) = self.root.as_ref() else {
            return Vec::new();
        };
        let scope = layers::active(root);
        let found = fixed_result(scope, position, &self.motions, true, target)
            .or_else(|| target(scope, position, &self.motions, true));
        let mut path = Vec::new();
        if let Some(found) = found {
            drop_handlers(scope, found, &mut path);
        }
        path
    }

    fn leave_file_target(&mut self, id: Option<ElementId>, paths: &Arc<[PathBuf]>, at: PointF) {
        let held = id.and_then(|id| {
            let node = self.root.as_ref()?.find(id)?;
            Some((node.rect, node.listeners.as_ref()?.file_drop.clone()?))
        });
        if let Some((rect, handler)) = held
            && let Some(message) = (handler.0)(&drop_event(FileDropPhase::Leave, paths, at, rect))
        {
            self.send(message);
        }
    }
}

fn drop_event(
    phase: FileDropPhase,
    paths: &Arc<[PathBuf]>,
    at: PointF,
    rect: RectF,
) -> FileDropEvent {
    FileDropEvent {
        phase,
        paths: Arc::clone(paths),
        x: at.x,
        y: at.y,
        local_x: at.x - rect.left,
        local_y: at.y - rect.top,
    }
}
