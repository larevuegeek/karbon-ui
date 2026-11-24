//! Les champs à l'écran : la boîte d'un nombre qu'on fait glisser, les trois boîtes d'un
//! vecteur, et la zone de saisie posée sur un champ cliqué.
use super::state::{FieldId, FieldMsg, FieldState, Inner, Spec};
use crate::ui::{
    drag::DragEvent,
    editor::EditorPalette,
    element::{Element, ElementId, Length},
    runtime::{Cursor, PointerButton, PointerEvent, PointerPhase},
};
use std::{rc::Rc, sync::Arc};

/// Lettres des axes d'un vecteur.
pub(super) const AXES: [&str; 3] = ["X", "Y", "Z"];

/// Identifiant de l'élément d'un champ.
pub(super) fn field_element(field: FieldId) -> ElementId {
    ElementId::named("karbon-number-field")
        .item(field.key)
        .child(AXES[field.axis.min(2)])
}

/// A number changed by dragging it sideways, or typed after a click. Shift drags ten
/// times faster, Ctrl ten times finer, Escape (forwarded to [`FieldState::key`]) puts
/// the value back. The value can have bounds, a step (the change for one logical pixel
/// of drag), decimals and a unit. The application keeps the value and a
/// [`FieldState`] and forwards the [`FieldMsg`]s to [`FieldState::update`]. A click
/// replaces the field by a typing box that takes the keyboard (Enter, Tab or a click
/// elsewhere confirms, Escape cancels).
///
/// ```
/// use karbon_ui::ui::{
///     editor::{FieldChange, FieldMsg, FieldState, NumberField},
///     prelude::*,
/// };
///
/// #[derive(Clone)]
/// enum Msg {
///     Field(FieldMsg),
/// }
/// #[derive(Default)]
/// struct Light {
///     intensity: f64,
///     fields: FieldState,
/// }
/// impl Application for Light {
///     type Message = Msg;
///     fn view(&self, _: &ViewContext) -> Element {
///         let field = NumberField::new(1, self.intensity, &self.fields, Msg::Field)
///             .range(0., 100.)
///             .step(0.1)
///             .unit(" lx")
///             .element();
///         column("page", [field])
///     }
///     fn update(&mut self, message: Msg) -> bool {
///         let Msg::Field(message) = message;
///         let reply = self.fields.update(message);
///         if let Some(event) = reply.event {
///             self.intensity = event.value;
///             if event.change == FieldChange::Commit {
///                 // record the change in the undo history
///             }
///         }
///         reply.rebuild
///     }
/// }
/// ```
pub struct NumberField<M> {
    field: FieldId,
    spec: Spec,
    decimals: Option<usize>,
    mixed: bool,
    axis: Option<usize>,
    palette: EditorPalette,
    dragging: bool,
    editing: Option<String>,
    map: Rc<dyn Fn(FieldMsg) -> M>,
    font: Option<crate::ui::font::Font>,
    comma: bool,
    tagged: bool,
    label: Option<Arc<str>>,
    tab_stop: bool,
    payload: Option<PayloadHandler<M>>,
    payload_over: bool,
}

/// Ce qu'un champ répond à ce qu'on lui apporte par un glisser libre (`ui::drag`).
type PayloadHandler<M> = Rc<dyn Fn(&DragEvent, FieldId) -> Option<M>>;

impl<M: 'static> NumberField<M> {
    /// The field of `key` showing `value`; `map` wraps its gestures in the
    /// application's message. Unbounded, step 0.1, decimals from the step.
    pub fn new(
        key: u64,
        value: f64,
        state: &FieldState,
        map: impl Fn(FieldMsg) -> M + 'static,
    ) -> Self {
        Self::with_map(FieldId { key, axis: 0 }, value, state, Rc::new(map))
    }

    fn with_map(
        field: FieldId,
        value: f64,
        state: &FieldState,
        map: Rc<dyn Fn(FieldMsg) -> M>,
    ) -> Self {
        Self {
            field,
            spec: Spec {
                value,
                min: f64::MIN,
                max: f64::MAX,
                step: 0.1,
                decimals: 1,
                unit: "".into(),
            },
            decimals: None,
            mixed: false,
            axis: None,
            palette: EditorPalette::DARK,
            dragging: state.drag.as_ref().is_some_and(|d| d.field == field),
            editing: state
                .edit
                .as_ref()
                .filter(|e| e.field == field)
                .map(|e| e.text.clone()),
            map,
            font: None,
            comma: false,
            label: None,
            tagged: false,
            tab_stop: false,
            payload: None,
            payload_over: false,
        }
    }

    /// Keep the value between `min` and `max`; the field then shows how far it is
    /// between them.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.spec.min = min.min(max);
        self.spec.max = max.max(min);
        self
    }

    /// Change of the value for one logical pixel of drag (0.1 by default); the value
    /// is rounded to it while dragging.
    pub fn step(mut self, step: f64) -> Self {
        self.spec.step = step.abs();
        self
    }

    /// Number of decimals shown (by default, those of the step).
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals.min(12));
        self
    }

    /// Text shown after the value (`" m"`, `"°"`), ignored when it is typed back.
    pub fn unit(mut self, unit: impl Into<Arc<str>>) -> Self {
        self.spec.unit = unit.into();
        self
    }

    /// Show « — » instead of the value: several objects with different values are
    /// selected. Dragging starts from the shown value of the first one.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// Colours of the field ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Show the field as the axis `axis` of a vector (0, 1, 2: X, Y, Z), with its letter
    /// and its colour.
    pub fn axis(mut self, axis: usize) -> Self {
        self.axis = Some(axis.min(2));
        self
    }

    /// Draw the value with `font` (a fixed-width family suits numbers) instead of the
    /// default system font.
    pub fn font(mut self, font: crate::ui::font::Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Show the decimal separator as a comma (« 1,50 »), as in French. Typing accepts
    /// both a point and a comma either way.
    pub fn decimal_comma(mut self, comma: bool) -> Self {
        self.comma = comma;
        self
    }

    /// Show the letter of the axis on a tinted tab at the left edge of the field,
    /// separated from the value by a line of the axis colour, instead of the thin bar.
    pub fn tagged(mut self, tagged: bool) -> Self {
        self.tagged = tagged;
        self
    }

    /// Let Tab and Shift+Tab stop on the field (`false` by default, the field is reached
    /// with the mouse only): when it takes the keyboard, it opens its typing box with
    /// the value selected, as after a click; Tab in that box confirms and goes on to the
    /// next field. The field shows a ring while it has the keyboard.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Let the field receive what is dragged with [`ui::drag`](crate::ui::drag) (a
    /// texture onto a material slot, a value from another field): `handler` gets the
    /// event and the field, returns a message to accept on
    /// [`DragPhase::Over`](crate::ui::drag::DragPhase::Over) (`None` refuses) and the
    /// message of the drop on [`DragPhase::Drop`](crate::ui::drag::DragPhase::Drop),
    /// as with [`Element::on_drag_over`] and [`Element::on_drop`]. Show the accepting
    /// field with [`payload_over`](Self::payload_over).
    pub fn on_payload(
        mut self,
        handler: impl Fn(&DragEvent, FieldId) -> Option<M> + 'static,
    ) -> Self {
        self.payload = Some(Rc::new(handler));
        self
    }

    /// Outline the field in the accent colour: a dragged payload would drop here.
    pub fn payload_over(mut self, over: bool) -> Self {
        self.payload_over = over;
        self
    }

    /// The element: a box one row high that fills its parent's width.
    pub fn element(self) -> Element {
        let Self {
            field,
            mut spec,
            decimals,
            mixed,
            axis,
            palette,
            dragging,
            editing,
            map,
            font,
            comma,
            tagged,
            label,
            tab_stop,
            payload,
            payload_over,
        } = self;
        if let Some(text) = editing {
            return typing(field, &text, palette, map);
        }
        spec.decimals = decimals.unwrap_or_else(|| decimals_of(spec.step));
        spec.value = spec.value.clamp(spec.min, spec.max);
        let id = field_element(field);
        let painter = FieldPainter {
            text: if mixed {
                "—".into()
            } else {
                spec.format(spec.value)
            },
            fraction: fraction(&spec),
            axis,
            dragging: dragging || payload_over,
            palette,
            font,
            tagged,
            tab_stop,
        };
        let painter = if comma {
            FieldPainter {
                text: painter.text.replace('.', ","),
                ..painter
            }
        } else {
            painter
        };
        let entered = tab_stop.then(|| (Rc::clone(&map), spec.clone()));
        let mut element = Element::column(id.0, vec![])
            .height(Length::Px(palette.row_height))
            .painter(painter)
            .on_pointer(move |event: PointerEvent| {
                let message = Inner::Pointer {
                    field,
                    spec: spec.clone(),
                    event,
                };
                wanted(event, dragging).then(|| map(FieldMsg(message)))
            });
        if let Some(label) = label {
            element = element.aria_label(label);
        }
        if let Some(handler) = payload {
            let over = Rc::clone(&handler);
            element = element
                .on_drag_over(move |event: &DragEvent| over(event, field))
                .on_drop(move |event: &DragEvent| handler(event, field));
        }
        if let Some((map, spec)) = entered {
            element = element.on_focus(move |focused| {
                let spec = spec.clone();
                map(FieldMsg(Inner::Enter {
                    field,
                    spec,
                    focused,
                }))
            });
        }
        // Curseur de redimensionnement horizontal : on fait glisser la valeur.
        element
            .styles
            .get_or_insert_with(Default::default)
            .normal
            .cursor = Some(Cursor::ResizeHorizontal);
        element
    }
}

/// Événements qu'un champ prend : l'appui du bouton gauche, puis, pendant son glisser,
/// les mouvements et le relâchement.
fn wanted(event: PointerEvent, dragging: bool) -> bool {
    match event.phase {
        PointerPhase::Down => event.button == Some(PointerButton::Primary),
        PointerPhase::Move | PointerPhase::Up => dragging,
        _ => false,
    }
}

/// Décimales d'un pas : 1 → 0, 0,1 → 1, 0,25 → 2 (au plus 6).
fn decimals_of(step: f64) -> usize {
    let mut decimals = 0;
    let mut scaled = step;
    while decimals < 6 && scaled > 0. && (scaled - scaled.round()).abs() > 1e-9 {
        scaled *= 10.;
        decimals += 1;
    }
    decimals
}

/// Part de la plage où se trouve la valeur, pour un champ borné des deux côtés.
fn fraction(spec: &Spec) -> Option<f32> {
    let bounded = spec.min > f64::MIN && spec.max < f64::MAX && spec.max > spec.min;
    bounded.then(|| ((spec.value - spec.min) / (spec.max - spec.min)) as f32)
}

/// Three number fields side by side, X, Y, Z, each with its letter and the colour of
/// its axis: a position, a rotation, a scale. Its events carry the axis in
/// [`FieldId::axis`]. The settings apply to the three fields.
///
/// ```
/// use karbon_ui::ui::editor::{FieldMsg, FieldState, VectorField};
///
/// #[derive(Clone)]
/// enum Msg {
///     Field(FieldMsg),
/// }
/// let fields = FieldState::default();
/// let position = VectorField::new(7, [1.0, 0.0, -2.5], &fields, Msg::Field)
///     .step(0.01)
///     .unit(" m")
///     .element();
/// # let _ = position;
/// ```
pub struct VectorField<M> {
    fields: [NumberField<M>; 3],
}

impl<M: 'static> VectorField<M> {
    /// The fields of `key` showing `values`.
    pub fn new(
        key: u64,
        values: [f64; 3],
        state: &FieldState,
        map: impl Fn(FieldMsg) -> M + 'static,
    ) -> Self {
        let map: Rc<dyn Fn(FieldMsg) -> M> = Rc::new(map);
        let field = |axis: usize| {
            NumberField::with_map(FieldId { key, axis }, values[axis], state, Rc::clone(&map))
                .axis(axis)
        };
        Self {
            fields: [field(0), field(1), field(2)],
        }
    }

    fn each(self, change: impl Fn(NumberField<M>) -> NumberField<M>) -> Self {
        Self {
            fields: self.fields.map(change),
        }
    }

    /// See [`NumberField::range`].
    pub fn range(self, min: f64, max: f64) -> Self {
        self.each(|f| f.range(min, max))
    }
    /// See [`NumberField::step`].
    pub fn step(self, step: f64) -> Self {
        self.each(|f| f.step(step))
    }
    /// See [`NumberField::decimals`].
    pub fn decimals(self, decimals: usize) -> Self {
        self.each(|f| f.decimals(decimals))
    }
    /// See [`NumberField::unit`].
    pub fn unit(self, unit: impl Into<Arc<str>>) -> Self {
        let unit = unit.into();
        self.each(|f| f.unit(Arc::clone(&unit)))
    }
    /// Show « — » on the axes whose value differs between the selected objects.
    pub fn mixed(self, mixed: [bool; 3]) -> Self {
        let [x, y, z] = self.fields;
        Self {
            fields: [x.mixed(mixed[0]), y.mixed(mixed[1]), z.mixed(mixed[2])],
        }
    }
    /// See [`NumberField::palette`].
    pub fn palette(self, palette: EditorPalette) -> Self {
        self.each(|f| f.palette(palette))
    }
    /// See [`NumberField::font`].
    pub fn font(self, font: crate::ui::font::Font) -> Self {
        self.each(|f| f.font(font))
    }
    /// See [`NumberField::decimal_comma`].
    pub fn decimal_comma(self, comma: bool) -> Self {
        self.each(|f| f.decimal_comma(comma))
    }
    /// See [`NumberField::tagged`].
    pub fn tagged(self, tagged: bool) -> Self {
        self.each(|f| f.tagged(tagged))
    }
    /// See [`NumberField::on_payload`]: each axis is a target, named by
    /// [`FieldId::axis`].
    pub fn on_payload(self, handler: impl Fn(&DragEvent, FieldId) -> Option<M> + 'static) -> Self {
        let handler: PayloadHandler<M> = Rc::new(handler);
        self.each(|f| {
            let handler = Rc::clone(&handler);
            f.on_payload(move |event, field| handler(event, field))
        })
    }
    /// See [`NumberField::payload_over`], for each axis.
    pub fn payload_over(self, over: [bool; 3]) -> Self {
        let [x, y, z] = self.fields;
        Self {
            fields: [
                x.payload_over(over[0]),
                y.payload_over(over[1]),
                z.payload_over(over[2]),
            ],
        }
    }
    /// See [`NumberField::tab_stop`]: Tab goes from X to Y to Z.
    pub fn tab_stop(self, tab_stop: bool) -> Self {
        self.each(|f| f.tab_stop(tab_stop))
    }

    /// The element: a row of three fields sharing its width.
    pub fn element(self) -> Element {
        let key = self.fields[0].field.key;
        let id = ElementId::named("karbon-vector-field").item(key);
        let children = self
            .fields
            .map(|f| f.element().width(Length::Fill(1.)))
            .into();
        Element::row(id.0, children).gap(3.)
    }
}

mod labels;
mod painter;
mod tagged;
mod typing;
use painter::{FieldPainter, with_alpha};
use typing::typing;
