//! Dégradés gardés d'une image à l'autre (`gradients.rs`) : rastérisés une fois, recopiés
//! ensuite, retrouvés après un défilement.
use super::*;
use crate::ui::{
    rendering::brush::{GradientStop, LinearGradient},
    scene::{DrawCommand, RectF},
};
use std::{rc::Rc, sync::Arc};

fn scene_at(y: f32) -> Scene {
    let mut scene = clear();
    scene.commands.push(DrawCommand::Gradient {
        rect: RectF::new(0., y, 200., 60.),
        radius: 0.,
        gradient: Arc::new(LinearGradient::new(
            0.,
            [
                GradientStop {
                    position: 0.,
                    color: Color::rgb(0, 0, 0),
                },
                GradientStop {
                    position: 1.,
                    color: Color::rgb(255, 255, 255),
                },
            ],
        )),
    });
    scene
}

#[test]
fn a_scrolled_gradient_reuses_its_rasterization() {
    let mut renderer = renderer();
    let (w, h) = (200u32, 100u32);
    let before = renderer.render(&scene_at(0.), w, h, 1.).unwrap();
    assert_eq!(renderer.caches.gradients.len(), 1);
    let (left, middle, right) = (
        pixel(&before, w, 2, 30),
        pixel(&before, w, 100, 30),
        pixel(&before, w, 197, 30),
    );
    assert!(left[0] < 10 && right[0] > 245, "{left:?} {right:?}");
    assert!((110..146).contains(&middle[0]), "{middle:?}");
    // Défilé de 7 px : même image du dégradé, mêmes pixels décalés.
    let image = renderer.caches.gradients.first();
    let after = renderer.render(&scene_at(7.), w, h, 1.).unwrap();
    assert_eq!(renderer.caches.gradients.len(), 1);
    assert!(Rc::ptr_eq(&image, &renderer.caches.gradients.first()));
    for x in [0, 50, 120, 199] {
        assert_eq!(pixel(&before, w, x, 30), pixel(&after, w, x, 37), "x = {x}");
    }
    // Plus dessiné : libéré à la fin de la peinture suivante.
    renderer.render(&clear(), w, h, 1.).unwrap();
    assert_eq!(renderer.caches.gradients.len(), 0);
}
