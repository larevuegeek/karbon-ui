//! Thème et contraste : variables `:root` conditionnelles, dégradés et ombres qui lisent
//! les variables, bordures d'un côté, variantes Tailwind `dark:` à variables `--tw-*`.
use crate::support::*;
use karbon_ui::{
    platform::headless::HeadlessApp,
    ui::{prelude::*, rendering::brush::Background, scene::DrawCommand},
};
use std::{cell::Cell, rc::Rc};
css_file!(mod css, "theme.css");

struct Themed(Rc<Cell<ColorPreferences>>);
impl Application for Themed {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let (dark, contrast) = color_scheme();
        let cx = TemplateContext::default();
        let cx = cx.with_variables(css::root_variables(dark, contrast));
        ui! { @context(cx); @styles(css);
            <div class="flex flex-col">
                <p class="card">Texte</p>
                <div class="band"></div>
                <div class="halo"></div>
                <div class="theme-band"></div>
                <div class="w-20 h-5 border-r border-(--line)"></div>
                <div class="w-20 h-5 ring-1 ring-[#050505] dark:ring-[#060606]"></div>
                <div class="w-20 h-5 bg-linear-to-r from-[#010203] to-[#040506] dark:from-[#0a0b0c]"></div>
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn color_preferences(&self) -> ColorPreferences {
        self.0.get()
    }
}

/// Couleurs de toutes les commandes de dessin.
fn colors(app: &HeadlessApp<Themed>) -> Vec<Color> {
    let mut out = Vec::new();
    for command in &app.scene().commands {
        match command {
            DrawCommand::Fill { color, .. }
            | DrawCommand::Stroke { color, .. }
            | DrawCommand::Line { color, .. }
            | DrawCommand::RichText { color, .. }
            | DrawCommand::FormattedText { color, .. }
            | DrawCommand::FontText { color, .. }
            | DrawCommand::Text { color, .. } => out.push(*color),
            DrawCommand::Shape { fill, stroke, .. } => {
                match fill {
                    Some(Background::Solid(color)) => out.push(*color),
                    Some(Background::Linear(g)) => out.extend(g.stops.iter().map(|s| s.color)),
                    Some(Background::Radial(g)) => out.extend(g.stops.iter().map(|s| s.color)),
                    None => {}
                }
                out.extend(stroke.map(|(_, color)| color));
            }
            DrawCommand::Gradient { gradient, .. } => {
                out.extend(gradient.stops.iter().map(|s| s.color));
            }
            _ => {}
        }
    }
    out
}
fn has(colors: &[Color], value: u32) -> bool {
    colors.contains(&Color::hex(value))
}

#[test]
fn conditional_root_variables_reach_every_painted_value() -> karbon_ui::Result<()> {
    let preferences = Rc::new(Cell::new(ColorPreferences {
        scheme: ColorScheme::Dark,
        high_contrast: false,
    }));
    let mut app = HeadlessApp::with_options(Themed(preferences.clone()), options(800., 400.))?;
    let dark = colors(&app);
    for value in [0x222222, 0x333333, 0x111111, 0xabcdef, 0x060606, 0x0a0b0c] {
        assert!(has(&dark, value), "sombre : {value:x} {dark:?}");
    }
    assert!(!has(&dark, 0x123456) && !has(&dark, 0xbbbbbb));

    preferences.set(ColorPreferences {
        scheme: ColorScheme::Light,
        high_contrast: false,
    });
    app.resize(801., 400.)?;
    let light = colors(&app);
    for value in [0xbbbbbb, 0xcccccc, 0xaaaaaa, 0x123456, 0x050505, 0x010203] {
        assert!(has(&light, value), "clair : {value:x} {light:?}");
    }
    assert!(!has(&light, 0xabcdef) && !has(&light, 0x333333));

    preferences.set(ColorPreferences {
        scheme: ColorScheme::Light,
        high_contrast: true,
    });
    app.resize(800., 400.)?;
    assert!(has(&colors(&app), 0xdddddd));
    Ok(())
}
