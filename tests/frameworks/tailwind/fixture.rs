pub(super) use crate::support::Measure;
pub(super) use karbon_ui::ui::{
    element::SizeF,
    layout,
    prelude::*,
    scene::{PointF, RectF},
    styling::{self, StyleEnvironment},
};

/// Fenêtre de `width` × `height`, pointeur éventuel (`:hover`).
pub(super) fn environment(
    width: f32,
    height: f32,
    pointer: Option<PointF>,
) -> StyleEnvironment<'static> {
    StyleEnvironment {
        viewport: SizeF { width, height },
        pointer,
        ..Default::default()
    }
}
pub(super) fn arrange_in(root: &mut Element, env: StyleEnvironment, w: f32, h: f32) {
    styling::resolve(root, env).unwrap();
    layout::validate(root).unwrap();
    let r = RectF::new(0., 0., w, h);
    layout::arrange(root, r, r, &mut Measure).unwrap();
}
pub(super) fn arrange(root: &mut Element, w: f32, h: f32) {
    arrange_in(root, environment(w, h, None), w, h);
}
pub(super) fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.02, "{a} != {b}");
}
