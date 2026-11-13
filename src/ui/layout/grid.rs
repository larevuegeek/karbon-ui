use super::*;
pub(super) fn place(
    node: &mut Element,
    area: RectF,
    size: SizeF,
    columns: usize,
    text: &mut dyn TextMeasurer,
    context: LayoutContext,
) -> crate::Result<()> {
    let clip = child_clip(node, area);
    let positions = placements(node, columns)?;
    let rows = positions.iter().map(|p| p.0 + 1).max().unwrap_or(0);
    let mut heights = vec![0f32; rows];
    let column_gap = node.style.horizontal_gap();
    let row_gap = node.style.vertical_gap();
    let widths = super::tracks::columns(node, size, &positions, columns, text)?;
    for (child, &(r, c, span)) in node.children.iter_mut().zip(&positions) {
        let desired = measure(
            child,
            SizeF {
                width: widths[c..c + span].iter().sum::<f32>() + column_gap * (span - 1) as f32,
                height: size.height,
            },
            text,
        )?;
        heights[r] = heights[r].max(desired.height);
    }
    super::tracks::rows(node, &mut heights, Some(size.height), row_gap);
    // justify-content : place les colonnes quand leurs pistes laissent de la place
    // (pistes fixes plus étroites que la grille ; sans effet avec une piste `fr`).
    let used = widths.iter().sum::<f32>() + column_gap * widths.len().saturating_sub(1) as f32;
    let free = (size.width - used).max(0.);
    let (start, column_gap) = match node.style.justify {
        Justify::Center => (free / 2., column_gap),
        Justify::End => (free, column_gap),
        Justify::SpaceBetween if widths.len() > 1 => {
            (0., column_gap + free / (widths.len() - 1) as f32)
        }
        _ => (0., column_gap),
    };
    // Lignes implicites d'une grille de hauteur définie : étirées à parts égales
    // (`align-content: normal`), comme les pistes `auto` d'un navigateur.
    if node.rules.as_ref().is_none_or(|r| r.rows.is_none())
        && matches!(
            node.style.height,
            Length::Px(_) | Length::Percent(_) | Length::Computed(_)
        )
        && !heights.is_empty()
        && size.height.is_finite()
    {
        let used = heights.iter().sum::<f32>() + row_gap * (heights.len() - 1) as f32;
        let extra = (size.height - used) / heights.len() as f32;
        if extra > 0. {
            for h in &mut heights {
                *h += extra;
            }
        }
    }
    // justify-items du conteneur : alignement horizontal par défaut des cellules.
    let justify_items = node.rules.as_ref().and_then(|r| r.justify_items);
    // align-items du conteneur : alignement par défaut des cellules (align-self prime).
    let align_items = match node.style.align {
        Align::Stretch => None,
        other => Some(other),
    };
    for (child, (r, c, span)) in node.children.iter_mut().zip(positions) {
        let y = area.top + heights[..r].iter().sum::<f32>() + row_gap * r as f32;
        let cell_width = widths[c..c + span].iter().sum::<f32>() + column_gap * (span - 1) as f32;
        let desired = measure(
            child,
            SizeF {
                width: cell_width,
                height: heights[r],
            },
            text,
        )?;
        let auto = child.auto_margins();
        let original = child.style.margin;
        let (min_width, max_width) = limits(child, true);
        let (min_height, max_height) = limits(child, false);
        let child_width = if auto.count(true) == 0
            && matches!(child.style.width, Length::Auto | Length::Fill(_))
        {
            cell_width.clamp(
                min_width + child.style.margin.horizontal(),
                max_width + child.style.margin.horizontal(),
            )
        } else {
            desired.width
        };
        let child_height = if auto.count(false) == 0
            && matches!(child.style.height, Length::Auto | Length::Fill(_))
        {
            heights[r].clamp(
                min_height + child.style.margin.vertical(),
                max_height + child.style.margin.vertical(),
            )
        } else {
            desired.height
        };
        let free_x = if auto.count(true) > 0 {
            (cell_width - child_width).max(0.)
        } else {
            0.
        };
        let free_y = if auto.count(false) > 0 {
            (heights[r] - child_height).max(0.)
        } else {
            0.
        };
        // justify-self / align-self dans la cellule (start, center, end).
        let justify = child
            .rules
            .as_ref()
            .and_then(|r| r.justify_self)
            .or(justify_items);
        let align = child
            .rules
            .as_ref()
            .and_then(|r| r.align_self)
            .or(align_items);
        let shift = |a: Option<Align>, free: f32| match a {
            Some(Align::Center) => free / 2.,
            Some(Align::End) => free,
            _ => 0.,
        };
        let aligned =
            |a: Option<Align>| matches!(a, Some(Align::Start | Align::Center | Align::End));
        let (child_width, offset_x) = if aligned(justify) && auto.count(true) == 0 {
            // Largeur `auto` alignée : ajustée au contenu (max-content), pas à la cellule.
            let natural = if matches!(child.style.width, Length::Fill(_)) {
                let fill = std::mem::replace(&mut child.style.width, Length::Auto);
                let natural = measure(
                    child,
                    SizeF {
                        width: cell_width,
                        height: heights[r],
                    },
                    text,
                );
                child.style.width = fill;
                natural?.width
            } else {
                desired.width
            };
            let w = natural.min(cell_width);
            (w, shift(justify, cell_width - w))
        } else {
            (child_width, 0.)
        };
        let (child_height, offset_y) = if aligned(align) && auto.count(false) == 0 {
            let h = desired.height.min(heights[r]);
            (h, shift(align, heights[r] - h))
        } else {
            (child_height, 0.)
        };
        let mut margin = original;
        if auto.count(true) > 0 {
            margin = auto.add(margin, true, free_x / auto.count(true) as f32);
        }
        if auto.count(false) > 0 {
            margin = auto.add(margin, false, free_y / auto.count(false) as f32);
        }
        child.style.margin = margin;
        let result = arrange_in(
            child,
            RectF::new(
                area.left
                    + start
                    + widths[..c].iter().sum::<f32>()
                    + c as f32 * column_gap
                    + offset_x,
                y + offset_y,
                child_width + free_x,
                child_height + free_y,
            ),
            clip,
            text,
            context,
        );
        child.style.margin = original;
        result?;
    }

    Ok(())
}
/// Nombre de colonnes d'une grille `repeat(auto-fill|auto-fit, piste)` pour une
/// largeur de contenu `width` : autant de pistes que la largeur en contient (taille
/// maximale de la piste si elle est définie, sinon minimale, écarts compris), au moins
/// une ; `auto-fit` n'en garde pas plus que d'enfants. Met à jour les pistes et le
/// modèle de la grille ; sans répétition, rend `columns`.
pub(super) fn auto_columns(node: &mut Element, width: f32, columns: usize) -> usize {
    let gap = node.style.horizontal_gap();
    let children = node
        .children
        .iter()
        .filter(|c| !super::position::out_of_flow(c) && !super::position::hidden(c))
        .count();
    let Some(rules) = node.rules.as_mut() else {
        return columns;
    };
    let (Some(fit), Some(track)) = (
        rules.grid_auto_repeat,
        rules.columns.as_ref().and_then(|t| t.first().copied()),
    ) else {
        return columns;
    };
    let definite = |b: TrackBreadth| match b {
        TrackBreadth::Px(n) => Some(n),
        TrackBreadth::Percent(p) if width.is_finite() => Some(width * p / 100.),
        _ => None,
    };
    let size = definite(track.max)
        .or_else(|| definite(track.min))
        .filter(|n| *n > 0.);
    let mut count = match size {
        Some(size) if width.is_finite() => (((width + gap) / (size + gap)).floor() as usize).max(1),
        _ => 1,
    };
    if fit {
        count = count.min(children.max(1));
    }
    let count = count.min(128);
    if rules.columns.as_ref().map_or(0, |t| t.len()) != count {
        rules.columns = Some(std::iter::repeat_n(track, count).collect());
    }
    node.layout = Layout::Grid { columns: count };
    count
}
/// Style d'un enfant dont l'étendue « jusqu'à la dernière colonne » (`1 / -1`,
/// [`Style::SPAN_FULL`]) est résolue pour `columns` colonnes. Dans une grille `fitted`
/// (`auto-fill`, `auto-fit`), le nombre de colonnes suit la largeur : l'étendue y est
/// bornée, et une colonne explicite qui n'y tient plus revient au placement automatique.
fn resolved_span(mut s: Style, columns: usize, fitted: bool) -> Style {
    if s.span == Style::SPAN_FULL {
        s.span = columns
            .saturating_sub(s.column.map_or(0, |c| c.saturating_sub(1)))
            .max(1);
    }
    if fitted {
        s.span = s.span.min(columns);
        if s.column
            .is_some_and(|c| c > 0 && (c - 1).saturating_add(s.span) > columns)
        {
            s.column = None;
        }
    }
    s
}
pub(super) fn placements(
    node: &Element,
    columns: usize,
) -> crate::Result<Vec<(usize, usize, usize)>> {
    if columns == 0 || columns > 128 {
        return Err(invalid("a grid must have between 1 and 128 columns"));
    }
    let fitted = node
        .rules
        .as_ref()
        .is_some_and(|r| r.grid_auto_repeat.is_some());
    let mut occupied = HashSet::new();
    let mut result = Vec::with_capacity(node.children.len());
    // Réserver d'abord les positions explicites pour que l'auto-placement ne les occupe pas.
    for child in node
        .children
        .iter()
        .filter(|c| !super::position::out_of_flow(c))
    {
        let s = resolved_span(child.style, columns, fitted);
        if s.span == 0
            || s.span > columns
            || s.column
                .is_some_and(|c| c == 0 || (c - 1).saturating_add(s.span) > columns)
            || s.row.is_some_and(|r| r == 0 || r > 4096)
        {
            return Err(invalid("invalid grid position or span"));
        }
        if let (Some(r), Some(c)) = (s.row, s.column) {
            for col in c - 1..c - 1 + s.span {
                if !occupied.insert((r - 1, col)) {
                    return Err(invalid("conflicting grid cells"));
                }
            }
        }
    }
    // Placement automatique « sparse » de CSS (sans `dense`) : d'abord les éléments à
    // ligne imposée, chacun après le précédent de sa ligne ; puis les autres dans l'ordre,
    // derrière un curseur qui ne recule jamais (un trou laissé plus haut reste vide, et
    // chaque enfant ne repart pas de la première ligne).
    let flow: Vec<Style> = node
        .children
        .iter()
        .filter(|c| !super::position::out_of_flow(c))
        .map(|c| resolved_span(c.style, columns, fitted))
        .collect();
    let mut placed: Vec<Option<(usize, usize, usize)>> = flow
        .iter()
        .map(|s| match (s.row, s.column) {
            (Some(r), Some(c)) => Some((r - 1, c - 1, s.span)),
            _ => None,
        })
        .collect();
    let fits = |occupied: &HashSet<(usize, usize)>, row: usize, column: usize, span: usize| {
        (column..column + span).all(|c| !occupied.contains(&(row, c)))
    };
    let take = |occupied: &mut HashSet<(usize, usize)>, (row, column, span)| {
        for c in column..column + span {
            occupied.insert((row, c));
        }
    };
    // Ligne imposée : première place libre après le dernier élément placé dans cette ligne.
    let mut row_cursors = std::collections::HashMap::new();
    for (s, slot) in flow.iter().zip(&mut placed) {
        let (Some(r), None) = (s.row, s.column) else {
            continue;
        };
        let row = r - 1;
        let start = row_cursors.get(&row).copied().unwrap_or(0);
        match (start..=columns - s.span).find(|&c| fits(&occupied, row, c, s.span)) {
            Some(column) => {
                take(&mut occupied, (row, column, s.span));
                row_cursors.insert(row, column + s.span);
                *slot = Some((row, column, s.span));
            }
            // Grille ajustée : une ligne imposée que la largeur a remplie n'est pas une
            // erreur, l'élément est placé automatiquement.
            None if fitted => {}
            None => return Err(invalid("grid too large")),
        }
    }
    let mut cursor = (0usize, 0usize);
    for (s, slot) in flow.iter().zip(&mut placed) {
        if slot.is_some() {
            continue;
        }
        let position = match s.column {
            // Colonne imposée à gauche du curseur : ligne suivante.
            Some(c) => {
                let column = c - 1;
                (cursor.0 + usize::from(column < cursor.1)..4096)
                    .find(|&row| fits(&occupied, row, column, s.span))
                    .map(|row| (row, column))
            }
            None => (cursor.0..4096).find_map(|row| {
                let start = if row == cursor.0 { cursor.1 } else { 0 };
                (start..=columns - s.span)
                    .find(|&column| fits(&occupied, row, column, s.span))
                    .map(|column| (row, column))
            }),
        };
        let (row, column) = position.ok_or_else(|| invalid("grid too large"))?;
        take(&mut occupied, (row, column, s.span));
        cursor = (row, column + s.span);
        *slot = Some((row, column, s.span));
    }
    result.extend(placed.into_iter().flatten());
    Ok(result)
}
