//! Chrome de fenêtre dessiné par le runtime : fond, barre de titre et boutons de légende.
use super::super::ChromeState;
use crate::application::ViewContext;
use crate::ui::{
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
    title_bar::{CaptionButton, TITLE_BAR_HEIGHT, button_rect},
};
pub(in crate::ui::runtime) fn paint_chrome(
    scene: &mut Scene,
    ctx: ViewContext,
    chrome: ChromeState,
) {
    let t = ctx.theme;
    let height = ctx.height
        + if chrome.visible {
            TITLE_BAR_HEIGHT as f32
        } else {
            0.
        };
    if !ctx.transparent {
        scene.fill(
            RectF::new(0., 0., ctx.width, height),
            if chrome.maximized { 0. } else { 12. },
            t.background,
        );
    }
    if !chrome.visible {
        return;
    }
    scene.fill(
        RectF::new(0., 0., ctx.width, 40.),
        if chrome.maximized { 0. } else { 12. },
        t.title_bar,
    );
    scene.fill(RectF::new(0., 20., ctx.width, 20.), 0., t.title_bar);
    scene.text(
        RectF::new(18., 10., ctx.width - 180., 24.),
        "NATIVE UI   /   RUST",
        13.,
        true,
        t.accent,
    );
    let area = crate::ui::geometry::Rect {
        left: 0,
        top: 0,
        right: ctx.width as i32,
        bottom: height as i32,
    };
    for button in [
        CaptionButton::Minimize,
        CaptionButton::Maximize,
        CaptionButton::Close,
    ] {
        let r = button_rect(&area, button);
        let active = chrome.hovered == Some(button);
        if active {
            scene.fill(
                RectF::new(r.left as f32, 0., (r.right - r.left) as f32, 40.),
                0.,
                if button == CaptionButton::Close {
                    t.danger
                } else if chrome.pressed == Some(button) {
                    t.pressed
                } else {
                    t.hover
                },
            );
        }
        let color = if active && button == CaptionButton::Close {
            Color::rgb(255, 255, 255)
        } else {
            t.text
        };
        let x = (r.left + r.right) as f32 / 2.;
        let y = 20.;
        let mut line = |x1, y1, x2, y2| {
            scene.commands.push(DrawCommand::Line {
                start: PointF { x: x1, y: y1 },
                end: PointF { x: x2, y: y2 },
                width: 1.,
                color,
            })
        };
        match button {
            CaptionButton::Minimize => line(x - 5., y, x + 5., y),
            CaptionButton::Close => {
                line(x - 5., y - 5., x + 5., y + 5.);
                line(x - 5., y + 5., x + 5., y - 5.);
            }
            CaptionButton::Maximize => {
                if chrome.maximized {
                    scene.commands.push(DrawCommand::Stroke {
                        rect: RectF::new(x - 2., y - 5., 8., 8.),
                        radius: 0.,
                        width: 1.,
                        color,
                    });
                    scene.fill(
                        RectF::new(x - 6., y - 3., 10., 10.),
                        0.,
                        if active { t.hover } else { t.title_bar },
                    );
                }
                scene.commands.push(DrawCommand::Stroke {
                    rect: RectF::new(x - 5., y - 5., 10., 10.),
                    radius: 0.,
                    width: 1.,
                    color,
                });
            }
        }
    }
}
