//! Ce qui complète le sélecteur (KUI-035) : les canaux rouge, vert, bleu tapés ou
//! glissés de 0 à 255, l'intensité au-delà de 1 d'une couleur de lumière, et les
//! couleurs récentes. Tout se joint au clavier : les champs s'arrêtent à Tab, une
//! couleur récente se choisit par Entrée.
use super::{
    convert::byte,
    state::{ColorEvent, ColorMsg, ColorPickerState, ColorReply, Inner},
    view::ColorPicker,
};
use crate::{
    application::Action,
    ui::{
        editor::{FieldChange, FieldId, FieldMsg, NumberField},
        element::{
            Element, ElementId, Length, PaintContext, Role, Semantics, SizeF, TextMeasurer, Widget,
        },
        scene::{DrawCommand, Scene},
        theme::Color,
    },
};
use std::rc::Rc;

/// Couleurs récentes gardées.
const RECENT: usize = 12;

/// Clé du champ du canal `axis` (0 rouge, 1 vert, 2 bleu) d'un sélecteur : à part de
/// celles des champs de l'application.
fn channel_key(key: u64, axis: usize) -> u64 {
    (key ^ 0xC010_0000_0000_0000).wrapping_add(axis as u64)
}

/// Le canal d'un champ du sélecteur `key`.
fn channel_of(key: u64, field: u64) -> usize {
    (field.wrapping_sub(channel_key(key, 0)) as usize).min(2)
}

/// Clé du champ d'intensité d'un sélecteur.
fn intensity_key(key: u64) -> u64 {
    key ^ 0x1A7E_0000_0000_0000
}

impl ColorPickerState {
    /// The colours committed lately, the latest first (red, green, blue, opacity from 0
    /// to 1), twelve at most, each once. A picker shows them with
    /// [`ColorPicker::recent`].
    pub fn recent(&self) -> &[[f32; 4]] {
        &self.recent
    }

    /// The intensity changed by the field of [`ColorPicker::intensity`] since the last
    /// call: the picker's key, the value, and whether it is a preview or a commit. Call
    /// it after [`update`](Self::update).
    pub fn take_intensity(&mut self) -> Option<(u64, f32, FieldChange)> {
        self.intensity.take()
    }

    /// Ajoute une couleur validée en tête des récentes.
    pub(super) fn remember(&mut self, rgb: [f32; 3], alpha: f32) {
        let rgba = [rgb[0], rgb[1], rgb[2], alpha];
        let same = |c: &[f32; 4]| c.iter().zip(rgba).all(|(a, b)| byte(*a) == byte(b));
        self.recent.retain(|c| !same(c));
        self.recent.insert(0, rgba);
        self.recent.truncate(RECENT);
    }

    /// Un geste sur un champ de canal ou d'intensité du sélecteur `key`.
    pub(super) fn channel(
        &mut self,
        key: u64,
        rgb: [f32; 3],
        alpha: f32,
        message: FieldMsg,
    ) -> ColorReply {
        self.last_channel = Some((key, rgb, alpha));
        let reply = self.fields.update(message);
        match reply.event {
            Some(event) => self.channel_event(key, rgb, alpha, event),
            None => ColorReply {
                rebuild: reply.rebuild,
                event: None,
            },
        }
    }

    /// La couleur, ou l'intensité, que donne un champ de canal du sélecteur `key`.
    pub(super) fn channel_event(
        &mut self,
        key: u64,
        rgb: [f32; 3],
        alpha: f32,
        event: crate::ui::editor::FieldEvent,
    ) -> ColorReply {
        if event.field.key == intensity_key(key) {
            self.intensity = Some((key, event.value as f32, event.change));
            return ColorReply {
                rebuild: true,
                event: None,
            };
        }
        let mut rgb = rgb;
        rgb[channel_of(key, event.field.key)] = (event.value / 255.).clamp(0., 1.) as f32;
        ColorReply {
            rebuild: true,
            event: Some(ColorEvent {
                key,
                rgb,
                alpha,
                change: event.change,
            }),
        }
    }
}

impl<M: Clone + 'static> ColorPicker<M> {
    /// Show red, green and blue from 0 to 255, each dragged or typed (Tab goes from one
    /// to the next), read from `state`.
    pub fn channels(mut self, state: &ColorPickerState) -> Self {
        let (key, rgb, alpha) = (self.key(), self.rgb(), self.shown_alpha());
        let map = self.map();
        let fields: Vec<Element> = (0..3)
            .map(|axis| {
                let map = Rc::clone(&map);
                let field = FieldId {
                    key: channel_key(key, axis),
                    axis: 0,
                };
                let value = f64::from(byte(rgb[axis]));
                NumberField::new(field.key, value, &state.fields, move |message| {
                    map(ColorMsg(Inner::Channel {
                        key,
                        rgb,
                        alpha,
                        message,
                    }))
                })
                .axis(axis)
                .range(0., 255.)
                .step(1.)
                .palette(self.palette_of())
                .tab_stop(true)
                .element()
                .width(Length::Fill(1.))
            })
            .collect();
        let row = Element::row(self.id().child("channels").0, fields).gap(3.);
        self.push_extra(row);
        self
    }

    /// Show an intensity field, from 0 to `max`, at `value`: the strength of a light's
    /// colour beyond 1 (a colour « HDR »). Read it back with
    /// [`ColorPickerState::take_intensity`]; the colour itself stays from 0 to 1.
    pub fn intensity(mut self, value: f32, max: f32, state: &ColorPickerState) -> Self {
        let (key, rgb, alpha) = (self.key(), self.rgb(), self.shown_alpha());
        let map = self.map();
        let field = NumberField::new(
            intensity_key(key),
            f64::from(value),
            &state.fields,
            move |message| {
                map(ColorMsg(Inner::Channel {
                    key,
                    rgb,
                    alpha,
                    message,
                }))
            },
        )
        .range(0., f64::from(max.max(0.)))
        .step(0.01)
        .unit(" ×")
        .palette(self.palette_of())
        .tab_stop(true)
        .element()
        .width(Length::Fill(1.));
        self.push_extra(field);
        self
    }

    /// Show the recent colours of `state` as swatches: a click (or Enter on a swatch
    /// reached by Tab) commits that colour.
    pub fn recent(mut self, state: &ColorPickerState) -> Self {
        let key = self.key();
        let id = self.id().child("recent");
        let palette = self.palette_of();
        let swatches: Vec<Element> = state
            .recent
            .iter()
            .enumerate()
            .map(|(index, &rgba)| {
                let message = (self.map())(ColorMsg(Inner::Recent { key, rgba }));
                Element::custom(
                    id.item(index as u64).0,
                    Swatch {
                        rgba,
                        action: Action::new(message),
                        accent: palette.accent,
                    },
                )
                .width(Length::Px(palette.row_height))
                .height(Length::Px(palette.row_height))
            })
            .collect();
        if !swatches.is_empty() {
            self.push_extra(Element::row(id.0, swatches).gap(3.));
        }
        self
    }
}

/// Une couleur récente, à choisir.
struct Swatch {
    rgba: [f32; 4],
    action: Action,
    accent: Color,
}

impl Widget for Swatch {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let [r, g, b, a] = self.rgba.map(byte);
        scene.fill(context.rect, 3., Color::rgba(r, g, b, a));
        if context.state.hovered || context.state.focused {
            scene.commands.push(DrawCommand::Stroke {
                rect: context.rect,
                radius: 3.,
                width: 2.,
                color: self.accent,
            });
        }
    }

    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Button,
            name: "Recent colour",
            enabled: true,
        }
    }
}

/// L'identifiant de l'élément d'un sélecteur.
pub(super) fn picker_id(key: u64) -> ElementId {
    ElementId::named("karbon-color-picker").item(key)
}
