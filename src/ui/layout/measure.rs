use super::*;
pub(super) fn resolve(length: Length, available: f32, natural: f32) -> f32 {
    match length {
        Length::Computed(value) => value.evaluate(Default::default(), available, None).max(0.),
        Length::Auto | Length::MaxContent => natural,
        Length::Px(n) => n,
        Length::Percent(n) => available * n / 100.,
        Length::Fill(_) => available,
    }
}
/// First layout pass: returns the natural size of `node`, margins included, given the
/// `available` space. Text is measured through `text`. Fails on invalid layout values.
pub fn measure(
    node: &mut Element,
    available: SizeF,
    text: &mut dyn TextMeasurer,
) -> crate::Result<SizeF> {
    let margin = node.style.margin;
    if super::position::hidden(node) {
        return Ok(SizeF::default());
    }
    // `width: max-content` : largeur naturelle sans limite de place (pas de retour à la
    // ligne ni de rétrécissement), retenue ensuite comme une largeur définie.
    if matches!(node.style.width, Length::MaxContent) {
        node.style.width = Length::Auto;
        let unbounded = SizeF {
            width: f32::INFINITY,
            height: available.height,
        };
        let result = match measure(node, unbounded, text) {
            Ok(size) if size.width.is_finite() => Ok(size),
            _ => measure(node, available, text),
        };
        node.style.width = Length::MaxContent;
        return result;
    }
    // Révélation : mesure naturelle avec la hauteur déclarée (pas celle qu'impose le
    // parent), retenue pour `arrange`, puis réduite à la fraction animée.
    if let Some(reveal) = node.rules.as_ref().and_then(|r| r.reveal) {
        let imposed = std::mem::replace(&mut node.style.height, reveal.height);
        let rules = node.layout_rules();
        // Fermeture : l'élément est déjà `hidden`, mesuré comme affiché.
        let hidden = std::mem::replace(&mut rules.hidden, false);
        rules.reveal = None;
        let result = measure(node, available, text);
        node.style.height = imposed;
        let natural = result
            .as_ref()
            .map_or(0., |s| (s.height - margin.vertical()).max(0.));
        let rules = node.layout_rules();
        rules.hidden = hidden;
        rules.reveal = Some(Reveal { natural, ..reveal });
        let mut size = result?;
        size.height = natural * reveal.progress + margin.vertical();
        return Ok(size);
    }
    // Conteneur inchangé depuis l'image précédente : sa mesure d'alors.
    if super::memo::cacheable(node) {
        if let Some((size, natural)) = super::memo::carried(node, available) {
            super::resolve_limits(node, available);
            node.intrinsic = natural;
            return Ok(size);
        }
        let size = measure_in_pass(node, available, text)?;
        super::memo::record(node, available, size);
        return Ok(size);
    }
    measure_in_pass(node, available, text)
}
/// Mesure avec la mémoire de la passe (mêmes contraintes plus tôt dans la passe).
fn measure_in_pass(
    node: &mut Element,
    available: SizeF,
    text: &mut dyn TextMeasurer,
) -> crate::Result<SizeF> {
    // Mêmes contraintes plus tôt dans la passe : taille déjà calculée (voir `memo`).
    let key = match super::memo::lookup(node, available) {
        Some((_, Some((size, natural)))) => {
            super::resolve_limits(node, available);
            node.intrinsic = natural;
            return Ok(size);
        }
        lookup => lookup.map(|(key, _)| key),
    };
    let result = measure_limited(node, available, text);
    if let (Some(key), Ok(size)) = (key, &result) {
        super::memo::put(key, *size, node.intrinsic);
    }
    result
}
/// Mesure de `node` hors révélation : limites relatives, positionnés retirés, boîte,
/// puis `aspect-ratio` et marges.
fn measure_limited(
    node: &mut Element,
    available: SizeF,
    text: &mut dyn TextMeasurer,
) -> crate::Result<SizeF> {
    let margin = node.style.margin;
    super::resolve_limits(node, available);
    let out = super::position::take(node);
    let order = super::position::reorder(node);
    let result = measure_box(
        node,
        SizeF {
            width: (available.width - margin.horizontal()).max(0.),
            height: (available.height - margin.vertical()).max(0.),
        },
        available,
        text,
    );
    super::position::unorder(node, order);
    super::position::restore(node, out);
    let mut measured = result?;
    // aspect-ratio : hauteur auto déduite de la largeur (largeur auto = place offerte).
    if let Some(ratio) = node
        .rules
        .as_ref()
        .and_then(|r| r.aspect_ratio)
        .filter(|r| r.is_finite() && *r > 0.)
        && matches!(node.style.height, Length::Auto | Length::Fill(_))
    {
        let auto = matches!(node.style.width, Length::Auto | Length::Fill(_));
        let width = if auto {
            (available.width - margin.horizontal()).max(0.)
        } else {
            measured.width
        };
        if width.is_finite() && width < 1e6 {
            let (min, max) = limits(node, false);
            measured.height = (width / ratio).clamp(min, max);
            if auto {
                measured.width = width;
            }
        }
    }
    Ok(SizeF {
        width: measured.width + margin.horizontal(),
        height: measured.height + margin.vertical(),
    })
}
fn measure_box(
    node: &mut Element,
    available: SizeF,
    containing: SizeF,
    text: &mut dyn TextMeasurer,
) -> crate::Result<SizeF> {
    let content_box = node.content_box();
    let context = node
        .rules
        .as_ref()
        .map_or(Default::default(), |r| r.unit_context);
    let variables = node.styles.as_ref().and_then(|s| s.variables.clone());
    for (length, reference) in [
        (node.style.width, containing.width),
        (node.style.height, containing.height),
    ] {
        if let Length::Computed(v) = length
            && !v
                .evaluate(context, reference, variables.as_ref())
                .is_finite()
        {
            return Err(invalid("non-finite expression or variable cycle"));
        }
    }
    let resolve = |length, available, natural, reference, edges| match length {
        Length::Computed(value) => {
            value
                .evaluate(context, reference, variables.as_ref())
                .max(0.)
                + if content_box { edges } else { 0. }
        }
        Length::Percent(n) => reference * n / 100. + if content_box { edges } else { 0. },
        Length::Px(n) => n + if content_box { edges } else { 0. },
        _ => self::resolve(length, available, natural),
    };
    let p = box_edges(node);
    let (min_width, max_width) = limits(node, true);
    let (min_height, max_height) = limits(node, false);
    let available = SizeF {
        width: available.width.max(0.),
        height: available.height.max(0.),
    };
    let local = SizeF {
        width: (resolve(
            node.style.width,
            available.width,
            available.width,
            containing.width,
            p.horizontal(),
        )
        .clamp(min_width, max_width)
            - p.left
            - p.right)
            .max(0.),
        height: (resolve(
            node.style.height,
            available.height,
            available.height,
            containing.height,
            p.vertical(),
        )
        .clamp(min_height, max_height)
            - p.top
            - p.bottom)
            .max(0.),
    };
    // `content-visibility` : contenu sauté, de la taille retenue ou déclarée.
    if super::containment::skipped(node) {
        let natural = super::containment::placeholder(node);
        node.intrinsic = natural;
        return Ok(SizeF {
            width: resolve(
                node.style.width,
                available.width,
                natural.width + p.horizontal(),
                containing.width,
                p.horizontal(),
            )
            .clamp(min_width, max_width),
            height: resolve(
                node.style.height,
                available.height,
                natural.height + p.vertical(),
                containing.height,
                p.vertical(),
            )
            .clamp(min_height, max_height),
        });
    }
    let natural = match node.layout {
        Layout::Layers => {
            for child in &mut node.children {
                measure(child, local, text)?;
            }
            local
        }
        // Like CSS `overflow: auto`, a scroller without a definite height is as tall as
        // its content (plus the reserved horizontal bar), never all the offered height:
        // a parent flex column shrinks it, a scroll ancestor sees its real extent.
        // The content is only measured when that height is used (`Auto`), so a sized
        // scroller stays a cheap cut in the measure recursion.
        // Un contenu masqué ou positionné est hors flux (`position::take`) : hauteur nulle.
        Layout::Scroll(axis) => SizeF {
            width: local.width,
            height: if !matches!(node.style.height, Length::Auto) {
                local.height
            } else if let Some(content) = node.children.first_mut() {
                scroll_content_height(content, local, axis, text)?
            } else {
                0.
            },
        },
        Layout::VirtualList => local,
        Layout::Leaf => {
            if let Some(widget) = &node.widget {
                super::tracks::measure_leaf(node.id, local, || widget.measure(local, text))?
            } else {
                SizeF::default()
            }
        }
        Layout::Row | Layout::Column => {
            if node
                .rules
                .as_ref()
                .is_some_and(|r| r.css_flex || r.flex_wrap != FlexWrap::NoWrap)
            {
                let natural = super::flex::natural(node, local, text)?;
                node.intrinsic = natural;
                return Ok(SizeF {
                    width: resolve(
                        node.style.width,
                        available.width,
                        natural.width + p.horizontal(),
                        containing.width,
                        p.horizontal(),
                    )
                    .clamp(min_width, max_width),
                    height: resolve(
                        node.style.height,
                        available.height,
                        natural.height + p.vertical(),
                        containing.height,
                        p.vertical(),
                    )
                    .clamp(min_height, max_height),
                });
            }
            let row = matches!(node.layout, Layout::Row);
            let mut main = 0f32;
            let mut cross = 0f32;
            for child in &mut node.children {
                let size = measure(child, local, text)?;
                main += if row {
                    if matches!(child.style.width, Length::Fill(_)) {
                        limits(child, true).0 + child.style.margin.horizontal()
                    } else {
                        size.width
                    }
                } else if matches!(child.style.height, Length::Fill(_)) {
                    limits(child, false).0 + child.style.margin.vertical()
                } else {
                    size.height
                };
                // Largeur naturelle d'un bloc : celle du contenu de ses enfants, même
                // s'ils remplissent la largeur offerte (`Fill`), comme le max-content CSS.
                let cross_size = if row {
                    size.height
                } else if matches!(child.style.width, Length::Fill(_)) && !hidden(child) {
                    let (min, max) = limits(child, true);
                    (child.intrinsic.width + box_edges(child).horizontal()).clamp(min, max)
                        + child.style.margin.horizontal()
                } else {
                    size.width
                };
                cross = cross.max(cross_size.min(size.width));
            }
            let gap = if row {
                node.style.horizontal_gap()
            } else {
                node.style.vertical_gap()
            };
            main += gap * node.children.len().saturating_sub(1) as f32;
            if row {
                SizeF {
                    width: main,
                    height: cross,
                }
            } else {
                SizeF {
                    width: cross,
                    height: main,
                }
            }
        }
        Layout::Grid { columns } => {
            let columns = super::grid::auto_columns(node, local.width, columns);
            let positions = placements(node, columns)?;
            let rows = positions.iter().map(|p| p.0 + 1).max().unwrap_or(0);
            let mut row_heights = vec![0f32; rows];
            let widths = super::tracks::columns(node, local, &positions, columns, text)?;
            for (child, (r, c, span)) in node.children.iter_mut().zip(positions) {
                let size = measure(
                    child,
                    SizeF {
                        width: widths[c..c + span].iter().sum::<f32>()
                            + node.style.horizontal_gap() * (span - 1) as f32,
                        height: local.height,
                    },
                    text,
                )?;
                row_heights[r] = row_heights[r].max(size.height);
            }
            let gap = node.style.vertical_gap();
            super::tracks::rows(node, &mut row_heights, None, gap);
            let rows = row_heights.len();
            SizeF {
                width: local.width,
                height: row_heights.iter().sum::<f32>()
                    + node.style.vertical_gap() * rows.saturating_sub(1) as f32,
            }
        }
        Layout::Split(split) => match node.children.as_mut_slice() {
            [first, second] => {
                let a = measure(first, local, text)?;
                let b = measure(second, local, text)?;
                if split.axis == Axis::Horizontal {
                    SizeF {
                        width: a.width + b.width + DIVIDER_SIZE,
                        height: a.height.max(b.height),
                    }
                } else {
                    SizeF {
                        width: a.width.max(b.width),
                        height: a.height + b.height + DIVIDER_SIZE,
                    }
                }
            }
            // Un volet masqué ou positionné est hors flux (`position::take`) : l'autre
            // occupe toute la place, sans séparateur.
            [pane] => measure(pane, local, text)?,
            _ => SizeF::default(),
        },
    };
    node.intrinsic = natural;
    Ok(SizeF {
        width: resolve(
            node.style.width,
            available.width,
            natural.width + p.left + p.right,
            containing.width,
            p.horizontal(),
        )
        .clamp(min_width, max_width),
        height: resolve(
            node.style.height,
            available.height,
            natural.height + p.top + p.bottom,
            containing.height,
            p.vertical(),
        )
        .clamp(min_height, max_height),
    })
}
/// The parent has already resolved the main-axis length into this allocation.
pub(super) fn measure_main(
    node: &mut Element,
    available: SizeF,
    row: bool,
    text: &mut dyn TextMeasurer,
) -> crate::Result<SizeF> {
    let previous = if row {
        node.style.width
    } else {
        node.style.height
    };
    let edges = box_edges(node);
    if row {
        node.style.width = Length::Px(
            (available.width
                - node.style.margin.horizontal()
                - if node.content_box() {
                    edges.horizontal()
                } else {
                    0.
                })
            .max(0.),
        );
    } else {
        node.style.height = Length::Px(
            (available.height
                - node.style.margin.vertical()
                - if node.content_box() {
                    edges.vertical()
                } else {
                    0.
                })
            .max(0.),
        );
    }
    let measured = measure(node, available, text);
    if row {
        node.style.width = previous;
    } else {
        node.style.height = previous;
    }
    measured
}
/// Natural height of a scroller: its content laid out in the viewport as
/// [`super::scrolling::place`] does (a horizontal scroller at its scrolled width, plus
/// its reserved bar). A `Fill` content height (stretch to the viewport) counts as `Auto`.
fn scroll_content_height(
    content: &mut Element,
    local: SizeF,
    axis: Axis,
    text: &mut dyn TextMeasurer,
) -> crate::Result<f32> {
    let bar = crate::ui::scroll::BAR_SIZE;
    let viewport = match axis {
        Axis::Horizontal => SizeF {
            width: local.width,
            height: (local.height - bar).max(0.),
        },
        Axis::Vertical => SizeF {
            width: (local.width - bar).max(0.),
            height: local.height,
        },
    };
    let previous = content.style.height;
    if matches!(previous, Length::Fill(_)) {
        content.style.height = Length::Auto;
    }
    let result = measure(content, viewport, text).and_then(|desired| match axis {
        Axis::Horizontal => {
            let extent = desired.width.max(viewport.width);
            measure(
                content,
                SizeF {
                    width: extent,
                    height: viewport.height,
                },
                text,
            )
            .map(|s| s.height + bar)
        }
        Axis::Vertical => Ok(desired.height),
    });
    content.style.height = previous;
    result
}
