//! Utilitaires Tailwind dans les templates (`@import "tailwindcss"` dans la feuille) :
//! mise en page, couleurs et variantes, effets, curseurs et défilement.
mod effects;
mod interaction;
mod placement;
mod variants;

use crate::support::Measure;
use karbon_ui::ui::{
    element::SizeF,
    layout,
    prelude::*,
    scene::{PointF, RectF},
    styling::{self, StyleEnvironment},
};
css_file!(mod css, "tailwind.css");

/// Fenêtre de `width` × `height`, pointeur éventuel (`:hover`).
fn environment(width: f32, height: f32, pointer: Option<PointF>) -> StyleEnvironment<'static> {
    StyleEnvironment {
        viewport: SizeF { width, height },
        pointer,
        ..Default::default()
    }
}
fn arrange_in(root: &mut Element, env: StyleEnvironment, w: f32, h: f32) {
    styling::resolve(root, env).unwrap();
    layout::validate(root).unwrap();
    let r = RectF::new(0., 0., w, h);
    layout::arrange(root, r, r, &mut Measure).unwrap();
}
fn arrange(root: &mut Element, w: f32, h: f32) {
    arrange_in(root, environment(w, h, None), w, h);
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.02, "{a} != {b}");
}
