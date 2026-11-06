use super::*;
impl Slider {
    pub(crate) fn paint_value(
        &self,
        value: f64,
        active: bool,
        ctx: &PaintContext,
        scene: &mut Scene,
    ) {
        if let Some(parts) = self.css_parts() {
            self.paint_parts(value, parts, ctx, scene);
            return;
        }
        let t = ctx.theme;
        let r = ctx.rect;
        let g = self.geometry(r, value);
        let thickness = self.style.track_thickness.unwrap_or(4.);
        let half = thickness / 2.;
        let track = self.style.track.unwrap_or(t.border);
        let fill = self
            .css
            .as_ref()
            .and_then(|c| c.local.accent)
            .unwrap_or_else(|| {
                if self.enabled {
                    self.style.fill.unwrap_or(t.accent)
                } else {
                    t.muted
                }
            });
        let cross = if self.axis == Axis::Horizontal {
            (r.top + r.bottom) / 2.
        } else {
            (r.left + r.right) / 2.
        };
        let (track_rect, fill_rect, thumb) = if self.axis == Axis::Horizontal {
            (
                RectF::new(g.start, cross - half, g.length, thickness),
                RectF::new(g.start, cross - half, g.center - g.start, thickness),
                RectF::new(g.center - 8., cross - 8., 16., 16.),
            )
        } else {
            (
                RectF::new(cross - half, g.start, thickness, g.length),
                RectF::new(
                    cross - half,
                    g.center,
                    thickness,
                    g.start + g.length - g.center,
                ),
                RectF::new(cross - 8., g.center - 8., 16., 16.),
            )
        };
        scene.commands.push(DrawCommand::PushClip(r));
        scene.fill(track_rect, half, track);
        scene.fill(fill_rect, half, fill);
        if self.enabled && (ctx.state.hovered || active) {
            scene.fill(
                RectF::new(thumb.left - 2., thumb.top - 2., 20., 20.),
                10.,
                t.hover,
            );
        }
        scene.fill(
            thumb,
            8.,
            if self.enabled {
                self.css
                    .as_ref()
                    .and_then(|c| c.text.color)
                    .or(self.style.thumb)
                    .unwrap_or(t.text)
            } else {
                t.muted
            },
        );
        if self.enabled && ctx.state.focused {
            scene.commands.push(DrawCommand::Stroke {
                rect: RectF::new(
                    r.left + 1.,
                    r.top + 1.,
                    (layout::width(r) - 2.).max(0.),
                    (layout::height(r) - 2.).max(0.),
                ),
                radius: 7.,
                width: 2.,
                color: self.style.focus.unwrap_or(t.accent),
            });
        }
        scene.commands.push(DrawCommand::PopClip);
    }
}
impl Slider {
    /// Pouce et piste décrits par la feuille quand elle demande `appearance: none`.
    pub(super) fn css_parts(
        &self,
    ) -> Option<(
        crate::ui::styling::ControlPart,
        crate::ui::styling::ControlPart,
    )> {
        let css = self.css.as_ref()?;
        (css.local.appearance_none == Some(true))
            .then_some((css.local.slider_thumb, css.local.slider_track))
    }
    /// Dessin `appearance: none` : piste sur toute la longueur, pouce centré sur la
    /// valeur, ombres réduites à leur décalage et leur étalement (anneaux de focus).
    fn paint_parts(
        &self,
        value: f64,
        (thumb, track): (
            crate::ui::styling::ControlPart,
            crate::ui::styling::ControlPart,
        ),
        ctx: &PaintContext,
        scene: &mut Scene,
    ) {
        let t = ctx.theme;
        let r = ctx.rect;
        let g = self.geometry(r, value);
        let horizontal = self.axis == Axis::Horizontal;
        let cross = if horizontal {
            (r.top + r.bottom) / 2.
        } else {
            (r.left + r.right) / 2.
        };
        let thickness = track.height.unwrap_or(4.);
        let (tw, th) = (thumb.width.unwrap_or(16.), thumb.height.unwrap_or(16.));
        let (track_rect, thumb_rect) = if horizontal {
            (
                RectF::new(r.left, cross - thickness / 2., layout::width(r), thickness),
                RectF::new(g.center - tw / 2., cross - th / 2., tw, th),
            )
        } else {
            (
                RectF::new(cross - thickness / 2., r.top, thickness, layout::height(r)),
                RectF::new(cross - th / 2., g.center - tw / 2., th, tw),
            )
        };
        let part = |scene: &mut Scene, rect: RectF, p: crate::ui::styling::ControlPart, fill| {
            let radius = p
                .radius
                .unwrap_or(0.)
                .min(layout::width(rect).min(layout::height(rect)) / 2.);
            if let Some(shadows) = p.shadows {
                for s in shadows.values.iter().rev().filter(|s| !s.inset) {
                    let grow = s.spread * shadows.scale;
                    scene.fill(
                        RectF {
                            left: rect.left - grow + s.x,
                            top: rect.top - grow + s.y,
                            right: rect.right + grow + s.x,
                            bottom: rect.bottom + grow + s.y,
                        },
                        (radius + grow).max(0.),
                        s.color,
                    );
                }
            }
            scene.fill(rect, radius, p.background.unwrap_or(fill));
            if let (Some(width), Some(color)) = (p.border_width.filter(|w| *w > 0.), p.border) {
                crate::ui::style::border(scene, rect, radius, width, color);
            }
        };
        part(scene, track_rect, track, t.border);
        part(scene, thumb_rect, thumb, t.accent);
    }
}
impl Widget for Slider {
    fn apply_style(&mut self, value: &crate::ui::styling::ComputedStyle) -> bool {
        crate::ui::styling::text::update(&mut self.css, value)
    }
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(if self.axis == Axis::Horizontal {
            SizeF {
                width: 240.,
                height: 40.,
            }
        } else {
            SizeF {
                width: 40.,
                height: 160.,
            }
        })
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        self.paint_value(self.value, false, ctx, scene);
    }
    fn slider(&self) -> Option<&Slider> {
        Some(self)
    }
    fn focusable(&self) -> bool {
        self.enabled
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Slider,
            name: &self.label,
            enabled: self.enabled,
        }
    }
}
