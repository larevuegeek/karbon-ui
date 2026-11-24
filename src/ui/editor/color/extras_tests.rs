//! Canaux de 0 à 255 au clavier, intensité au-delà de 1, couleurs récentes choisies par
//! un clic visé par la position.
use super::*;
use crate::{
    platform::headless::HeadlessApp,
    ui::{
        editor::{FieldChange, testing::window},
        element::{Element, ElementId},
        prelude::{Application, ViewContext, column},
        runtime::Key,
        scene::PointF,
    },
};

/// Une lumière : sa couleur, son intensité, et ce que le sélecteur montre en plus.
struct Light {
    rgb: [f32; 3],
    intensity: f32,
    extras: bool,
    colors: ColorPickerState,
    events: Vec<ColorEvent>,
}

impl Application for Light {
    type Message = ColorMsg;
    fn view(&self, _: &ViewContext) -> Element {
        let mut picker = ColorPicker::new(1, self.rgb, &self.colors, |m| m);
        if self.extras {
            picker = picker
                .channels(&self.colors)
                .intensity(self.intensity, 20., &self.colors)
                .recent(&self.colors);
        }
        column("page", [picker.element()]).padding(10.)
    }
    fn update(&mut self, message: ColorMsg) -> bool {
        let reply = self.colors.update(message);
        if let Some(event) = reply.event {
            self.rgb = event.rgb;
            self.events.push(event);
        }
        if let Some((1, value, _)) = self.colors.take_intensity() {
            self.intensity = value;
        }
        reply.rebuild
    }
}

fn light(extras: bool) -> HeadlessApp<Light> {
    window(
        Light {
            rgb: [1., 0., 0.],
            intensity: 1.,
            extras,
            colors: ColorPickerState::default(),
            events: Vec::new(),
        },
        300.,
        400.,
    )
}

/// Tab jusqu'au champ du rouge (après la zone hexadécimale), puis une valeur tapée.
fn type_into_red(app: &mut HeadlessApp<Light>, text: &str) {
    app.press(Key::Tab).unwrap(); // zone hexadécimale
    app.press(Key::Tab).unwrap(); // rouge
    app.type_text(text).unwrap();
}

#[test]
fn channels_are_typed_from_the_keyboard_and_join_the_recent_colours() {
    let mut app = light(true);
    type_into_red(&mut app, "64");
    app.press(Key::Tab).unwrap(); // valide, passe au vert
    app.type_text("128").unwrap();
    app.press(Key::Enter).unwrap();
    assert_eq!(app.app().rgb, [64. / 255., 128. / 255., 0.]);
    let commits: Vec<_> = app.app().events.iter().map(|e| e.change).collect();
    assert_eq!(commits, [FieldChange::Commit, FieldChange::Commit]);
    // Les deux couleurs validées sont les récentes, la dernière d'abord.
    let recent = app.app().colors.recent();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0], [64. / 255., 128. / 255., 0., 1.]);
}

#[test]
fn the_intensity_goes_beyond_one_and_the_colour_stays_between_zero_and_one() {
    let mut app = light(true);
    for _ in 0..5 {
        app.press(Key::Tab).unwrap(); // hexadécimal, rouge, vert, bleu, intensité
    }
    app.type_text("12,5").unwrap();
    app.press(Key::Enter).unwrap();
    assert_eq!(app.app().intensity, 12.5);
    assert_eq!(app.app().rgb, [1., 0., 0.]);
    // Passer sur les canaux par Tab valide leur valeur, inchangée.
    assert!(app.app().events.iter().all(|e| e.rgb == [1., 0., 0.]));
}

#[test]
fn a_recent_colour_is_chosen_by_a_click_on_its_swatch() {
    let mut app = light(true);
    type_into_red(&mut app, "10");
    app.press(Key::Enter).unwrap();
    // Une autre couleur, puis un clic sur la seconde pastille (la première validée).
    app.update_app(|light| light.rgb = [0., 0., 1.]).unwrap();
    let swatch = ElementId::named("karbon-color-picker")
        .item(1)
        .child("recent")
        .item(0);
    let r = app.bounds(swatch).expect("one recent colour");
    app.click_at(PointF {
        x: (r.left + r.right) / 2.,
        y: (r.top + r.bottom) / 2.,
    })
    .unwrap();
    assert_eq!(app.app().rgb, [10. / 255., 0., 0.]);
    assert_eq!(app.app().events.last().unwrap().change, FieldChange::Commit);
}

#[test]
fn without_the_extras_tab_stops_at_the_hexadecimal_box_only() {
    // Témoin : sans canaux, le deuxième Tab ne mène à aucun champ ; le texte tapé ne
    // change pas la couleur.
    let mut app = light(false);
    type_into_red(&mut app, "64");
    app.press(Key::Enter).unwrap();
    assert_eq!(app.app().rgb, [1., 0., 0.]);
    assert!(app.app().colors.recent().is_empty());
}
