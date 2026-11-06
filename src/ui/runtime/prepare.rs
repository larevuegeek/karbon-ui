use super::restyle::{carry_caches, containers_changed, has_dynamic_styles};
use super::*;
impl UiRuntime {
    /// Bring the view up to date for `context`: rebuild, lay out and paint whatever was
    /// invalidated, then store the result in [`scene`](Self::scene). Uses the current time.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) for invalid view dimensions or
    /// an invalid tree (see [`layout::validate`]), and any
    /// error returned by `text`.
    pub fn prepare(
        &mut self,
        context: ViewContext,
        chrome: ChromeState,
        text: &mut dyn TextMeasurer,
    ) -> crate::Result<()> {
        self.prepare_at(context, chrome, text, Instant::now())
    }
    /// Like [`prepare`](Self::prepare) with an explicit clock, for tests, captures and hosts
    /// that manage their own frames.
    ///
    /// # Errors
    ///
    /// Same as [`prepare`](Self::prepare).
    pub fn prepare_at(
        &mut self,
        context: ViewContext,
        chrome: ChromeState,
        text: &mut dyn TextMeasurer,
        now: Instant,
    ) -> crate::Result<()> {
        // Mesures de texte gardées d'une image à l'autre (voir `text_cache`).
        let mut cache = std::mem::take(&mut self.text_cache);
        cache.disabled = !self.invalidation.incremental;
        cache.begin(context.scale);
        let mut text = super::text_cache::Cached {
            inner: text,
            cache: &mut cache,
        };
        let result = self.prepare_frame(context, chrome, &mut text, now);
        self.text_cache = cache;
        result
    }
    fn prepare_frame(
        &mut self,
        context: ViewContext,
        chrome: ChromeState,
        text: &mut dyn TextMeasurer,
        now: Instant,
    ) -> crate::Result<()> {
        if !context.width.is_finite()
            || !context.height.is_finite()
            || context.width < 0.
            || context.height < 0.
        {
            return Err(crate::Error::invalid_input("invalid view dimensions"));
        }
        if self.context != Some(context) {
            self.context = Some(context);
            self.invalidate();
        }
        if self.chrome != chrome {
            self.invalidation.relayout |= self.chrome.visible != chrome.visible;
            self.chrome = chrome;
            self.invalidation.repaint = true;
        }
        let rebuilt = self.invalidation.rebuild;
        let mut motion_tree_changed = rebuilt || self.invalidation.relayout;
        if self.invalidation.rebuild {
            if self.components.item_drag.take().is_some() {
                self.interaction.pressed = None;
                self.interaction.release_pending = true;
            }
            let mut root = layers::hoist(self.app.view(&context));
            layout::validate(&root)?;
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
                    root = layers::hoist(self.app.view(&context));
                    layout::validate(&root)?;
                }
            }
            let live = self.interaction.index_tree(&mut root, &self.components);
            if layer_changed {
                self.interaction.focus_visible = !pointer_close;
                self.interaction.keyboard_pressed = None;
                self.invalidation.semantics();
            }
            if self.components.retain_rebuilt(&root, &live) {
                self.interaction.release_pending = true;
            }
            self.interaction.retain_focusable();
            if let Some(mut old) = self.root.take() {
                carry_caches(&mut old, &mut root, &live);
                self.motions.collect_ghosts(old, &live, now);
            }
            self.root = Some(root);
            self.invalidation.rebuild = false;
            self.invalidation.relayout = true;
            self.invalidation.stats.views += 1;
        }
        let signature = self
            .interaction
            .style_signature(self.components.inputs.revision());
        if !rebuilt && self.pointer_moved_only(signature) {
            self.invalidation.style_signature = Some(signature);
        }
        if (rebuilt
            || self.invalidation.relayout
            || self.invalidation.style_signature != Some(signature)
            // Les keyframes (opacité, transformations) s'appliquent à la peinture : seules
            // les transitions de couleurs et de longueurs demandent une cascade par image.
            // Un survol déplacé par une animation change la signature d'interaction.
            || self.motions.animating_styles())
            && self.root.is_some()
        {
            let changes = self.resolve_styles(context, now)?;
            self.invalidation.relayout |= changes.layout;
            self.invalidation.repaint |= changes.paint;
            motion_tree_changed |= changes.layout;
            self.invalidation.style_signature = Some(signature);
            self.invalidation.dynamic_styles = has_dynamic_styles(self.root.as_ref().unwrap());
        }
        if self.invalidation.relayout {
            let pass = layout::LayoutPass::begin(
                self.root.as_ref().unwrap(),
                self.invalidation.incremental,
            );
            let root = self.root.as_mut().unwrap();
            layout::validate(root)?;
            let bounds = RectF::new(
                0.,
                if chrome.visible {
                    TITLE_BAR_HEIGHT as f32
                } else {
                    0.
                },
                context.width,
                context.height,
            );
            layout::measure(
                root,
                SizeF {
                    width: context.width,
                    height: context.height,
                },
                text,
            )?;
            layout::arrange(root, bounds, bounds, text)?;
            // Requêtes de conteneur : un conteneur dont ce layout change la taille relance
            // la résolution des styles puis le layout, sans reconstruire la vue. Pendant une
            // transition de taille ou un redimensionnement, la taille change à chaque image :
            // les requêtes suivent avec une image de retard (une passe au lieu de deux), et
            // redeviennent exactes à la première image stable.
            let settle = if self.live_resize || self.motions.animating_lengths() {
                0
            } else {
                4
            };
            for _ in 0..settle {
                if !containers_changed(self.root.as_ref().unwrap()) {
                    break;
                }
                let changes = self.resolve_styles(context, now)?;
                self.invalidation.repaint |= changes.paint;
                if !changes.layout {
                    break;
                }
                pass.reset(self.root.as_ref().unwrap());
                let root = self.root.as_mut().unwrap();
                layout::validate(root)?;
                layout::measure(
                    root,
                    SizeF {
                        width: context.width,
                        height: context.height,
                    },
                    text,
                )?;
                layout::arrange(root, bounds, bounds, text)?;
            }
            drop(pass);
            let root = self.root.as_mut().unwrap();
            // Les fabriques peuvent ajouter des IDs pendant le layout : vérifier l'arbre final.
            layout::validate(root)?;
            let orphan = popover::top(root)
                .filter(|p| popover::anchor(root, p).is_none())
                .and_then(popover::config)
                .map(|p| p.on_dismiss.emit());
            if let Some(action) = orphan {
                // Une ancre retirée, désactivée ou cachée ne laisse pas une couche sans accès.
                root.children.pop();
                layout::update_paint_order(root);
                let live = self.interaction.index_tree(root, &self.components);
                self.components.retain_live(&live);
                if self
                    .interaction
                    .pressed
                    .is_some_and(|id| !live.contains(&id))
                {
                    self.interaction.pressed = None;
                    self.interaction.release_pending = true;
                }
                self.interaction.keyboard_pressed = None;
                self.invalidation.semantics();
                if self.app.update(action) {
                    self.invalidation.rebuild = true;
                }
            }
            self.components.save(root);
            self.interaction.hovered = self.interaction.pointer.and_then(|p| self.hit(p));
            self.invalidation.relayout = false;
            self.invalidation.repaint = true;
            self.invalidation.stats.layouts += 1;
        }
        if let Some(shift) = self.interaction.pending_tab.take() {
            self.move_tab(shift);
        }
        if self.invalidation.repaint {
            let root = self.root.as_mut().unwrap();
            if virtual_list::refresh_all(root, self.interaction.focused, text)? {
                motion_tree_changed = true;
                layout::validate(root)?;
                self.components.retain_refreshed(root);
            }
            fn count(node: &Element) -> usize {
                node.virtual_list
                    .as_ref()
                    .map_or(0, |list| list.indices.len())
                    + node.children.iter().map(count).sum::<usize>()
            }
            self.invalidation.stats.virtual_rows = count(root);
            // Layout/scroll can materialize styled rows after the initial style pass.
            self.invalidation.dynamic_styles = has_dynamic_styles(root);
            let viewport = RectF::new(
                4.,
                if self.chrome.visible {
                    TITLE_BAR_HEIGHT as f32 + 4.
                } else {
                    4.
                },
                (context.width - 8.).max(0.),
                (context.height - 8.).max(0.),
            );
            let outcome = self.components.reconcile(&mut SyncContext {
                root,
                focused: self.interaction.focused,
                text: &mut *text,
                rebuilt,
                viewport,
            })?;
            if outcome.semantics {
                self.invalidation.semantics();
            }
            self.interaction.release_pending |= outcome.release;
            self.interaction.hovered = self.interaction.pointer.and_then(|p| self.hit(p));
        }
        if self.motions.has_ghosts() {
            self.motions.tick(now);
            self.invalidation.repaint = true;
        }
        let motion_hover = self.motion_hover();
        if let Some(root) = &self.root
            && (motion_tree_changed || self.motions.has_tracks())
        {
            self.invalidation.repaint |=
                self.motions
                    .sync(root, motion_hover, self.interaction.active_press(), now);
            self.interaction.hovered = self.interaction.pointer.and_then(|p| self.hit(p));
        }
        self.sync_tooltip();
        if self.components.tooltips.measure(text)? {
            self.invalidation.repaint_semantics();
        }
        if self.invalidation.repaint {
            self.scene.commands.clear();
            if let Some(root) = &mut self.root {
                // Pendant un redimensionnement ou une transition de taille, les éléments
                // changent de taille à chaque image : leurs ombres floutées sont étirées,
                // l'image exacte revient à la première image stable.
                let stretch = self.live_resize || self.motions.animating_lengths();
                crate::ui::rendering::component_shadow::prepare(root, stretch)?;
            }
            paint_chrome(&mut self.scene, context, chrome);
            let state = self.interaction.paint_state();
            if let Some(root) = &self.root {
                let resources = super::painting::PaintResources {
                    inputs: &self.components.inputs,
                    sliders: &self.components.sliders,
                    selects: &self.components.selects,
                    motions: &self.motions,
                };
                paint_node(root, &mut self.scene, context.theme, state, &resources);
                // Fantômes des éléments retirés : animation de sortie, sans interaction.
                for (ghost, transform, opacity) in self.motions.ghosts() {
                    self.scene.commands.push(DrawCommand::PushEffect {
                        transform,
                        opacity,
                        bounds: ghost.clip,
                    });
                    paint_node(
                        ghost,
                        &mut self.scene,
                        context.theme,
                        super::PaintState {
                            hovered: None,
                            pressed: None,
                            focused: None,
                            keyboard_pressed: None,
                            focus_visible: false,
                        },
                        &resources,
                    );
                    self.scene.commands.push(DrawCommand::PopEffect);
                }
                self.components
                    .selects
                    .paint_popup(root, context.theme, &mut self.scene);
                self.components
                    .edit_menu
                    .paint(context.theme, &mut self.scene);
            }
            self.paint_drag_drop();
            self.components
                .tooltips
                .paint(&mut self.scene, context.theme);
            self.invalidation.repaint = false;
            self.invalidation.stats.paints += 1;
        }
        // Appui qui a fermé un popover : la vue est reconstruite sans lui, l'élément
        // dessous le reçoit maintenant. Sa capture est rendue au relâchement.
        if let Some(press) = self.interaction.replay.take() {
            self.interaction.replaying = true;
            self.event(press);
            self.interaction.replaying = false;
            self.interaction.replay_release = true;
        }
        Ok(())
    }
}
