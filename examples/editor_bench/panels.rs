//! Les panneaux d'une interface d'éditeur chargée, faits des composants qui existent :
//! barre d'outils, arbre de 100 000 objets (liste virtualisée), vue 3D, frise, 200
//! propriétés, 100 vignettes et ligne d'état.
use crate::{Bench, Msg};
use karbon_ui::ui::{
    element::{PaintContext, SizeF, TextMeasurer},
    prelude::*,
    scene::{DrawCommand, Image, Scene},
    template::ImageSlot,
    virtual_list::VirtualList,
};
use std::sync::Arc;

/// Nombre d'objets de l'arbre.
pub const OBJECTS: usize = 100_000;
/// Nombre de propriétés du panneau de droite.
pub const PROPERTIES: usize = 200;
/// Nombre de vignettes du navigateur de ressources.
pub const THUMBNAILS: usize = 100;

const PANEL: Color = Color::hex(0x1B1E22);
const SELECTED: Color = Color::hex(0x2F4F7A);
const KEY: Color = Color::hex(0xD9A441);
const MUTED: Color = Color::hex(0x8A9099);

/// Toute la fenêtre.
pub fn editor(bench: &Bench) -> Element {
    let center = column("center", [viewport(&bench.slot), timeline()]).width(Length::Fill(1.));
    let main = row(
        "main",
        [
            tree(bench.selected).width(300),
            center,
            properties(bench).width(380),
        ],
    )
    .height(Length::Fill(1.))
    .gap(4.);
    column(
        "root",
        [
            toolbar(),
            main,
            thumbnails(&bench.thumbnails).height(196),
            text("status", bench.status.as_str()).height(22),
        ],
    )
    .gap(4.)
    .padding(4.)
}

fn toolbar() -> Element {
    const TOOLS: [&str; 12] = [
        "Déplacer",
        "Tourner",
        "Échelle",
        "Local",
        "Grille",
        "Aimant",
        "Lumières",
        "Ombres",
        "Fil de fer",
        "Jouer",
        "Pause",
        "Image suivante",
    ];
    row(
        "toolbar",
        TOOLS
            .iter()
            .enumerate()
            .map(|(index, label)| button(100 + index as u64, *label, Msg::Tool(index))),
    )
    .gap(4.)
    .height(36)
}

/// L'arbre des objets : 100 000 lignes, seules les visibles sont construites.
fn tree(selected: usize) -> Element {
    let rows = VirtualList::new(OBJECTS, 22., move |index| {
        let id = 1_000_000 + index as u64 * 4;
        let depth = (index % 4) as f32 * 14.;
        let line = row(
            id,
            [
                column(id + 1, []).width(10).height(10).surface(KEY, 2.),
                text(id + 2, format!("Objet {index:06}")),
                Text::new(kind(index))
                    .size(11.)
                    .color(MUTED)
                    .element(id + 3),
            ],
        )
        .gap(6.)
        .padding(2.)
        .margin(Insets {
            left: depth,
            ..Insets::default()
        });
        if index == selected {
            line.surface(SELECTED, 3.)
        } else {
            line
        }
    })
    .overscan(2);
    Element::virtual_list(10, rows).surface(PANEL, 4.)
}

fn kind(index: usize) -> &'static str {
    ["maillage", "lumière", "caméra", "groupe", "décor"][index % 5]
}

/// La vue 3D : la dernière image livrée par le fil de la vue.
fn viewport(slot: &ImageSlot) -> Element {
    widget("viewport", ViewportWidget { slot: slot.clone() })
        .width(Length::Fill(1.))
        .height(Length::Fill(1.))
}

/// Une frise : 120 clés sur une piste.
fn timeline() -> Element {
    row(
        "timeline",
        (0..120).map(|index| {
            column(30_000 + index as u64, [])
                .width(6)
                .height(if index % 5 == 0 { 28 } else { 14 })
                .surface(KEY, 2.)
        }),
    )
    .gap(4.)
    .padding(6.)
    .height(48)
    .surface(PANEL, 4.)
}

/// 200 propriétés : champs de texte, cases et curseurs, dans un panneau qui défile.
fn properties(bench: &Bench) -> Element {
    let rows = (0..PROPERTIES).map(|index| {
        let id = 10_000 + index as u64 * 4;
        let editor = match index % 10 {
            7 => Checkbox::new("actif", bench.checks[index], move |on| {
                Msg::Toggle(index, on)
            })
            .element(id + 2),
            8 | 9 => Slider::new(bench.slides[index], move |value| Msg::Slide(index, value))
                .range(0., 100.)
                .element(id + 2),
            _ => TextInput::new(bench.values[index].as_str(), move |value| {
                Msg::Edit(index, value)
            })
            .element(id + 2),
        };
        row(
            id,
            [
                text(id + 1, format!("Propriété {index:03}")).width(130),
                editor.width(Length::Fill(1.)),
            ],
        )
        .gap(6.)
    });
    scroll(
        "properties",
        Axis::Vertical,
        column("property-rows", rows).gap(4.).padding(6.),
    )
    .surface(PANEL, 4.)
}

/// 100 vignettes de ressources, douze par ligne.
fn thumbnails(images: &[Arc<Image>]) -> Element {
    let cells = images.iter().enumerate().map(|(index, image)| {
        let id = 20_000 + index as u64 * 4;
        column(
            id,
            [
                widget(
                    id + 1,
                    Thumbnail {
                        image: Arc::clone(image),
                    },
                )
                .width(96)
                .height(64),
                Text::new(format!("ressource_{index:03}"))
                    .size(11.)
                    .element(id + 2),
            ],
        )
        .gap(2.)
    });
    scroll(
        "thumbnails",
        Axis::Vertical,
        grid("thumbnail-grid", 12, cells).gap(6.).padding(6.),
    )
    .surface(PANEL, 4.)
}

/// La vue 3D dans la fenêtre : elle peint l'image de son `ImageSlot`, que le runtime
/// repeint à chaque livraison sans reconstruire la vue.
struct ViewportWidget {
    slot: ImageSlot,
}

impl Widget for ViewportWidget {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> karbon_ui::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        if let Some(image) = self.slot.image() {
            let rect = context.rect;
            scene.commands.push(DrawCommand::PushClip(rect));
            scene.commands.push(DrawCommand::Image { rect, image });
            scene.commands.push(DrawCommand::PopClip);
        }
    }

    fn deferred_image(&self) -> Option<&ImageSlot> {
        Some(&self.slot)
    }
}

/// Une vignette : une petite image fixe.
struct Thumbnail {
    image: Arc<Image>,
}

impl Widget for Thumbnail {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> karbon_ui::Result<SizeF> {
        Ok(SizeF {
            width: 96.,
            height: 64.,
        })
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        scene.commands.push(DrawCommand::Image {
            rect: context.rect,
            image: Arc::clone(&self.image),
        });
    }
}
