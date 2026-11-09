use super::restyle::{containers_changed, has_dynamic_styles};
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
        self.sync_colors();
        let held = self.take_held_rebuild(context, now);
        let result = self.prepare_frame(context, chrome, &mut text, now);
        if held {
            self.release_rebuild();
        }
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
        // Anciens arbres : une tranche par image qui ne reconstruit pas la vue.
        if !self.invalidation.rebuild && !self.invalidation.graveyard.is_empty() {
            self.bury(super::rebuild::GRAVEYARD_SLICE);
        }
        // Images livrées hors d'une tâche (message, réveil) : repeintes elles aussi.
        self.images_arrived();
        // Contenu défilé : un élément `content-visibility: auto` qui s'approche de la vue
        // est disposé (layout de son seul chemin, sans styles) ; un qui s'en éloigne
        // cesse d'être peint.
        let revealed = std::mem::take(&mut self.invalidation.containment_check)
            && self.reveal_contained(context, chrome, text, now)?;
        // Vue reconstruite (`rebuild.rs`).
        let rebuilding = self.invalidation.rebuild;
        let layouts = self.invalidation.stats.layouts;
        // Ancres de défilement prises sur l'arbre disposé, avant qu'il ne change.
        let anchors = if rebuilding || self.invalidation.relayout || self.invalidation.reflow {
            self.root
                .as_ref()
                .map(anchoring::capture)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let rebuilt = self.rebuild_view(context, now)?;
        let mut motion_tree_changed = rebuilt || self.invalidation.relayout;
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
            let stage = Instant::now();
            // Seuls le pointeur et des animations ont pu changer : résolution partielle.
            let partial = !rebuilt
                && !self.invalidation.relayout
                && self.invalidation.incremental
                && self
                    .invalidation
                    .style_signature
                    .is_some_and(|previous| previous.same_except_pointer(signature))
                && self
                    .root
                    .as_ref()
                    .is_some_and(|root| !containers_changed(root));
            let changes = self.resolve_styles(context, now, partial)?;
            self.invalidation.stats.style_time += stage.elapsed();
            self.invalidation.relayout |= changes.layout;
            self.invalidation.repaint |= changes.paint;
            motion_tree_changed |= changes.layout;
            self.invalidation.style_signature = Some(signature);
            self.invalidation.dynamic_styles = has_dynamic_styles(self.root.as_ref().unwrap());
        }
        if self.invalidation.relayout || self.invalidation.reflow {
            let stage = Instant::now();
            layout::containment::enter(std::mem::take(&mut self.invalidation.containment));
            let pass = layout::LayoutPass::begin(
                self.root.as_mut().unwrap(),
                self.invalidation.incremental,
            );
            let root = self.root.as_mut().unwrap();
            let bounds = window_area(context, chrome);
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
                let containers = containers_changed(self.root.as_ref().unwrap());
                // Survol : les éléments que ce layout vient de placer (vue reconstruite,
                // contenu déplacé) n'avaient pas leur position quand leurs styles ont été
                // résolus ; ceux que le pointeur touche maintenant sont restylés.
                let hover = !containers && self.hover_moved_by_layout();
                if !containers && !hover {
                    break;
                }
                let partial = hover && self.invalidation.incremental;
                let changes = self.resolve_styles(context, now, partial)?;
                self.invalidation.repaint |= changes.paint;
                if !changes.layout {
                    break;
                }
                pass.reset(self.root.as_mut().unwrap());
                let root = self.root.as_mut().unwrap();
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
            // `content-visibility: auto` : proximité constatée sur ce placement ; les
            // éléments qui s'approchent sont disposés à leur tour (leur seul chemin).
            for _ in 0..3 {
                let mut state = layout::containment::leave();
                let root = self.root.as_mut().unwrap();
                let near = layout::containment::refresh(
                    &mut state,
                    root,
                    bounds,
                    self.interaction.focused,
                );
                layout::containment::enter(state);
                if !layout::containment::unsettle(root, &near) {
                    break;
                }
                // Contenu rendu : ses styles d'abord (sautés jusque-là).
                let changes = self.resolve_revealed(&near, context, now)?;
                self.invalidation.repaint |= changes.paint;
                // Mesures de la passe oubliées : celles des ancêtres ne tenaient pas compte
                // du contenu qui vient d'être disposé.
                pass.reset(self.root.as_mut().unwrap());
                let root = self.root.as_mut().unwrap();
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
            self.invalidation.containment = layout::containment::leave();
            drop(pass);
            let root = self.root.as_mut().unwrap();
            for (id, delta) in anchoring::apply(root, &anchors) {
                self.components.scrolls.shift_smooth(id, delta);
            }
            anchoring::initial_targets(root, &self.components.scrolls.offsets);
            let orphan = popover::top(root)
                .filter(|p| popover::anchor(root, p).is_none())
                .and_then(popover::config)
                .map(|p| p.on_dismiss.emit());
            if let Some(action) = orphan {
                // Une ancre retirée, désactivée ou cachée ne laisse pas une couche sans accès.
                root.children.pop();
                layout::update_paint_order(root);
                let live = self.interaction.index_tree(root, &self.components)?;
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
            self.refresh_hover();
            self.invalidation.relayout = false;
            self.invalidation.reflow = false;
            self.invalidation.repaint = true;
            (
                self.invalidation.fixed_paths,
                self.invalidation.virtual_lists,
            ) = super::painting::fixed_paths(self.root.as_ref().unwrap());
            // Les fabriques des listes virtuelles ajoutent des IDs pendant le layout.
            if self.invalidation.virtual_lists {
                layout::validate(self.root.as_ref().unwrap())?;
            }
            self.invalidation.stats.layouts += 1;
            self.invalidation.stats.layout_time += stage.elapsed();
        }
        if let Some(shift) = self.interaction.pending_tab.take() {
            self.move_tab(shift);
        }
        if self.invalidation.repaint {
            let root = self.root.as_mut().unwrap();
            // Sans liste virtuelle, rien à matérialiser : pas de parcours de l'arbre.
            let refreshed = self.invalidation.virtual_lists
                && virtual_list::refresh_all(root, self.interaction.focused, text)?;
            if refreshed {
                motion_tree_changed = true;
                layout::validate(root)?;
                self.components.retain_refreshed(root);
                (
                    self.invalidation.fixed_paths,
                    self.invalidation.virtual_lists,
                ) = super::painting::fixed_paths(root);
            }
            // Arbre changé (vue reconstruite, lignes matérialisées) : compteurs et styles
            // d'état relevés de nouveau ; une image de défilement ne parcourt pas l'arbre.
            if refreshed || rebuilt || self.invalidation.stats.layouts != layouts {
                fn count(node: &Element) -> usize {
                    node.virtual_list
                        .as_ref()
                        .map_or(0, |list| list.indices.len())
                        + node.children.iter().map(count).sum::<usize>()
                }
                self.invalidation.stats.virtual_rows = count(root);
                self.invalidation.dynamic_styles = has_dynamic_styles(root);
            }
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
            self.refresh_hover();
        }
        if self.motions.has_ghosts() {
            self.motions.tick(now);
            self.invalidation.repaint = true;
        }
        let motion_hover = self.motion_hover();
        if motion_tree_changed {
            self.motions.invalidate_paths();
        }
        // Contenu disposé en approchant de la vue : ses pistes d'animation sont créées.
        if let Some(root) = &self.root
            && (motion_tree_changed || revealed || self.motions.has_tracks())
        {
            self.invalidation.repaint |=
                self.motions
                    .sync(root, motion_hover, self.interaction.active_press(), now);
            self.refresh_hover();
        }
        self.sync_tooltip();
        if self.components.tooltips.measure(text)? {
            self.invalidation.repaint_semantics();
        }
        if self.invalidation.repaint {
            let stage = Instant::now();
            self.scene.commands.clear();
            if let Some(root) = &mut self.root {
                // Pendant un redimensionnement ou une transition de taille, les éléments
                // changent de taille à chaque image : leurs ombres floutées sont étirées,
                // l'image exacte revient à la première image stable.
                let stretch = self.live_resize || self.motions.animating_lengths();
                crate::ui::rendering::component_shadow::prepare(
                    root,
                    stretch,
                    &self.invalidation.fixed_paths,
                    &|node| self.motions.paint_clip(node),
                )?;
            }
            paint_chrome(&mut self.scene, context, chrome);
            let state = self.interaction.paint_state();
            if let Some(root) = &self.root {
                let resources = super::painting::PaintResources {
                    inputs: &self.components.inputs,
                    sliders: &self.components.sliders,
                    selects: &self.components.selects,
                    motions: &self.motions,
                    fixed: Some(&self.invalidation.fixed_paths),
                };
                paint_node(root, &mut self.scene, context.theme, state, &resources);
                let resources = super::painting::PaintResources {
                    fixed: None,
                    ..resources
                };
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
            self.invalidation.stats.paint_time += stage.elapsed();
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

/// Zone de la fenêtre sous la barre de titre du moteur.
pub(super) fn window_area(context: ViewContext, chrome: ChromeState) -> RectF {
    RectF::new(
        0.,
        if chrome.visible {
            TITLE_BAR_HEIGHT as f32
        } else {
            0.
        },
        context.width,
        context.height,
    )
}
