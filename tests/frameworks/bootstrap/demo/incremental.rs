//! Mémoires du moteur sur la démo : défilement, accordéon animé (révélation de hauteur),
//! carte repliable et redimensionnement à travers les points de rupture donnent, avec
//! et sans elles, le même arbre placé, les mêmes textes et les mêmes pixels.
use super::{Demo, demo, scroll_to};
use karbon_ui::{
    platform::headless::HeadlessApp,
    ui::{element::Element, runtime::UiEvent, scene::PointF},
};
use std::time::Duration;

fn snapshot(app: &mut HeadlessApp<Demo>, pixels: bool) -> Vec<String> {
    fn walk(node: &Element, out: &mut Vec<String>) {
        let b = node.bounds();
        out.push(format!(
            "{:?} {:?}",
            node.id,
            [b.left, b.top, b.right, b.bottom].map(f32::to_bits)
        ));
        for child in &node.children {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(app.root().unwrap(), &mut out);
    out.extend(app.texts().iter().map(|t| t.to_string()));
    if pixels {
        out.push(format!("{:?}", app.render(1.).unwrap().pixels));
    }
    out
}

fn play(incremental: bool) -> Vec<Vec<String>> {
    let mut app = demo();
    app.runtime_mut().set_incremental(incremental);
    app.prepare().unwrap();
    let mut steps = vec![snapshot(&mut app, true)];
    let frame = Duration::from_millis(16);
    // Accordéon : le second volet s'ouvre pendant que le premier se referme.
    let second = scroll_to(&mut app, "accordion-button", 1);
    app.click_at(second).unwrap();
    for i in 0..24 {
        app.advance(frame).unwrap();
        steps.push(snapshot(&mut app, i % 6 == 0));
    }
    for i in 0..30 {
        app.event(UiEvent::Scroll {
            position: PointF { x: 640., y: 450. },
            delta_x: 0.,
            delta_y: if i < 20 { 90. } else { -150. },
        })
        .unwrap();
        steps.push(snapshot(&mut app, i % 10 == 0));
    }
    // Redimensionnement à travers les points de rupture de Bootstrap (992, 768, 576).
    for i in 0..36 {
        app.resize(1280. - 23.3 * i as f32, 900.).unwrap();
        steps.push(snapshot(&mut app, i % 9 == 0));
    }
    let first = scroll_to(&mut app, "accordion-button", 0);
    app.click_at(first).unwrap();
    for i in 0..24 {
        app.advance(frame).unwrap();
        app.resize(440. + 35. * i as f32, 900.).unwrap();
        steps.push(snapshot(&mut app, i % 8 == 0));
    }
    steps
}

#[test]
fn caches_give_the_same_page_as_a_full_computation() {
    let cached = play(true);
    let full = play(false);
    assert_eq!(cached.len(), full.len());
    for (step, (a, b)) in cached.iter().zip(&full).enumerate() {
        if a != b {
            let line = a.iter().zip(b).position(|(x, y)| x != y).unwrap_or(0);
            panic!(
                "step {step} differs: {:?} / {:?}",
                a.get(line).map(|s| &s[..s.len().min(200)]),
                b.get(line).map(|s| &s[..s.len().min(200)])
            );
        }
    }
}
