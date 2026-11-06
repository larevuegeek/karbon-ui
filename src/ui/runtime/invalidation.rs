//! Invalidation : ce qui doit être reconstruit, réarrangé ou repeint, et les caches associés.
use super::*;

/// Clé du cache de résolution des styles d'état (`:hover`, `:focus-visible`…).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct StyleSignature {
    pub(super) input_revision: u64,
    pub(super) focus_visible: bool,
    pub(super) pointer: Option<(u32, u32)>,
    pub(super) pressed: Option<ElementId>,
    pub(super) focused: Option<ElementId>,
}
impl StyleSignature {
    /// Même état d'interaction, à la position du pointeur près.
    pub(super) fn same_except_pointer(self, other: Self) -> bool {
        Self {
            pointer: other.pointer,
            ..self
        } == other
    }
}

pub(super) struct Invalidation {
    pub(super) rebuild: bool,
    pub(super) relayout: bool,
    pub(super) repaint: bool,
    pub(super) style_signature: Option<StyleSignature>,
    /// L'arbre porte des styles d'état : le backend doit repeindre au survol.
    pub(super) dynamic_styles: bool,
    /// Révision des informations d'accessibilité non portées par l'arbre (popups, bulles…).
    pub(super) accessibility_revision: u64,
    /// Mémoires de la préparation actives (voir `UiRuntime::set_incremental`).
    pub(super) incremental: bool,
    /// Faits du pointeur lus par la dernière résolution des styles.
    pub(super) pointer_facts: Option<crate::ui::styling::resolve::PointerFacts>,
    pub(super) stats: RuntimeStats,
}
impl Default for Invalidation {
    /// Tout est à construire au premier `prepare`.
    fn default() -> Self {
        Self {
            rebuild: true,
            relayout: true,
            repaint: true,
            style_signature: None,
            dynamic_styles: false,
            accessibility_revision: 0,
            incremental: true,
            pointer_facts: None,
            stats: RuntimeStats::default(),
        }
    }
}
impl Invalidation {
    pub(super) fn all(&mut self) {
        self.rebuild = true;
        self.relayout = true;
        self.repaint = true;
    }

    /// Une information exposée à l'accessibilité a changé hors de l'arbre.
    pub(super) fn semantics(&mut self) {
        self.accessibility_revision = self.accessibility_revision.wrapping_add(1);
    }

    /// Changement visible et exposé à l'accessibilité.
    pub(super) fn repaint_semantics(&mut self) {
        self.repaint = true;
        self.semantics();
    }

    /// Une image doit être produite, quelle que soit la passe invalidée.
    pub(super) fn pending(&self) -> bool {
        self.repaint || self.rebuild || self.relayout
    }
}
