//! Le sélecteur à l'écran : le carré saturation et valeur, la barre de teinte, la barre
//! d'opacité, puis la pastille et la valeur hexadécimale à taper.
use super::{
    convert::{Hsv, byte, hex_of},
    state::{ColorMsg, ColorPickerState, Inner, Part, Shown},
};
use crate::ui::{
    components::{InputStyle, TextInput},
    editor::EditorPalette,
    element::{Element, ElementId, Length, PaintContext, SizeF, TextMeasurer, Widget},
    rendering::brush::{GradientStop, LinearGradient},
    runtime::{Cursor, PointerButton, PointerEvent, PointerPhase},
    scene::{DrawCommand, RectF, Scene},
    theme::Color,
};
use std::{cell::Cell, rc::Rc, sync::Arc};

/// Hauteur des barres de teinte et d'opacité.
const BAR: f32 = 14.;
/// Hauteur du carré par défaut.
const SQUARE: f32 = 120.;
/// Côté des cases du damier qui montre la transparence.
const CHECKER: f32 = 5.;

/// A colour picker for dense editors: a square of saturation (across) and value (up), a
/// bar of hue, a bar of opacity when [`alpha`](Self::alpha) is given, then a sample of
/// the colour and its hexadecimal value to type. Press anywhere in the square or a bar
/// to take the colour there, drag to change it; the application stores the
/// [`ColorEvent`](super::ColorEvent)s that [`ColorPickerState::update`] returns (previews
/// during the drag, one commit at its end). Red, green and blue go from 0 to 1, in the
/// space the application chooses (sRGB for colours shown as they are).
///
/// ```
/// use karbon_ui::ui::{
///     editor::{ColorMsg, ColorPicker, ColorPickerState, FieldChange},
///     prelude::*,
/// };
///
/// #[derive(Clone)]
/// enum Msg {
///     Color(ColorMsg),
/// }
/// #[derive(Default)]
/// struct Paint {
///     rgb: [f32; 3],
///     colors: ColorPickerState,
/// }
/// impl Application for Paint {
///     type Message = Msg;
///     fn view(&self, _: &ViewContext) -> Element {
///         let picker = ColorPicker::new(1, self.rgb, &self.colors, Msg::Color).element();
///         column("page", [picker])
///     }
///     fn update(&mut self, message: Msg) -> bool {
///         let Msg::Color(message) = message;
///         let reply = self.colors.update(message);
///         if let Some(event) = reply.event {
///             self.rgb = event.rgb;
///             if event.change == FieldChange::Commit {
///                 // record the change in the undo history
///             }
///         }
///         reply.rebuild
///     }
/// }
/// ```
pub struct ColorPicker<M> {
    key: u64,
    rgb: [f32; 3],
    hsv: Hsv,
    alpha: Option<f32>,
    square: f32,
    palette: EditorPalette,
    dragging: Option<Part>,
    /// L'opacité du glisser en cours de la barre d'opacité.
    dragged_alpha: Option<f32>,
    typing: Option<String>,
    map: Rc<dyn Fn(ColorMsg) -> M>,
    /// Canaux, intensité, couleurs récentes (`extras.rs`), sous la valeur hexadécimale.
    extras: Vec<Element>,
}

impl<M: 'static> ColorPicker<M> {
    /// The picker of `key` showing `rgb`; `map` wraps its gestures in the application's
    /// message.
    pub fn new(
        key: u64,
        rgb: [f32; 3],
        state: &ColorPickerState,
        map: impl Fn(ColorMsg) -> M + 'static,
    ) -> Self {
        Self {
            key,
            rgb,
            hsv: state.shown(key, rgb),
            alpha: None,
            square: SQUARE,
            palette: EditorPalette::DARK,
            dragging: state.dragging(key),
            dragged_alpha: state.dragged_alpha(key),
            typing: state.typing(key).map(str::to_owned),
            map: Rc::new(map),
            extras: Vec::new(),
        }
    }

    pub(super) fn key(&self) -> u64 {
        self.key
    }

    /// La couleur montrée (celle du glisser en cours).
    pub(super) fn rgb(&self) -> [f32; 3] {
        self.hsv.to_rgb()
    }

    pub(super) fn shown_alpha(&self) -> f32 {
        self.alpha.unwrap_or(1.)
    }

    pub(super) fn map(&self) -> Rc<dyn Fn(ColorMsg) -> M> {
        Rc::clone(&self.map)
    }

    pub(super) fn palette_of(&self) -> EditorPalette {
        self.palette
    }

    pub(super) fn id(&self) -> ElementId {
        super::extras::picker_id(self.key)
    }

    pub(super) fn push_extra(&mut self, element: Element) {
        self.extras.push(element);
    }

    /// Show a bar of opacity, at `alpha` (0: transparent, 1: opaque).
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = Some(self.dragged_alpha.unwrap_or(alpha).clamp(0., 1.));
        self
    }

    /// Height of the square, in logical pixels (120 by default).
    pub fn height(mut self, height: f32) -> Self {
        self.square = height.max(BAR);
        self
    }

    /// Colours of the picker ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The element: the square, the bars and the hexadecimal row, filling the width of
    /// its parent.
    pub fn element(mut self) -> Element {
        let id = self.id();
        let extras = std::mem::take(&mut self.extras);
        let mut children = vec![
            self.part(id, Part::Square, self.square),
            self.part(id, Part::Hue, BAR),
        ];
        if self.alpha.is_some() {
            children.push(self.part(id, Part::Alpha, BAR));
        }
        children.push(self.hex_row(id));
        children.extend(extras);
        Element::column(id.0, children).gap(6.)
    }

    /// Une partie qu'on fait glisser : le carré, une barre.
    fn part(&self, id: ElementId, part: Part, height: f32) -> Element {
        let size = Rc::new(Cell::new((0., 0.)));
        let painter = PartPainter {
            part,
            hsv: self.hsv,
            alpha: self.alpha.unwrap_or(1.),
            size: Rc::clone(&size),
            palette: self.palette,
        };
        let shown = Shown {
            key: self.key,
            hsv: self.hsv,
            alpha: self.alpha.unwrap_or(1.),
            size: (0., 0.),
        };
        let (map, dragging) = (Rc::clone(&self.map), self.dragging == Some(part));
        let mut element = Element::custom(id.child(part_name(part)).0, painter)
            .height(Length::Px(height))
            .width(Length::Fill(1.))
            .on_pointer(move |event: PointerEvent| {
                let shown = Shown {
                    size: size.get(),
                    ..shown
                };
                let message = Inner::Pointer { part, shown, event };
                wanted(event, dragging).then(|| map(ColorMsg(message)))
            });
        element
            .styles
            .get_or_insert_with(Default::default)
            .normal
            .cursor = Some(Cursor::Crosshair);
        element
    }

    /// La pastille de la couleur et sa valeur hexadécimale à taper.
    fn hex_row(&self, id: ElementId) -> Element {
        let p = self.palette;
        let swatch = Element::custom(
            id.child("sample").0,
            Sample {
                rgb: self.hsv.to_rgb(),
                alpha: self.alpha.unwrap_or(1.),
                palette: p,
            },
        )
        .width(Length::Px(p.row_height * 1.6))
        .height(Length::Px(p.row_height));
        let text = self.typing.clone().unwrap_or_else(|| hex_of(self.rgb));
        let (key, alpha) = (self.key, self.alpha.unwrap_or(1.));
        let (on_text, on_key, on_focus) = (
            Rc::clone(&self.map),
            Rc::clone(&self.map),
            Rc::clone(&self.map),
        );
        let input = TextInput::new(text, move |text| {
            on_text(ColorMsg(Inner::HexText { key, text, alpha }))
        })
        .on_key(move |pressed| on_key(ColorMsg(Inner::HexKey(pressed))))
        .label("Hexadecimal colour")
        .style(InputStyle {
            font_size: p.font_size,
            padding: 3.,
            radius: 3.,
            background: Some(p.field),
            foreground: Some(p.text),
            border: Some(p.border),
            focus_border: Some(p.accent),
        });
        let hex = Element::custom(id.child("hex").0, input)
            .height(Length::Px(p.row_height))
            .width(Length::Fill(1.))
            .on_focus(move |focused| on_focus(ColorMsg(Inner::HexFocus(focused))));
        Element::row(id.child("hex-row").0, vec![swatch, hex]).gap(6.)
    }
}

/// Le nom de l'élément d'une partie.
fn part_name(part: Part) -> &'static str {
    match part {
        Part::Square => "square",
        Part::Hue => "hue",
        Part::Alpha => "alpha",
    }
}

/// Événements qu'une partie prend : l'appui du bouton gauche, puis, pendant son glisser,
/// les mouvements et le relâchement.
fn wanted(event: PointerEvent, dragging: bool) -> bool {
    match event.phase {
        PointerPhase::Down => event.button == Some(PointerButton::Primary),
        PointerPhase::Move | PointerPhase::Up => dragging,
        _ => false,
    }
}

/// La couleur d'un rouge, vert, bleu de 0 à 1, d'opacité `alpha`.
fn color_of(rgb: [f32; 3], alpha: f32) -> Color {
    let [r, g, b] = rgb.map(byte);
    Color::rgba(r, g, b, byte(alpha))
}

/// Un dégradé de `from` à `to`, dans la direction `angle` (0 : de gauche à droite, 90 :
/// de haut en bas).
fn gradient(scene: &mut Scene, rect: RectF, radius: f32, angle: f32, stops: &[Color]) {
    let last = (stops.len() - 1).max(1) as f32;
    let stops = stops.iter().enumerate().map(|(i, color)| GradientStop {
        position: i as f32 / last,
        color: *color,
    });
    scene.commands.push(DrawCommand::Gradient {
        rect,
        radius,
        gradient: Arc::new(LinearGradient::new(angle, stops)),
    });
}

/// Un damier clair et sombre sous ce qui peut être transparent.
fn checker(scene: &mut Scene, rect: RectF, radius: f32) {
    scene.fill(rect, radius, Color::hex(0xBFBFBF));
    scene.commands.push(DrawCommand::PushShapeClip {
        rect,
        radii: [radius; 4],
    });
    let mut y = rect.top;
    let mut row = 0;
    while y < rect.bottom {
        let mut x = rect.left + if row % 2 == 0 { 0. } else { CHECKER };
        while x < rect.right {
            let cell = RectF::new(
                x,
                y,
                CHECKER.min(rect.right - x),
                CHECKER.min(rect.bottom - y),
            );
            scene.fill(cell, 0., Color::hex(0x7F7F7F));
            x += CHECKER * 2.;
        }
        y += CHECKER;
        row += 1;
    }
    scene.commands.push(DrawCommand::PopShapeClip);
}

/// Un anneau clair bordé de sombre : la marque de la couleur choisie.
fn ring(scene: &mut Scene, rect: RectF, radius: f32) {
    let outer = RectF {
        left: rect.left - 1.,
        top: rect.top - 1.,
        right: rect.right + 1.,
        bottom: rect.bottom + 1.,
    };
    let stroke = |scene: &mut Scene, rect, radius, width, color| {
        scene.commands.push(DrawCommand::Stroke {
            rect,
            radius,
            width,
            color,
        });
    };
    stroke(scene, outer, radius + 1., 1., Color::rgba(0, 0, 0, 160));
    stroke(scene, rect, radius, 2., Color::rgb(255, 255, 255));
}

/// La peinture d'une partie ; elle retient sa taille pour son gestionnaire du pointeur.
struct PartPainter {
    part: Part,
    hsv: Hsv,
    alpha: f32,
    size: Rc<Cell<(f32, f32)>>,
    palette: EditorPalette,
}

impl Widget for PartPainter {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        let (width, height) = ((r.right - r.left).max(0.), (r.bottom - r.top).max(0.));
        self.size.set((width, height));
        let clear = Color::rgba(0, 0, 0, 0);
        match self.part {
            Part::Square => {
                scene.fill(r, 4., color_of(self.hsv.pure(), 1.));
                let white = Color::rgb(255, 255, 255);
                gradient(scene, r, 4., 0., &[white, Color { alpha: 0, ..white }]);
                gradient(scene, r, 4., 90., &[clear, Color::rgb(0, 0, 0)]);
                let x = r.left + self.hsv.saturation * width;
                let y = r.top + (1. - self.hsv.value) * height;
                ring(scene, RectF::new(x - 6., y - 6., 12., 12.), 6.);
            }
            Part::Hue => {
                let stops: Vec<Color> = (0..=6)
                    .map(|i| {
                        color_of(
                            Hsv {
                                hue: i as f32 * 60.,
                                saturation: 1.,
                                value: 1.,
                            }
                            .to_rgb(),
                            1.,
                        )
                    })
                    .collect();
                gradient(scene, r, height / 2., 0., &stops);
                let x = r.left + self.hsv.hue / 360. * width;
                ring(scene, RectF::new(x - 3., r.top - 1., 6., height + 2.), 3.);
            }
            Part::Alpha => {
                checker(scene, r, height / 2.);
                let rgb = self.hsv.to_rgb();
                gradient(
                    scene,
                    r,
                    height / 2.,
                    0.,
                    &[color_of(rgb, 0.), color_of(rgb, 1.)],
                );
                let x = r.left + self.alpha * width;
                ring(scene, RectF::new(x - 3., r.top - 1., 6., height + 2.), 3.);
            }
        }
        if context.state.focused {
            scene.commands.push(DrawCommand::Stroke {
                rect: r,
                radius: 4.,
                width: 1.,
                color: self.palette.accent,
            });
        }
    }
}

/// La pastille de la couleur, sur un damier quand elle est transparente.
struct Sample {
    rgb: [f32; 3],
    alpha: f32,
    palette: EditorPalette,
}

impl Widget for Sample {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        if self.alpha < 1. {
            checker(scene, r, 3.);
        }
        scene.fill(r, 3., color_of(self.rgb, self.alpha));
        scene.commands.push(DrawCommand::Stroke {
            rect: r,
            radius: 3.,
            width: 1.,
            color: self.palette.border,
        });
    }
}
