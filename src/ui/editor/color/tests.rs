//! Sélecteur de couleur : conversions aller-retour, écriture hexadécimale, gestes sur le
//! carré et les barres (un seul `Commit` par geste, Échap), teinte gardée d'un gris,
//! frappe hexadécimale.
use super::*;
use crate::ui::{
    editor::{
        FieldChange,
        testing::{NONE, at, key, pointer, window},
    },
    element::{Element, ElementId},
    prelude::{Application, ViewContext, column},
    runtime::{KeyCode, KeyPress, PointerButton, PointerPhase},
};

#[test]
fn every_byte_colour_comes_back_from_hue_saturation_and_value() {
    // Toutes les couleurs d'un pas de 15 sur 255 : l'aller-retour rend le même octet.
    let steps: Vec<u8> = (0..=17).map(|i| i * 15).collect();
    for &r in &steps {
        for &g in &steps {
            for &b in &steps {
                let rgb = [r, g, b].map(|c| f32::from(c) / 255.);
                let back = Hsv::from_rgb(rgb).to_rgb();
                let bytes = back.map(|c| (c * 255.).round() as u8);
                assert_eq!(bytes, [r, g, b], "{rgb:?} → {back:?}");
            }
        }
    }
    let grey = Hsv::from_rgb([0.5; 3]);
    assert_eq!((grey.hue, grey.saturation, grey.value), (0., 0., 0.5));
    let blue = Hsv::from_rgb([0., 0., 1.]);
    assert!((blue.hue - 240.).abs() < 1e-3);
}

#[test]
fn hexadecimal_is_written_and_read_back() {
    assert_eq!(hex_of([0., 0.5, 1.]), "#0080FF");
    assert_eq!(parse_hex("#0080FF"), Some([0., 128. / 255., 1.]));
    assert_eq!(parse_hex("0080ff"), parse_hex("#0080FF"));
    assert_eq!(parse_hex("#fff"), Some([1.; 3]));
    for wrong in ["", "#12345", "#GG0000", "rouge", "#1234567"] {
        assert_eq!(parse_hex(wrong), None, "{wrong}");
    }
    // Un octet écrit puis relu reste le même.
    for byte in 0..=255u8 {
        let c = f32::from(byte) / 255.;
        assert_eq!(parse_hex(&hex_of([c, c, c])), Some([c, c, c]));
    }
}

/// Une couleur d'opacité réglable, et les événements reçus.
#[derive(Default)]
struct Paint {
    rgb: [f32; 3],
    alpha: f32,
    colors: ColorPickerState,
    events: Vec<ColorEvent>,
}

impl Application for Paint {
    type Message = ColorMsg;
    fn view(&self, _: &ViewContext) -> Element {
        let picker = ColorPicker::new(7, self.rgb, &self.colors, |m| m)
            .alpha(self.alpha)
            .height(100.)
            .element();
        column("page", [picker]).padding(10.)
    }
    fn update(&mut self, message: ColorMsg) -> bool {
        let reply = self.colors.update(message);
        self.store(reply)
    }
    fn key(&mut self, press: KeyPress) -> bool {
        let reply = self.colors.key(press);
        self.store(reply)
    }
}

impl Paint {
    fn store(&mut self, reply: ColorReply) -> bool {
        if let Some(event) = reply.event {
            self.rgb = event.rgb;
            self.alpha = event.alpha;
            self.events.push(event);
        }
        reply.rebuild
    }

    fn changes(&self) -> Vec<FieldChange> {
        self.events.iter().map(|e| e.change).collect()
    }
}

/// Une couleur opaque `rgb`.
fn opaque(rgb: [f32; 3]) -> Paint {
    Paint {
        rgb,
        alpha: 1.,
        ..Paint::default()
    }
}

fn part(name: &str) -> ElementId {
    ElementId::named("karbon-color-picker").item(7).child(name)
}

#[test]
fn a_drag_in_the_square_previews_then_commits_once() {
    let mut w = window(opaque([0.; 3]), 300., 260.);
    let square = w.bounds(part("square")).expect("carré");
    let width = square.right - square.left;
    let left = Some(PointerButton::Primary);
    let point = at(&w, part("square"), width - 0.5, 0.);
    pointer(&mut w, PointerPhase::Down, left, point, NONE);
    // Le coin haut droit : rouge pur (teinte 0), saturé, clair.
    let [r, g, b] = w.app().rgb;
    assert!(r == 1. && g < 0.01 && b < 0.01, "{:?}", w.app().rgb);
    let middle = at(&w, part("square"), width / 2., 50.);
    pointer(&mut w, PointerPhase::Move, None, middle, NONE);
    pointer(&mut w, PointerPhase::Up, left, middle, NONE);
    assert_eq!(
        w.app().changes(),
        [
            FieldChange::Preview,
            FieldChange::Preview,
            FieldChange::Commit
        ]
    );
    let hsv = Hsv::from_rgb(w.app().rgb);
    assert!((hsv.saturation - 0.5).abs() < 0.01 && (hsv.value - 0.5).abs() < 0.01);
    // Plus rien ne bouge après le relâchement.
    let point = at(&w, part("square"), 5., 5.);
    pointer(&mut w, PointerPhase::Move, None, point, NONE);
    assert_eq!(w.app().events.len(), 3);
}

#[test]
fn escape_puts_the_colour_back_and_a_grey_keeps_its_hue() {
    let mut w = window(opaque([0., 0., 1.]), 300., 260.);
    let left = Some(PointerButton::Primary);
    // La teinte au tiers de la barre (120°, vert), relâchée.
    let hue = w.bounds(part("hue")).expect("barre");
    let third = at(&w, part("hue"), (hue.right - hue.left) / 3., 7.);
    pointer(&mut w, PointerPhase::Down, left, third, NONE);
    pointer(&mut w, PointerPhase::Up, left, third, NONE);
    let green = w.app().rgb;
    assert!(
        green[1] > 0.99 && green[0] < 0.01 && green[2] < 0.01,
        "{green:?}"
    );
    // Le carré tout à gauche : un gris ; le vert reste choisi sur la barre.
    let grey = at(&w, part("square"), 0., 10.);
    pointer(&mut w, PointerPhase::Down, left, grey, NONE);
    pointer(&mut w, PointerPhase::Up, left, grey, NONE);
    assert_eq!(w.app().colors.shown(7, w.app().rgb).hue.round(), 120.);
    // Un glisser abandonné par Échap rend la couleur d'avant lui.
    let before = w.app().rgb;
    let point = at(&w, part("square"), 200., 0.);
    pointer(&mut w, PointerPhase::Down, left, point, NONE);
    assert_ne!(w.app().rgb, before);
    key(&mut w, KeyCode::Escape, NONE);
    assert_eq!(w.app().rgb, before);
    assert_eq!(
        w.app().events.last().map(|e| e.change),
        Some(FieldChange::Cancel)
    );
}

#[test]
fn the_opacity_bar_and_the_typed_value_change_the_colour() {
    let mut w = window(opaque([0.; 3]), 300., 260.);
    let left = Some(PointerButton::Primary);
    let bar = at(&w, part("alpha"), 0., 7.);
    pointer(&mut w, PointerPhase::Down, left, bar, NONE);
    pointer(&mut w, PointerPhase::Up, left, bar, NONE);
    assert_eq!(w.app().alpha, 0.);
    // Le texte tapé ne compte qu'une fois confirmé ; illisible, il ne change rien.
    let typed = |text: &str| ColorMsg(state_text(7, text));
    w.send(typed("#3366CC")).unwrap();
    assert_eq!(w.app().rgb, [0.; 3]);
    w.send(ColorMsg(state_key(crate::ui::runtime::Key::Enter)))
        .unwrap();
    assert_eq!(hex_of(w.app().rgb), "#3366CC");
    assert_eq!(w.app().alpha, 0., "l'opacité ne change pas");
    w.send(typed("bleu")).unwrap();
    w.send(ColorMsg(state_key(crate::ui::runtime::Key::Enter)))
        .unwrap();
    assert_eq!(hex_of(w.app().rgb), "#3366CC");
    // Échap pendant la frappe l'abandonne.
    w.send(typed("#000000")).unwrap();
    w.send(ColorMsg(state_key(crate::ui::runtime::Key::Escape)))
        .unwrap();
    assert!(!w.app().colors.is_active());
    assert_eq!(hex_of(w.app().rgb), "#3366CC");
}

fn state_text(key: u64, text: &str) -> state::Inner {
    state::Inner::HexText {
        key,
        text: text.to_owned(),
        alpha: 0.,
    }
}

fn state_key(pressed: crate::ui::runtime::Key) -> state::Inner {
    state::Inner::HexKey(pressed)
}
