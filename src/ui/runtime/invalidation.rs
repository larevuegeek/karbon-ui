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
    /// Éléments `fixed` et leurs ancêtres, relevés au dernier layout (peinture).
    pub(super) fixed_paths: crate::ui::core::hash::FastMap<ElementId, ()>,
    /// L'arbre contient une liste virtuelle (relevé au dernier layout).
    pub(super) virtual_lists: bool,
    /// Élément focalisé et élément appuyé lors de la dernière résolution des styles.
    pub(super) style_targets: [Option<ElementId>; 2],
    pub(super) rebuild: bool,
    pub(super) relayout: bool,
    /// Layout sans résolution des styles : un élément `content-visibility: auto` s'est
    /// approché de la vue (voir `layout::containment`).
    pub(super) reflow: bool,
    /// Le contenu a défilé : la proximité des éléments `content-visibility: auto` est à
    /// constater de nouveau.
    pub(super) containment_check: bool,
    /// Proximité et tailles retenues des éléments `content-visibility: auto`.
    pub(super) containment: crate::ui::layout::containment::Containment,
    pub(super) repaint: bool,
    pub(super) style_signature: Option<StyleSignature>,
    /// L'arbre porte des styles d'état : le backend doit repeindre au survol.
    pub(super) dynamic_styles: bool,
    /// Images livrées aux `ImageSlot` lors de la dernière vérification (voir
    /// `UiRuntime::images_arrived`).
    pub(super) image_arrivals: u64,
    /// Somme des révisions des `ImageSlot` de l'arbre à la dernière vérification.
    pub(super) image_revisions: u64,
    /// Révision des informations d'accessibilité non portées par l'arbre (popups, bulles…).
    pub(super) accessibility_revision: u64,
    /// Mémoires de la préparation actives (voir `UiRuntime::set_incremental`).
    pub(super) incremental: bool,
    /// Faits du pointeur lus par la dernière résolution des styles.
    pub(super) pointer_facts: Option<crate::ui::styling::resolve::PointerFacts>,
    pub(super) stats: RuntimeStats,
    /// Anciens arbres remplacés par une reconstruction, libérés par tranches hors des
    /// images qui reconstruisent (voir `UiRuntime::bury`).
    pub(super) graveyard: Vec<Element>,
    /// Prochaine tranche de libération au repos.
    pub(super) graveyard_wake: Option<Instant>,
    /// Dernière tranche du travail étalé de l'application.
    pub(super) frame_work_at: Option<Instant>,
    /// Sonde de défilement (`KARBON_SCROLL_PROBE`).
    pub(super) probe: Option<super::scroll_probe::ScrollProbe>,
    /// Thème du système (`ColorScheme::System`).
    pub(super) system_dark: bool,
    /// Préférences de couleurs résolues : (sombre, contraste élevé).
    pub(super) colors: (bool, bool),
    /// Une vue a posé des gestionnaires de survol, d'appui, de focus ou de double clic.
    pub(super) has_listeners: bool,
    /// Ce dont ces gestionnaires ont été prévenus.
    pub(super) listeners: super::listeners::ListenerState,
    /// La vue est bâtie avec les composants du kit.
    pub(super) kit_used: bool,
    /// Révision du thème du kit lue à la dernière vue.
    pub(super) kit_revision: u64,
    /// Préférences changées : la prochaine vue ne reprend aucun sous-arbre mémorisé.
    pub(super) forget_memos: bool,
    /// Jusqu'à cet instant, une reconstruction demandée attend l'image suivante (voir
    /// `hold_rebuild`).
    pub(super) rebuild_hold: Option<Instant>,
    /// La tranche de cette image est déjà faite : le réveil qui suit n'en fait pas d'autre.
    pub(super) frame_work_done: bool,
}
impl Default for Invalidation {
    /// Tout est à construire au premier `prepare`.
    fn default() -> Self {
        Self {
            fixed_paths: Default::default(),
            virtual_lists: false,
            style_targets: [None; 2],
            rebuild: true,
            relayout: true,
            reflow: false,
            containment_check: false,
            containment: Default::default(),
            repaint: true,
            style_signature: None,
            dynamic_styles: false,
            image_arrivals: crate::ui::template::ImageSlot::arrivals(),
            image_revisions: 0,
            accessibility_revision: 0,
            incremental: true,
            pointer_facts: None,
            stats: RuntimeStats::default(),
            graveyard: Vec::new(),
            graveyard_wake: None,
            frame_work_at: None,
            probe: super::scroll_probe::ScrollProbe::from_env(),
            system_dark: false,
            colors: (false, false),
            has_listeners: false,
            listeners: Default::default(),
            kit_used: false,
            kit_revision: 0,
            forget_memos: false,
            rebuild_hold: None,
            frame_work_done: false,
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
