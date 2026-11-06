use super::*;
pub(super) fn measure(
    label: &str,
    switch: bool,
    css: Option<&crate::ui::styling::ComputedStyle>,
    text: &mut dyn TextMeasurer,
) -> crate::Result<SizeF> {
    // `appearance: none` : la boîte CSS (largeur, hauteur, bordure) fait le contrôle.
    if appearance_none(css) {
        return Ok(SizeF::default());
    }
    let size = if let Some(css) = css {
        crate::ui::styling::text::measure(
            text,
            label,
            crate::ui::styling::text::TextFormat::new(
                css.text,
                crate::ui::font::Font::system(false),
                14.,
            ),
            16384.,
        )?
    } else {
        text.measure_text(label, 14., false, 16384.)?
    };
    let padding = if css.is_some_and(|c| c.element_padding) {
        0.
    } else {
        16.
    };
    Ok(SizeF {
        width: size.width + if switch { 50. + padding } else { 32. + padding },
        height: (size.height + padding).max(if padding == 0. { 20. } else { 40. }),
    })
}
pub(super) struct ChoicePaint<'a> {
    pub label: &'a Arc<str>,
    pub choice: ChoiceState,
    pub switch: bool,
    pub enabled: bool,
    pub style: &'a ChoiceStyle,
    pub css: Option<&'a crate::ui::styling::ComputedStyle>,
}
pub(super) fn paint(value: ChoicePaint<'_>, ctx: &PaintContext, scene: &mut Scene) {
    let ChoicePaint {
        label,
        choice,
        switch,
        enabled,
        style,
        css,
    } = value;
    if appearance_none(css) {
        return;
    }
    let t = ctx.theme;
    if enabled
        && (ctx.state.hovered || ctx.state.pressed)
        && !css.is_some_and(|c| c.element_background)
    {
        scene.fill(
            ctx.rect,
            8.,
            if ctx.state.pressed {
                t.pressed
            } else {
                t.hover
            },
        );
    }
    let accent = css.and_then(|c| c.local.accent).unwrap_or_else(|| {
        if enabled {
            style.accent.unwrap_or(t.accent)
        } else {
            t.muted
        }
    });
    let border = if enabled {
        style.border.unwrap_or(t.muted)
    } else {
        t.border
    };
    let foreground = css.and_then(|c| c.text.color).unwrap_or_else(|| {
        if enabled {
            style.foreground.unwrap_or(t.text)
        } else {
            t.muted
        }
    });
    let checked = matches!(
        choice,
        ChoiceState::Toggle(CheckState::On | CheckState::Mixed) | ChoiceState::Radio(true)
    );
    let w = if switch { 38. } else { 20. };
    let r = RectF::new(
        ctx.rect.left
            + if css.is_some_and(|c| c.element_padding) {
                0.
            } else {
                8.
            },
        ctx.rect.top + (layout::height(ctx.rect) - 20.).max(0.) / 2.,
        w,
        20.,
    );
    let radius = if switch || matches!(choice, ChoiceState::Radio(_)) {
        10.
    } else {
        4.
    };
    scene.fill(
        r,
        radius,
        if checked {
            accent
        } else {
            style.track.unwrap_or(t.title_bar)
        },
    );
    scene.commands.push(DrawCommand::Stroke {
        rect: r,
        radius,
        width: 1.5,
        color: if checked { accent } else { border },
    });
    if switch {
        scene.fill(
            RectF::new(
                if checked { r.right - 18. } else { r.left + 2. },
                r.top + 2.,
                16.,
                16.,
            ),
            8.,
            if enabled {
                Color::rgb(255, 255, 255)
            } else {
                t.border
            },
        );
    } else if checked {
        let ink = if enabled {
            Color::rgb(255, 255, 255)
        } else {
            t.surface
        };
        match choice {
            ChoiceState::Radio(_) => {
                scene.fill(RectF::new(r.left + 6., r.top + 6., 8., 8.), 4., ink)
            }
            ChoiceState::Toggle(CheckState::Mixed) => {
                scene.fill(RectF::new(r.left + 5., r.top + 9., 10., 2.), 1., ink)
            }
            _ => {
                for (start, end) in [
                    (
                        PointF {
                            x: r.left + 5.,
                            y: r.top + 10.,
                        },
                        PointF {
                            x: r.left + 9.,
                            y: r.top + 14.,
                        },
                    ),
                    (
                        PointF {
                            x: r.left + 9.,
                            y: r.top + 14.,
                        },
                        PointF {
                            x: r.left + 15.,
                            y: r.top + 6.,
                        },
                    ),
                ] {
                    scene.commands.push(DrawCommand::Line {
                        start,
                        end,
                        width: 2.,
                        color: ink,
                    });
                }
            }
        }
    }
    let text_height = (ctx.intrinsic.height
        - if css.is_some_and(|c| c.element_padding) {
            0.
        } else {
            16.
        })
    .max(18.);
    let label_rect = RectF::new(
        r.right + 10.,
        ctx.rect.top + (layout::height(ctx.rect) - text_height).max(0.) / 2.,
        (ctx.rect.right - r.right - 10.).max(0.),
        text_height + 2.,
    );
    if let Some(css) = css {
        crate::ui::styling::text::draw(
            scene,
            label_rect,
            Arc::clone(label),
            crate::ui::styling::text::TextFormat::new(
                css.text,
                crate::ui::font::Font::system(false),
                14.,
            ),
            foreground,
            false,
        );
    } else {
        scene.text_single_line(
            RectF::new(
                r.right + 10.,
                ctx.rect.top + (layout::height(ctx.rect) - text_height).max(0.) / 2.,
                (ctx.rect.right - r.right - 18.).max(0.),
                text_height + 2.,
            ),
            Arc::clone(label),
            14.,
            false,
            foreground,
        );
    }
    if enabled && ctx.state.focused {
        scene.commands.push(DrawCommand::Stroke {
            rect: RectF::new(
                ctx.rect.left + 1.,
                ctx.rect.top + 1.,
                (layout::width(ctx.rect) - 2.).max(0.),
                (layout::height(ctx.rect) - 2.).max(0.),
            ),
            radius: 7.,
            width: 2.,
            color: style.focus.unwrap_or(t.accent),
        });
    }
}
/// La feuille demande `appearance: none` : pas de dessin natif.
pub(super) fn appearance_none(css: Option<&crate::ui::styling::ComputedStyle>) -> bool {
    css.is_some_and(|c| c.local.appearance_none == Some(true))
}
