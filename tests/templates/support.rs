//! Outils communs : mesureur de texte fixe et mise en page directe d'un arbre.
use karbon_ui::ui::{
    element::{SizeF, TextMeasurer},
    layout,
    prelude::*,
    scene::RectF,
};

/// Messages inertes : ces tests vérifient la construction, le style et le layout.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Msg {
    Click(u8),
    Text(String),
    Value(f64),
    Checked(bool),
}

/// Mesureur fixe : 0,5 em par caractère, lignes de 1,2 em.
pub struct Measure;
impl TextMeasurer for Measure {
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        _: bool,
        max: f32,
    ) -> karbon_ui::Result<SizeF> {
        Ok(SizeF {
            width: (text.chars().count() as f32 * size * 0.5).min(max),
            height: size * 1.2,
        })
    }
}
/// Mise en page dans 600 × 400, sans résolution des styles.
pub fn arrange(root: &mut Element) {
    let bounds = RectF::new(0., 0., 600., 400.);
    layout::validate(root).unwrap();
    layout::arrange(root, bounds, bounds, &mut Measure).unwrap();
}
