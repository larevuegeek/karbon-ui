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

struct InlineDigits(Rc<Cell<ColorPreferences>>);
impl Application for InlineDigits {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let (dark, contrast) = color_scheme();
        let cx = TemplateContext::default();
        let cx = cx.with_variables(css::root_variables(dark, contrast));
        ui! { @context(cx); @styles(css);
            <div class="flex flex-wrap items-baseline gap-x-4 text-sm">
                {#each [("9 440".to_string(), "titres".to_string())] as c (c.1.as_str())}
                    <span class="whitespace-nowrap"><b class="font-bold text-(--digit)">{c.0}</b> {c.1}</span>
                {/each}
                <p><b class="font-bold text-(--digit)">602</b> albums</p>
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        true
    }
    fn color_preferences(&self) -> ColorPreferences {
        self.0.get()
    }
}

#[test]
fn inline_runs_read_the_variables_of_the_current_scheme() -> karbon_ui::Result<()> {
    let preferences = Rc::new(Cell::new(ColorPreferences {
        scheme: ColorScheme::Light,
        high_contrast: false,
    }));
    let mut app = HeadlessApp::with_options(InlineDigits(preferences.clone()), options(800., 400.))?;
    let light = colors_of(&app);
    assert_eq!(light.len(), 2, "{light:?}");
    assert!(has(&light, 0x1a1a1a) && !has(&light, 0xf1f1f1), "clair : {light:?}");
    // Vue reconstruite sans changement de style : la portion garde la couleur du thème.
    app.send(())?;
    let light = colors_of(&app);
    assert!(has(&light, 0x1a1a1a) && !has(&light, 0xf1f1f1), "clair, après mise à jour : {light:?}");
    Ok(())
}

fn colors_of(app: &HeadlessApp<InlineDigits>) -> Vec<Color> {
    let mut out = Vec::new();
    for command in &app.scene().commands {
        match command {
            DrawCommand::RichText { paragraph, .. } => {
                out.extend(paragraph.spans.iter().filter_map(|r| r.style.color));
            }
            _ => {}
        }
    }
    out
}

struct LightSelect;
impl Application for LightSelect {
    type Message = u64;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div class="flex flex-col p-4">
                <select id="langue" class="w-40 h-9" value={1u64} onchange={|i: u64| i}>
                    <option value={1u64}>Français</option>
                    <option value={2u64}>English</option>
                </select>
            </div>
        }
    }
    fn update(&mut self, _: u64) -> bool {
        false
    }
    fn color_preferences(&self) -> ColorPreferences {
        ColorPreferences {
            scheme: ColorScheme::Light,
            high_contrast: false,
        }
    }
}

#[test]
fn an_open_select_list_follows_the_scheme_the_application_chose() -> karbon_ui::Result<()> {
    // Fenêtre au thème sombre (défaut du harnais), application en clair.
    let mut app = HeadlessApp::with_options(LightSelect, options(400., 300.))?;
    app.click("langue")?;
    let fills = app
        .scene()
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Fill { color, .. } => Some(*color),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(fills.contains(&karbon_ui::ui::theme::Theme::LIGHT.surface), "{fills:?}");
    assert!(!fills.contains(&karbon_ui::ui::theme::Theme::DARK.surface), "{fills:?}");
    Ok(())
}
