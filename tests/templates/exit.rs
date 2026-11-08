//! Sorties `data-exit` imbriquées : le fantôme d'un élément retiré joue aussi la sortie
//! de ses descendants, courbe `cubic-bezier()` et animation Rust calculée par la vue.
use karbon_ui::{
    platform::headless::HeadlessApp,
    ui::{prelude::*, scene::DrawCommand},
};
use std::time::Duration;

css_file!(mod css, "exit.css");

struct Popin {
    open: bool,
}
impl Application for Popin {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        let exit = Animation::fade_out().duration(300).easing(Easing::Linear);
        ui! {@context(cx);@styles(css);
            <div>
                {#if self.open}
                    <div class="fond" data-exit="fade-out 150ms linear" onclick={()}>
                        <div class="dialogue" data-exit="popin-out 300ms cubic-bezier(0, 0, 1, 1)">
                            <p data-exit={exit}>Dialogue</p>
                        </div>
                    </div>
                {/if}
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        self.open = false;
        true
    }
}

/// Opacité cumulée et échelle horizontale des groupes d'effet autour du texte `text`.
fn pose(app: &HeadlessApp<Popin>, text: &str) -> Option<(f32, f32)> {
    let mut stack = Vec::new();
    for command in &app.scene().commands {
        match command {
            DrawCommand::PushEffect {
                opacity, transform, ..
            } => stack.push((*opacity, transform.m11)),
            DrawCommand::PopEffect => {
                stack.pop();
            }
            DrawCommand::RichText { paragraph, .. } if &*paragraph.text == text => {
                return Some(stack.iter().fold((1., 1.), |(o, s), (opacity, scale)| {
                    (o * opacity, s * scale)
                }));
            }
            DrawCommand::Text { text: t, .. }
            | DrawCommand::FormattedText { text: t, .. }
            | DrawCommand::FontText { text: t, .. }
                if &**t == text =>
            {
                return Some(stack.iter().fold((1., 1.), |(o, s), (opacity, scale)| {
                    (o * opacity, s * scale)
                }));
            }
            _ => {}
        }
    }
    None
}

#[test]
fn a_ghost_plays_the_exit_animations_of_its_descendants() {
    let mut app = HeadlessApp::new(Popin { open: true }).unwrap();
    app.advance(Duration::from_millis(100)).unwrap();
    let fond = app.root().unwrap().children[0].id;
    app.click(fond).unwrap();
    assert!(app.find(fond).is_none());
    app.advance(Duration::from_millis(60)).unwrap();
    // Fond (150 ms) × dialogue (300 ms, courbe linéaire écrite en cubic-bezier) × texte
    // (animation Rust de 300 ms).
    let (opacity, scale) = pose(&app, "Dialogue").unwrap();
    let expected = (1. - 60. / 150.) * (1. - 60. / 300.) * (1. - 60. / 300.);
    assert!((opacity - expected).abs() < 0.03, "{opacity} {expected}");
    assert!((scale - (1. - 0.5 * 60. / 300.)).abs() < 0.03, "{scale}");
    // Le fond a fini de s'effacer, le fantôme reste jusqu'à la fin de la plus longue sortie.
    app.advance(Duration::from_millis(150)).unwrap();
    assert!(app.contains_text("Dialogue"));
    assert!(app.runtime().next_wake().is_some());
    app.advance(Duration::from_millis(120)).unwrap();
    assert!(!app.contains_text("Dialogue"));
}
