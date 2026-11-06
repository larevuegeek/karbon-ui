//! Interaction : curseurs, `pointer-events-none`, `overscroll-contain`, `scroll-margin`.
use super::{css, near};
use crate::support::{Measure, Msg, Services, View};
use karbon_ui::ui::{
    prelude::*,
    runtime::{ChromeState, Cursor, UiRuntime},
    scene::PointF,
};

fn runtime(view: fn() -> Element) -> UiRuntime {
    let mut ui = UiRuntime::new(View(view), &Services).unwrap();
    prepare(&mut ui);
    ui
}
fn prepare(ui: &mut UiRuntime) {
    ui.prepare_at(
        ViewContext {
            width: 600.,
            height: 400.,
            theme: Theme::DARK,
            transparent: true,
            backend: "test",
            scale: 1.,
        },
        ChromeState {
            visible: false,
            ..Default::default()
        },
        &mut Measure,
        std::time::Instant::now(),
    )
    .unwrap();
}

fn point(ui: &mut karbon_ui::ui::runtime::UiRuntime, x: f32, y: f32) -> Cursor {
    use karbon_ui::ui::runtime::UiEvent;
    ui.event(UiEvent::PointerMove(PointF { x, y }));
    prepare(ui);
    ui.cursor()
}

#[test]
fn cursor_utilities_choose_the_pointer_shape_and_pointer_events_none_lets_clicks_through() {
    let mut ui = runtime(|| {
        ui! {@styles(css);
            <div class="flex flex-col w-[300px]">
                <div class="cursor-pointer h-[40px]"><span>Carte</span></div>
                <button class="h-[40px] cursor-default" onclick={Msg::Click}>Défaut</button>
                <div class="relative h-[40px]">
                    <button class="w-full h-full" onclick={Msg::Click}>Dessous</button>
                    <div class="absolute inset-0 pointer-events-none bg-black/20 hover:bg-white"></div>
                </div>
                <div class="h-[40px] cursor-grab"><input type="text" value="" readonly class="h-[30px]"></div>
                <button class="h-[40px] cursor-pointer disabled:cursor-not-allowed" disabled={true} onclick={Msg::Click}>Off</button>
            </div>
        }
    });
    assert_eq!(point(&mut ui, 10., 20.), Cursor::Hand);
    assert_eq!(point(&mut ui, 10., 60.), Cursor::Arrow);
    // Le calque sans événements laisse le bouton dessous recevoir le pointeur.
    assert_eq!(point(&mut ui, 10., 100.), Cursor::Hand);
    let overlay = &ui.root().unwrap().children[2].children[1];
    assert_ne!(
        overlay.style.background,
        Some(Color::hex(0xffffff)),
        "no :hover"
    );
    assert_eq!(point(&mut ui, 10., 135.), Cursor::Text);
    assert_eq!(point(&mut ui, 250., 158.), Cursor::Move);
    assert_eq!(point(&mut ui, 10., 180.), Cursor::NotAllowed);
}

#[test]
fn overscroll_contain_keeps_the_rest_of_a_wheel_scroll() {
    use karbon_ui::ui::runtime::UiEvent;
    let mut ui = runtime(|| {
        ui! {@styles(css);
            <div><div class="flex flex-col w-[300px] h-[100px] overflow-y-auto">
                <div class="flex flex-col h-[50px] shrink-0 overflow-y-auto overscroll-contain">
                    <div class="h-[60px] shrink-0"></div>
                </div>
                <div class="h-[200px] shrink-0"></div>
            </div></div>
        }
    });
    ui.event(UiEvent::Scroll {
        position: PointF { x: 10., y: 10. },
        delta_x: 0.,
        delta_y: 40.,
    });
    prepare(&mut ui);
    let outer = &ui.root().unwrap().children[0];
    // Le conteneur intérieur défile de 10 px (sa limite) ; l'extérieur ne bouge pas.
    near(outer.children[0].children[1].bounds().top, 50.);
}

#[test]
fn scroll_margin_keeps_space_around_an_element_scrolled_into_view() {
    use karbon_ui::ui::runtime::{Key, UiEvent};
    let mut ui = runtime(|| {
        ui! {@styles(css);
            <div><div class="flex flex-col w-[300px] h-[100px] overflow-y-auto">
                <button class="h-[60px] shrink-0 scroll-mb-5" onclick={Msg::Click}>Un</button>
                <button class="h-[60px] shrink-0 scroll-mb-5" onclick={Msg::Click}>Deux</button>
                <button class="h-[60px] shrink-0 scroll-mb-5" onclick={Msg::Click}>Trois</button>
                <button class="h-[60px] shrink-0 scroll-mb-5" onclick={Msg::Click}>Quatre</button>
            </div></div>
        }
    });
    for _ in 0..3 {
        ui.event(UiEvent::KeyDown {
            key: Key::Tab,
            shift: false,
            repeat: false,
        });
        prepare(&mut ui);
    }
    let root = &ui.root().unwrap().children[0];
    let third = root.children[0].children[2].bounds();
    // Bas du troisième bouton (180) plus 20 px de marge au bas de la vue (100).
    near(third.bottom - root.bounds().top, 80.);
}
