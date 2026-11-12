//! Reconstruction de la vue (sous-arbres repris, ancien arbre remplacé). Les anciens
//! arbres d'une grande reconstruction sont libérés par tranches, hors des images qui
//! reconstruisent (`bury`).
//!
//! Construire la vue à une image et la brancher à la suivante pendant un défilement a été
//! essayé et retiré : mesuré à l'écran, l'image qui reporte ne montre rien de neuf et
//! allonge l'écart entre deux images (21 à-coups au lieu de 12 sur un défilement de 30 s).
//! `hold_rebuild` est autre chose : la reconstruction entière passe à l'image suivante, où
//! elle se fait pendant l'attente de l'écran, et l'image courante montre le défilement.
use super::restyle::carry_caches;
use super::*;

/// Taille à partir de laquelle un ancien arbre est libéré par tranches.
pub(super) const GRAVEYARD_MIN: usize = 4096;
/// Durée d'une tranche de libération des anciens arbres pendant une image.
pub(super) const GRAVEYARD_SLICE: std::time::Duration = std::time::Duration::from_millis(2);
/// Délai avant de libérer au repos ce que les images n'ont pas libéré.
pub(super) const GRAVEYARD_DELAY: std::time::Duration = std::time::Duration::from_millis(60);

/// Écart minimal entre deux tranches du travail étalé : la préparation peut être appelée
/// plusieurs fois par image affichée, une seule tranche doit y entrer.
const FRAME_WORK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);
/// Réveil de la tranche suivante, un peu après l'écart minimal.
const FRAME_WORK_WAKE: std::time::Duration = std::time::Duration::from_millis(12);

/// Durée maximale d'une reconstruction retenue : une image, si la suivante ne vient pas.
const REBUILD_HOLD: std::time::Duration = std::time::Duration::from_millis(20);

/// Vue construite et clés des sous-arbres mémorisés de la vue précédente.
type BuiltView = (
    Element,
    Option<crate::ui::core::hash::FastMap<ElementId, u64>>,
);

impl UiRuntime {
    /// Vue de l'application ; ses sous-arbres mémorisés inchangés restent à reprendre.
    fn build_view(&mut self, context: ViewContext) -> BuiltView {
        // Thème changé : les dégradés et les ombres lisent leurs variables à la construction.
        let reuse =
            self.invalidation.incremental && !std::mem::take(&mut self.invalidation.forget_memos);
        let keys = reuse.then(|| crate::ui::view::previous_keys(self.root.as_ref()));
        let root = layers::hoist(crate::ui::view::with_previous(keys.clone(), || {
            self.app.view(&context)
        }));
        (root, keys)
    }

    /// Reconstruit la vue si elle est demandée ; `true` si l'arbre a changé.
    pub(super) fn rebuild_view(
        &mut self,
        context: ViewContext,
        now: Instant,
    ) -> crate::Result<bool> {
        if !self.invalidation.rebuild {
            return Ok(false);
        }
        let stage = Instant::now();
        crate::ui::styling::set_color_scheme(self.invalidation.colors);
        let (mut root, keys) = self.build_view(context);
        if self.components.item_drag.take().is_some() {
            self.interaction.pressed = None;
            self.interaction.release_pending = true;
        }
        let layer_changed = self.interaction.layer_focus.changed(&root);
        // Une couche qui s'ouvre ou se ferme garde la modalité de la dernière
        // interaction. Après la souris (clic sur un bouton de menu, choix d'un élément,
        // clic ailleurs), le focus déplacé n'est pas montré et son aide ne s'affiche
        // pas ; au clavier (Entrée, Échap…), il l'est, avec l'aide du contrôle.
        let pointer_close = layer_changed
            && !self.interaction.tooltip_keyboard
            && !self.interaction.escape_consumed;
        if pointer_close {
            if self.components.tooltips.reset() {
                self.invalidation.repaint_semantics();
            }
        } else if layer_changed {
            self.tooltip_input_mode(true);
        }
        if layer_changed {
            // Terminer les interactions de l'ancienne couche avant de déplacer le focus.
            // La vue n'est reconstruite de nouveau que si un message livré pendant
            // l'annulation (fin d'un glisser…) l'a demandé : sinon elle serait la même.
            self.invalidation.rebuild = false;
            let backdrop = self.interaction.backdrop_capture;
            let reply = self.event(UiEvent::Cancel);
            self.interaction.backdrop_capture = backdrop;
            self.interaction.release_pending |= reply.release && backdrop.is_none();
            if self.invalidation.rebuild || !self.invalidation.incremental {
                root = layers::hoist(crate::ui::view::with_previous(keys, || {
                    self.app.view(&context)
                }));
            }
        }
        crate::ui::view::transplant(&mut root, self.root.as_mut());
        // Arbre vérifié une fois par reconstruction : sa structure ne change plus jusqu'à
        // la suivante (sauf les lignes qu'une liste virtuelle fabrique pendant le layout,
        // vérifiées après lui). Les sous-arbres repris ne sont pas reparcourus : seul ce
        // qui dépend de leurs nouveaux ancêtres est regardé (`CheckedSubtree`).
        layout::validate(&root)?;
        let live = self.interaction.index_tree(&mut root, &self.components)?;
        if layer_changed {
            self.interaction.focus_visible = !pointer_close;
            self.interaction.keyboard_pressed = None;
            self.invalidation.semantics();
        }
        if self.components.retain_rebuilt(&root, &live) {
            self.interaction.release_pending = true;
        }
        self.interaction.retain_focusable(&live);
        if let Some(mut old) = self.root.take() {
            carry_caches(&mut old, &mut root, &live);
            // Grand arbre (liste de morceaux : ~10 000 éléments, 4 à 8 ms) : libéré plus
            // tard, par tranches ; un petit l'est tout de suite.
            if self.motions.collect_ghosts(&mut old, &live, now) >= GRAVEYARD_MIN {
                // Au plus un arbre en attente : des reconstructions à chaque image (saisie,
                // animation de la vue) ne laissent aucune image libérer, et la mémoire
                // grossirait sans fin ; l'allocateur réutilise alors tout de suite ce qui
                // est rendu ici au lieu de demander des pages neuves au système.
                drop(std::mem::take(&mut self.invalidation.graveyard));
                self.invalidation.graveyard.push(old);
                self.invalidation.graveyard_wake = Some(now + GRAVEYARD_DELAY);
            }
        }
        self.root = Some(root);
        self.invalidation.rebuild = false;
        self.invalidation.relayout = true;
        self.invalidation.stats.views += 1;
        self.invalidation.stats.view_time += stage.elapsed();
        Ok(true)
    }

    /// Contexte de la suite de l'image : une vue bâtie avec le kit donne ses couleurs au
    /// thème du moteur.
    pub(super) fn themed_context(&mut self, context: ViewContext) -> ViewContext {
        self.invalidation.kit_used |= crate::ui::kit::take_used();
        if self.invalidation.kit_used {
            ViewContext {
                theme: crate::ui::kit::engine_theme(self.invalidation.colors.0),
                ..context
            }
        } else {
            context
        }
    }

    /// Relit les préférences de couleurs de l'application ; un changement refait tout.
    pub(super) fn sync_colors(&mut self) {
        use crate::application::ColorScheme;
        let preferences = self.app.color_preferences();
        let dark = match preferences.scheme {
            ColorScheme::System => self.invalidation.system_dark,
            ColorScheme::Light => false,
            ColorScheme::Dark => true,
        };
        let colors = (dark, preferences.high_contrast);
        let kit = crate::ui::kit::revision();
        if self.invalidation.colors != colors || self.invalidation.kit_revision != kit {
            self.invalidation.colors = colors;
            self.invalidation.kit_revision = kit;
            self.invalidation.forget_memos = true;
            self.invalidate();
        }
    }

    /// Prochaine tranche du travail que l'application étale sur plusieurs images.
    pub(super) fn frame_work_wake(&self) -> Option<Instant> {
        self.app.has_frame_work().then(|| {
            self.invalidation
                .frame_work_at
                .map_or_else(Instant::now, |at| at + FRAME_WORK_WAKE)
        })
    }

    /// Fait une tranche du travail étalé de l'application.
    fn run_frame_work(&mut self, now: Instant) {
        self.invalidation.frame_work_at = Some(now);
        if self.app.frame_work() {
            self.invalidate();
        }
    }

    /// Image de défilement dont l'attente de l'écran vient de finir : une reconstruction
    /// demandée à partir de maintenant (message reçu pendant l'attente, arrivée d'un lot)
    /// se ferait dans ce qui reste de l'image et lui ferait manquer son affichage. Elle
    /// attend l'image suivante, qui la fera avant son attente ; celle-ci montre quand même
    /// la nouvelle position du défilement.
    pub(crate) fn hold_rebuild(&mut self, now: Instant) {
        self.invalidation.rebuild_hold = self.is_scrolling().then_some(now + REBUILD_HOLD);
    }

    /// Reconstruction retenue par `hold_rebuild` : réveil immédiat pour l'image suivante.
    pub(super) fn held_rebuild_wake(&self) -> Option<Instant> {
        (self.invalidation.rebuild && self.invalidation.rebuild_hold.is_some()).then(Instant::now)
    }

    /// Retient la reconstruction demandée le temps de cette préparation ; `true` si elle
    /// est retenue, elle est alors redemandée après (`release_rebuild`).
    pub(super) fn take_held_rebuild(&mut self, context: ViewContext, now: Instant) -> bool {
        let held = self.invalidation.rebuild
            && self.root.is_some()
            && self.context == Some(context)
            && self
                .invalidation
                .rebuild_hold
                .is_some_and(|until| now < until);
        if held {
            self.invalidation.rebuild = false;
            self.invalidation.relayout = false;
        }
        held
    }

    pub(super) fn release_rebuild(&mut self) {
        self.invalidation.rebuild = true;
        self.invalidation.relayout = true;
    }

    /// Début d'une image d'animation, avant l'attente de l'écran (voir « Cadence d'une
    /// image » dans `platform/backend.rs`) : la tranche de cette image se fait ici, et non
    /// au réveil qui suit l'attente, où elle s'ajouterait au temps de l'image.
    pub(crate) fn begin_frame_work(&mut self, now: Instant) {
        self.invalidation.rebuild_hold = None;
        if self.app.has_frame_work() {
            self.run_frame_work(now);
            self.invalidation.frame_work_done = true;
        }
    }

    /// Réveil demandé par `next_wake` : horloges, défilement, travail étalé.
    pub(super) fn timer_event(&mut self, now: Instant) -> EventResult {
        self.probe_tick(now);
        if self
            .invalidation
            .graveyard_wake
            .is_some_and(|wake| wake <= now)
        {
            self.bury(std::time::Duration::from_millis(4));
        }
        // Réveil sans image d'animation : la préparation peut être appelée plusieurs fois
        // par image affichée, une seule tranche doit y entrer.
        let due = self
            .invalidation
            .frame_work_at
            .is_none_or(|at| now.saturating_duration_since(at) >= FRAME_WORK_INTERVAL);
        if !std::mem::take(&mut self.invalidation.frame_work_done)
            && due
            && self.app.has_frame_work()
        {
            self.run_frame_work(now);
        }
        self.invalidation.repaint |= self.motions.tick(now);
        self.invalidation.repaint |= self.smooth_scroll_tick(now);
        self.invalidation.repaint |= self.thaw_hover(now);
        if self.components.tooltips.tick(now) {
            self.invalidation.repaint_semantics();
        }
        EventResult {
            repaint: self.invalidation.repaint,
            ..EventResult::default()
        }
    }

    /// Libère des éléments des anciens arbres pendant au plus `budget`, un élément à la
    /// fois (ses enfants restent à libérer), et planifie la tranche suivante.
    pub(super) fn bury(&mut self, budget: std::time::Duration) {
        let start = Instant::now();
        let graveyard = &mut self.invalidation.graveyard;
        let mut count = 0usize;
        while let Some(mut element) = graveyard.pop() {
            graveyard.append(&mut element.children);
            drop(element);
            count += 1;
            if count.is_multiple_of(64) && start.elapsed() >= budget {
                break;
            }
        }
        self.invalidation.graveyard_wake =
            (!graveyard.is_empty()).then(|| Instant::now() + GRAVEYARD_DELAY / 2);
    }
}
