//! Peinture d'un champ multiligne : sélection, chaque ligne visible avec sa propre
//! mise en page, soulignement de composition, caret et indicateur de défilement.
use super::*;
use crate::ui::styling::TextAlign;

impl TextInputs {
    pub(super) fn paint_rows(
        &self,
        id: ElementId,
        e: &Editor,
        input: &TextInput,
        ctx: &PaintContext,
        scene: &mut Scene,
    ) {
        let rows = e.rows.as_ref().unwrap();
        let focused = self.focused == Some(id) && self.active;
        paint_input_container(input, ctx.rect, ctx.theme, focused, scene);
        scene.commands.push(DrawCommand::PushClip(e.clip));
        let align = e.font.map_or(TextAlign::Start, |f| f.align);
        // Lignes visibles : position de leur coin haut gauche dans la fenêtre.
        let visible: Vec<(usize, f32, f32)> = rows
            .lines
            .iter()
            .enumerate()
            .map(|(index, row)| {
                (
                    index,
                    e.rect.left + rows.offset(index, align),
                    e.rect.top + row.top - e.scroll_y,
                )
            })
            .filter(|&(index, _, top)| {
                top < e.clip.bottom && top + rows.lines[index].line.height > e.clip.top
            })
            .collect();
        let range = e.range();
        if focused && e.composition.is_none() && !range.is_empty() {
            for &(index, left, top) in &visible {
                let row = &rows.lines[index];
                for mut rect in row.line.selection(range.clone()) {
                    rect.left += left;
                    rect.right += left;
                    rect.top += top;
                    rect.bottom += top;
                    scene.fill(rect, 0., ctx.theme.pressed);
                }
                // Retour à la ligne sélectionné : une petite marque en fin de ligne.
                if index + 1 < rows.lines.len()
                    && !rows.soft(index)
                    && range.start <= row.range.end
                    && range.end > row.range.end
                {
                    scene.fill(
                        RectF::new(
                            left + row.line.x(row.range.end),
                            top,
                            (input.format().size * 0.3).max(3.),
                            row.line.height,
                        ),
                        0.,
                        ctx.theme.pressed,
                    );
                }
            }
        }
        let color = input
            .css
            .as_ref()
            .and_then(|c| c.text.color)
            .unwrap_or_else(|| {
                if !input.enabled {
                    ctx.theme.muted
                } else {
                    input.appearance().foreground.unwrap_or(ctx.theme.text)
                }
            });
        if e.display.is_empty() {
            let ink = input
                .css
                .as_ref()
                .and_then(|c| c.local.placeholder_color)
                .unwrap_or(ctx.theme.muted);
            let rect = RectF::new(
                e.rect.left,
                e.rect.top,
                layout::width(e.rect),
                layout::height(e.rect).max(rows.height) + 2.,
            );
            let placeholder = Arc::clone(&input.placeholder);
            if input.css.is_some() {
                crate::ui::styling::text::draw(scene, rect, placeholder, input.format(), ink, true);
            } else {
                scene.commands.push(DrawCommand::Text {
                    rect,
                    text: placeholder,
                    size: input.style.font_size,
                    bold: false,
                    color: ink,
                    wrap: true,
                });
            }
        } else {
            let (source, _, _) = e.displayed();
            let mut format = input.format();
            format.align = TextAlign::Start;
            for &(index, left, top) in &visible {
                let row = &rows.lines[index];
                let part = &source[row.range.clone()];
                if part.is_empty() {
                    continue;
                }
                let value: Arc<str> = if input.masked {
                    part.chars().map(|_| '\u{2022}').collect::<String>().into()
                } else {
                    part.into()
                };
                let width = row.line.width.max(layout::width(e.rect));
                if input.css.is_some() {
                    let rect = RectF::new(left, top, width, row.line.height + 2.);
                    crate::ui::styling::text::draw(scene, rect, value, format, color, false);
                } else {
                    let rect = RectF::new(left, top, width + 2., row.line.height + 2.);
                    scene.text_single_line(rect, value, input.style.font_size, false, color);
                }
            }
        }
        if let Some(composed) = e
            .composition
            .as_ref()
            .map(|c| c.range.start..c.range.start + c.value.len())
        {
            for &(index, left, top) in &visible {
                let row = &rows.lines[index];
                for rect in row.line.selection(composed.clone()) {
                    scene.fill(
                        RectF::new(
                            left + rect.left,
                            top + row.line.height - 1.,
                            layout::width(rect),
                            1.,
                        ),
                        0.,
                        ctx.theme.accent,
                    );
                }
            }
        }
        if focused && self.caret_visible {
            scene.fill(e.caret(), 0., color);
        }
        scene.commands.push(DrawCommand::PopClip);
        paint_thumb(e, rows.height, ctx, scene);
    }
}
/// Indicateur de défilement (sans interaction) dans la marge droite du champ quand le
/// texte dépasse sa hauteur.
fn paint_thumb(e: &Editor, content: f32, ctx: &PaintContext, scene: &mut Scene) {
    let track = layout::height(e.rect);
    if content <= track + 0.5 || track <= 0. {
        return;
    }
    let thumb = (track * track / content).clamp(track.min(16.), track);
    let top = e.rect.top + (track - thumb) * (e.scroll_y / (content - track)).clamp(0., 1.);
    let margin = (ctx.rect.right - e.rect.right).max(0.);
    let x = if margin >= 6. {
        e.rect.right + margin / 2. - 2.
    } else {
        e.rect.right - 4.
    };
    // Le clip du texte, élargi à la marge droite si le champ n'est pas rogné à droite.
    let mut clip = e.clip;
    if clip.right >= e.rect.right - 0.5 {
        clip.right = clip.right.max(ctx.rect.right);
    }
    scene.commands.push(DrawCommand::PushClip(clip));
    scene.fill(
        RectF::new(x, top, 4., thumb),
        2.,
        Color {
            alpha: 110,
            ..ctx.theme.muted
        },
    );
    scene.commands.push(DrawCommand::PopClip);
}
