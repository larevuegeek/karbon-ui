//! Toile d'éléments : 10 000 rectangles dont seuls les visibles produisent des commandes,
//! du zoom de 1 % à 1 000 % ; peinture gardée (KUI-056) identique à la peinture refaite.
use super::canvas::{CanvasItem, CanvasLayer, CanvasView};
use crate::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{
        element::Element,
        prelude::{Application, ViewContext, column, text},
        rendering::filter::Filters,
        scene::{DrawCommand, PointF, RectF, Scene, signature::Signature},
        theme::Color,
    },
};
use std::rc::Rc;

/// 100 × 100 rectangles d'une unité, espacés de deux.
fn grid() -> Rc<Vec<CanvasItem>> {
    let items = (0..10_000).map(|i| {
        let (x, y) = ((i % 100) as f32 * 2., (i / 100) as f32 * 2.);
        CanvasItem::new(RectF::new(x, y, 1., 1.), Color::hex(0x8B9CFF))
    });
    Rc::new(items.collect())
}

/// Une vue sur le coin du monde, au zoom donné (pixels par unité).
fn view(zoom: f32) -> CanvasView {
    CanvasView {
        origin: PointF { x: -0.5, y: 200. },
        zoom: (zoom, zoom),
    }
}

#[test]
fn only_the_visible_items_of_ten_thousand_produce_commands() {
    let items = grid();
    let size = (800., 600.);
    // 100 px par unité : 4 colonnes sur 8 unités, 3 rangées sur 6 unités, en partie.
    let layer = CanvasLayer::new(view(100.), Rc::clone(&items));
    let shown = layer.visible(size).count();
    assert!((12..=25).contains(&shown), "{shown} visible");
    // 1 % : un pixel pour cent unités, tout le monde tient dans la toile.
    assert_eq!(
        CanvasLayer::new(view(0.01), Rc::clone(&items))
            .visible(size)
            .count(),
        10_000
    );
    // 1 000 % de 100 : un seul rectangle, et le temps reste court.
    let started = std::time::Instant::now();
    for zoom in [0.01, 0.1, 1., 10., 100., 1000.] {
        let layer = CanvasLayer::new(view(zoom), Rc::clone(&items));
        let mut scene = Scene::new(Color::hex(0));
        let context = crate::ui::element::PaintContext {
            rect: RectF::new(0., 0., size.0, size.1),
            intrinsic: Default::default(),
            theme: crate::ui::theme::Theme::LIGHT,
            state: Default::default(),
        };
        crate::ui::element::Widget::paint(&layer, &context, &mut scene);
        let fills = scene
            .commands
            .iter()
            .filter(|c| matches!(c, DrawCommand::Fill { .. }))
            .count();
        assert_eq!(fills, layer.visible(size).count(), "zoom {zoom}");
    }
    // Large borne : la suite entière tourne sur tous les cœurs.
    assert!(started.elapsed().as_millis() < 500);
}

/// Une toile et un compteur qui change à côté d'elle.
struct Board {
    items: Rc<Vec<CanvasItem>>,
    zoom: f32,
    counter: u32,
}

impl Application for Board {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let layer = CanvasLayer::new(view(self.zoom), Rc::clone(&self.items))
            .background(Color::hex(0x1E2128))
            .revision(1);
        let canvas = Element::custom(9, layer)
            .width(crate::ui::element::Length::Fill(1.))
            .height(crate::ui::element::Length::Px(300.));
        column(
            "page",
            [canvas, text("counter", format!("{}", self.counter))],
        )
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

fn digest(scene: &Scene) -> u64 {
    Signature::new(
        &scene.commands,
        RectF::new(0., 0., 0., 0.),
        Filters::new(&[]),
        0.,
    )
    .digest()
}

/// Les empreintes des scènes de trois images : compteur changé, puis zoom changé.
fn frames(incremental: bool) -> (Vec<(u64, usize)>, u64) {
    let mut app = HeadlessApp::with_options(
        Board {
            items: grid(),
            zoom: 40.,
            counter: 0,
        },
        HeadlessOptions {
            width: 600.,
            height: 400.,
            ..Default::default()
        },
    )
    .unwrap();
    app.runtime_mut().set_incremental(incremental);
    app.prepare().unwrap();
    let mut out = vec![(digest(app.scene()), app.scene().commands.len())];
    app.update_app(|board| board.counter += 1).unwrap();
    out.push((digest(app.scene()), app.scene().commands.len()));
    app.update_app(|board| board.zoom = 50.).unwrap();
    out.push((digest(app.scene()), app.scene().commands.len()));
    (out, app.runtime().kept_paints_reused())
}

#[test]
fn kept_paints_give_the_same_scene_as_painting_again() {
    let (kept, reused) = frames(true);
    let (painted, none) = frames(false);
    assert_eq!(kept, painted);
    // La toile a été recopiée à l'image qui suit la première et à celle du compteur,
    // pas à celle du zoom.
    assert_eq!(reused, 2);
    assert_eq!(none, 0);
    // Les scènes changent bien d'une image à l'autre (le témoin compare quelque chose).
    assert_ne!(kept[0].0, kept[1].0);
    assert_ne!(kept[1].0, kept[2].0);
}
