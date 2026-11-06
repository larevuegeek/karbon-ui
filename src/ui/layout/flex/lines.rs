//! Lignes flex : collecte des éléments en lignes (bases, limites, retour à la ligne) et
//! résolution des longueurs flexibles avec gel itératif des min/max (CSS Flexbox §9.7).
use super::*;
pub(super) struct Item {
    pub(super) index: usize,
    pub(super) main: f32,
    base: f32,
    min: f32,
    max: f32,
    grow: f32,
    shrink: f32,
    shrink_factor: f32,
    frozen: bool,
    violation: f32,
    pub(super) cross: f32,
}
pub(super) struct Line {
    pub(super) items: Vec<Item>,
    pub(super) cross: f32,
}
pub(super) fn axis(size: SizeF, row: bool) -> (f32, f32) {
    if row {
        (size.width, size.height)
    } else {
        (size.height, size.width)
    }
}
pub(super) fn sized(main: f32, cross: f32, row: bool) -> SizeF {
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
pub(super) fn plan(
    node: &mut Element,
    size: SizeF,
    text: &mut dyn TextMeasurer,
) -> crate::Result<Vec<Line>> {
    let row = matches!(node.layout, Layout::Row);
    let css = node.rules.as_ref().is_some_and(|r| r.css_flex);
    let wrap = node
        .rules
        .as_ref()
        .is_some_and(|r| r.flex_wrap != FlexWrap::NoWrap);
    let gap = if row {
        node.style.horizontal_gap()
    } else {
        node.style.vertical_gap()
    };
    let available = axis(size, row).0;
    let mut lines = vec![Line {
        items: Vec::new(),
        cross: 0.,
    }];
    let mut used = 0.;
    for (index, child) in node.children.iter_mut().enumerate() {
        let rule = child.rules.as_deref();
        let original = if row {
            child.style.width
        } else {
            child.style.height
        };
        // `flex-basis: auto` reprend la dimension principale déclarée (CSS Flexbox §7.2).
        let basis = match rule.and_then(|r| r.flex_basis) {
            Some(Length::Auto) | None => original,
            Some(basis) => basis,
        };
        let grow = rule.and_then(|r| r.flex_grow).unwrap_or(if css {
            0.
        } else {
            if let Length::Fill(n) = original {
                n
            } else {
                0.
            }
        });
        let shrink = rule
            .and_then(|r| r.flex_shrink)
            .unwrap_or(if css { 1. } else { 0. });
        let measured_basis = if matches!(basis, Length::Fill(_)) {
            Length::Auto
        } else {
            basis
        };
        if row {
            child.style.width = measured_basis;
        } else {
            child.style.height = measured_basis;
        }
        // Flex bases are measured before their min/max hypothetical sizes.
        let constraints = if row {
            (child.style.min_width, child.style.max_width)
        } else {
            (child.style.min_height, child.style.max_height)
        };
        if row {
            child.style.min_width = 0.;
            child.style.max_width = f32::INFINITY;
        } else {
            child.style.min_height = 0.;
            child.style.max_height = f32::INFINITY;
        }
        let desired = measure(child, size, text);
        // CSS `min-height: auto` of a column item: never shrink below its content.
        let automatic = if css && !row && automatic_minimum(child, constraints.0) {
            Some(match (&desired, measured_basis, original) {
                (Ok(d), Length::Auto, Length::Auto | Length::Fill(_)) => Ok(d.height),
                _ => {
                    child.style.height = Length::Auto;
                    let content = measure(child, size, text).map(|s| s.height);
                    child.style.height = original;
                    match (content, original) {
                        (Ok(content), Length::Px(_) | Length::Percent(_) | Length::Computed(_)) => {
                            measure(child, size, text).map(|s| content.min(s.height))
                        }
                        (content, _) => content,
                    }
                }
            })
        } else {
            None
        };
        if row {
            child.style.width = original;
            child.style.min_width = constraints.0;
            child.style.max_width = constraints.1;
        } else {
            child.style.height = original;
            child.style.min_height = constraints.0;
            child.style.max_height = constraints.1;
        }
        let desired = desired?;
        let automatic = automatic.transpose()?;
        let (natural, cross) = axis(desired, row);
        let margin = if row {
            child.style.margin.horizontal()
        } else {
            child.style.margin.vertical()
        };
        let (min, max) = limits(child, row);
        let max = max + margin;
        // `automatic` is a margin-box height, like `desired`.
        let min = (min + margin).max(automatic.unwrap_or(0.).min(max));
        let base = if !css
            && matches!(original, Length::Fill(_))
            && child.rules.as_ref().is_none_or(|r| r.flex_basis.is_none())
        {
            min
        } else {
            natural
        };
        let main = base.clamp(min, max);
        let edges = box_edges(child);
        let inner_base = (base
            - margin
            - if row {
                edges.horizontal()
            } else {
                edges.vertical()
            })
        .max(0.);
        if wrap && !lines.last().unwrap().items.is_empty() && used + gap + main > available + 0.001
        {
            lines.push(Line {
                items: Vec::new(),
                cross: 0.,
            });
            used = 0.;
        }
        let line = lines.last_mut().unwrap();
        if !line.items.is_empty() {
            used += gap;
        }
        used += main;
        line.cross = line.cross.max(cross);
        line.items.push(Item {
            index,
            main,
            base,
            min,
            max,
            grow,
            shrink: shrink * inner_base,
            shrink_factor: shrink,
            frozen: false,
            violation: 0.,
            cross,
        });
    }
    Ok(lines)
}
/// CSS automatic minimum size (`min-height: auto`) applies to items without an explicit
/// minimum that are not scroll containers: `overflow: hidden | auto | scroll` may shrink
/// (`clip`, the engine default, is not a scroll container in CSS).
fn automatic_minimum(child: &Element, min_height: f32) -> bool {
    let scrolls = |o: Overflow| matches!(o, Overflow::Hidden | Overflow::Auto | Overflow::Scroll);
    min_height == 0.
        && !matches!(child.layout, Layout::Scroll(_) | Layout::VirtualList)
        && child
            .rules
            .as_ref()
            .is_none_or(|r| !scrolls(r.overflow_x) && !scrolls(r.overflow_y))
}
pub(super) fn distribute(line: &mut Line, target: f32) {
    // CSS Flexbox 9.7: preserve bases while freezing min/max violations. Factors
    // whose sum is below one deliberately consume only part of the free space.
    let growing = line.items.iter().map(|i| i.main).sum::<f32>() < target;
    for item in &mut line.items {
        item.frozen = if growing {
            item.grow == 0. || item.base > item.main
        } else {
            item.shrink_factor == 0. || item.base < item.main
        };
    }
    let initial = target
        - line
            .items
            .iter()
            .map(|i| if i.frozen { i.main } else { i.base })
            .sum::<f32>();
    for _ in 0..=line.items.len() {
        for item in &mut line.items {
            if !item.frozen {
                item.main = item.base;
            }
        }
        let mut free = target - line.items.iter().map(|i| i.main).sum::<f32>();
        let factor = line
            .items
            .iter()
            .filter(|i| !i.frozen)
            .map(|i| if growing { i.grow } else { i.shrink_factor })
            .sum::<f32>();
        if factor < 1. && (initial * factor).abs() < free.abs() {
            free = initial * factor;
        }
        let weight = |i: &Item| {
            if i.frozen {
                0.
            } else if growing {
                i.grow
            } else {
                i.shrink
            }
        };
        let total = line.items.iter().map(weight).sum::<f32>();
        if total <= 0. {
            break;
        }
        for item in &mut line.items {
            item.violation = 0.;
            if item.frozen {
                continue;
            }
            let proposed = item.base + free * weight(item) / total;
            item.main = proposed.clamp(item.min, item.max);
            item.violation = item.main - proposed;
        }
        let violation = line.items.iter().map(|i| i.violation).sum::<f32>();
        if violation.abs() < 0.001 {
            break;
        }
        for item in &mut line.items {
            if (violation > 0. && item.violation > 0.) || (violation < 0. && item.violation < 0.) {
                item.frozen = true;
            }
        }
    }
}
