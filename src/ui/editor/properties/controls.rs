//! Les petits contrôles d'une grille de propriétés, à la densité de l'éditeur : case à
//! cocher, texte, liste de choix, pastille de couleur et valeur en lecture seule.
use crate::{
    application::{Action, Handler},
    ui::{
        components::{InputStyle, Select, SelectOption, SelectStyle, Text, TextInput},
        editor::EditorPalette,
        element::{
            CheckState, ChoiceState, Element, ElementId, Length, PaintContext, Role, Semantics,
            SizeF, TextMeasurer, Widget,
        },
        scene::{DrawCommand, PointF, RectF, Scene},
        theme::Color,
    },
};
use std::sync::Arc;

/// A check box of the size of the text: `checked` is `None` when several selected
/// objects differ (drawn « — »); a click or Space sends `on_toggle` with the new state.
///
/// ```
/// use karbon_ui::ui::editor::{EditorPalette, checkbox};
///
/// #[derive(Clone)]
/// enum Msg {
///     CastShadows(bool),
/// }
/// let shadows = checkbox("shadows", Some(true), Msg::CastShadows, &EditorPalette::DARK);
/// # let _ = shadows;
/// ```
pub fn checkbox<M: 'static>(
    id: impl Into<ElementId>,
    checked: Option<bool>,
    on_toggle: impl Fn(bool) -> M + 'static,
    palette: &EditorPalette,
) -> Element {
    let state = match checked {
        Some(true) => CheckState::On,
        Some(false) => CheckState::Off,
        None => CheckState::Mixed,
    };
    let action = Action::toggle(&Handler::new(on_toggle), state.next() == CheckState::On);
    Element::custom(
        id.into().0,
        CheckBox {
            state,
            action,
            palette: *palette,
        },
    )
}

/// A single-line text field at the density of the editor; each edit sends the whole new
/// text through `on_change`.
pub fn text_field<M: 'static>(
    id: impl Into<ElementId>,
    value: impl Into<Arc<str>>,
    on_change: impl Fn(String) -> M + 'static,
    palette: &EditorPalette,
) -> Element {
    let input = TextInput::new(value, on_change).style(InputStyle {
        font_size: palette.font_size,
        padding: 3.,
        radius: 3.,
        background: Some(palette.field),
        foreground: Some(palette.text),
        border: Some(palette.border),
        focus_border: Some(palette.accent),
    });
    Element::custom(id.into().0, input).height(Length::Px(palette.row_height))
}

/// A drop-down list of `options` (key and label), one row high, showing the option of
/// `selected`; a choice sends `on_change` with its key.
///
/// ```
/// use karbon_ui::ui::editor::{EditorPalette, choice};
///
/// #[derive(Clone)]
/// enum Msg {
///     Kind(u64),
/// }
/// let kind = choice("kind", 1, &[(0, "Ponctuelle"), (1, "Spot")], Msg::Kind, &EditorPalette::DARK);
/// # let _ = kind;
/// ```
pub fn choice<M: 'static>(
    id: impl Into<ElementId>,
    selected: u64,
    options: &[(u64, &str)],
    on_change: impl Fn(u64) -> M + 'static,
    palette: &EditorPalette,
) -> Element {
    let options: Vec<SelectOption> = options
        .iter()
        .map(|(key, label)| SelectOption::new(*key, *label))
        .collect();
    let select = Select::new(Some(selected), options, on_change).style(SelectStyle {
        background: Some(palette.field),
        foreground: Some(palette.text),
        border: Some(palette.border),
        focus: Some(palette.accent),
        highlight: Some(palette.hover),
    });
    Element::custom(id.into().0, select).height(Length::Px(palette.row_height))
}

/// A colour sample followed by its hexadecimal value; a click sends `message` (to open
/// a colour picker).
pub fn color_swatch<M: Clone + 'static>(
    id: impl Into<ElementId>,
    color: Color,
    message: M,
    palette: &EditorPalette,
) -> Element {
    Element::custom(
        id.into().0,
        Swatch {
            color,
            hex: format!("#{:02X}{:02X}{:02X}", color.red, color.green, color.blue).into(),
            action: Action::new(message),
            palette: *palette,
        },
    )
    .height(Length::Px(palette.row_height))
}

/// A value shown but not editable, in the secondary text colour.
pub fn read_only(
    id: impl Into<ElementId>,
    value: impl Into<Arc<str>>,
    palette: &EditorPalette,
) -> Element {
    Element::custom(
        id.into().0,
        Text::new(value)
            .size(palette.font_size)
            .color(palette.muted),
    )
}

struct CheckBox {
    state: CheckState,
    action: Action,
    palette: EditorPalette,
}

impl Widget for CheckBox {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: 16.,
            height: 16.,
        })
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let p = &self.palette;
        let r = context.rect;
        // À gauche de sa case : une case à cocher s'aligne sur les autres contrôles.
        let cx = r.left + 9.;
        let cy = (r.top + r.bottom) / 2.;
        let square = RectF::new(cx - 7., cy - 7., 14., 14.);
        let on = self.state != CheckState::Off;
        let hovered = context.state.hovered || context.state.pressed;
        let fill = if on {
            p.accent
        } else if hovered {
            p.hover
        } else {
            p.field
        };
        scene.fill(square, 3., fill);
        scene.commands.push(DrawCommand::Stroke {
            rect: square,
            radius: 3.,
            width: 1.,
            color: if on { p.accent } else { p.border },
        });
        let mark = |x: f32, y: f32| PointF {
            x: cx + x,
            y: cy + y,
        };
        let white = Color::hex(0xFFFFFF);
        match self.state {
            CheckState::On => {
                for (start, end) in [
                    (mark(-4., 0.), mark(-1., 3.)),
                    (mark(-1., 3.), mark(4., -3.)),
                ] {
                    scene.commands.push(DrawCommand::Line {
                        start,
                        end,
                        width: 2.,
                        color: white,
                    });
                }
            }
            CheckState::Mixed => scene.fill(RectF::new(cx - 4., cy - 1., 8., 2.), 1., white),
            CheckState::Off => {}
        }
        if context.state.focused {
            scene.commands.push(DrawCommand::Stroke {
                rect: RectF::new(cx - 9., cy - 9., 18., 18.),
                radius: 4.,
                width: 1.,
                color: p.accent,
            });
        }
    }

    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }

    fn choice(&self) -> Option<ChoiceState> {
        Some(ChoiceState::Toggle(self.state))
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Checkbox,
            name: "",
            enabled: true,
        }
    }
}

struct Swatch {
    color: Color,
    hex: Arc<str>,
    action: Action,
    palette: EditorPalette,
}

impl Widget for Swatch {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: 96.,
            height: self.palette.row_height,
        })
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let p = &self.palette;
        let r = context.rect;
        let height = r.bottom - r.top;
        let sample = RectF::new(r.left, r.top + 3., 34., (height - 6.).max(0.));
        scene.fill(
            sample,
            3.,
            Color {
                alpha: 255,
                ..self.color
            },
        );
        let border = if context.state.hovered || context.state.focused {
            p.accent
        } else {
            p.border
        };
        scene.commands.push(DrawCommand::Stroke {
            rect: sample,
            radius: 3.,
            width: 1.,
            color: border,
        });
        let top = r.top + ((height - p.font_size * 1.35) / 2.).max(0.);
        scene.text_single_line(
            RectF::new(
                r.left + 42.,
                top,
                (r.right - r.left - 42.).max(0.),
                p.font_size * 1.4,
            ),
            Arc::clone(&self.hex),
            p.font_size,
            false,
            p.text,
        );
    }

    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Button,
            name: &self.hex,
            enabled: true,
        }
    }
}
