//! Effets et texte : pseudo-éléments, transformations, animations du thème, anneaux et
//! ombres en `currentColor`, décorations de texte, dégradés vers un coin.
use super::{arrange, css, near};
use karbon_ui::ui::{layout, prelude::*, styling};

#[test]
fn pseudo_elements_transforms_animations_and_theme_fonts_compile() {
    let mut page = ui! {@styles(css);
        <div><div class="relative w-[100px] h-[50px] before:absolute before:inset-0 before:bg-black/50">
            <div class="translate-x-2 hover:scale-95 transition-transform duration-200 ease-out"></div>
            <div class="animate-glow font-display"></div>
            <div class="animate-[slide_1s_linear] animate-spin"></div>
            <p class="text-sm/6 font-semibold tracking-wide truncate uppercase">Texte</p>
        </div></div>
    };
    arrange(&mut page, 200., 200.);
    let root = &page.children[0];
    assert!(root.children[0].id.0 != 0);
    let pseudo = &root.children[0];
    near(layout::width(pseudo.bounds()), 100.);
    near(layout::height(pseudo.bounds()), 50.);
    assert!(pseudo.style.background.is_some());
}

#[test]
fn rings_without_a_color_use_the_text_color_of_the_element() {
    let mut page = ui! {@styles(css);
        <div class="text-[#ff0000]">
            <div class="ring-2 w-4 h-4"></div>
            <div class="first:ring-1 shadow-[0_0_6px_currentColor] text-[#0000ff] w-4 h-4"></div>
        </div>
    };
    arrange(&mut page, 200., 200.);
    let shadow = |e: &Element| {
        let s = e.computed_style().unwrap().local.shadows.clone().unwrap();
        (s.values[0].spread, s.values[0].blur, s.values[0].color)
    };
    let (spread, _, color) = shadow(&page.children[0]);
    near(spread, 2.);
    assert_eq!(color, Color::hex(0xff0000));
    let (_, blur, color) = shadow(&page.children[1]);
    near(blur, 6.);
    assert_eq!(color, Color::hex(0x0000ff));
}

#[test]
fn decoration_color_offset_and_thickness_reach_the_text_format() {
    let mut page = ui! {@styles(css);
        <div>
            <p class="underline decoration-[#ff0000] underline-offset-2 decoration-[1.5px]">Lien</p>
            <p class="underline hover:decoration-(--panel) underline-offset-auto decoration-solid">Lien</p>
        </div>
    };
    arrange(&mut page, 200., 200.);
    let text = page.children[0].computed_style().unwrap().text;
    let format =
        styling::text::TextFormat::new(text, karbon_ui::ui::font::Font::system(false), 16.);
    assert!(format.decoration.underline);
    assert_eq!(format.decoration_style.color, Some(Color::hex(0xff0000)));
    near(format.decoration_style.offset.unwrap(), 2.);
    near(format.decoration_style.thickness.unwrap(), 1.5);
    let text = page.children[1].computed_style().unwrap().text;
    assert_eq!(text.decoration_color, None);
}

#[test]
fn corner_gradients_follow_the_box_aspect_ratio() {
    let stops = [
        GradientStop {
            position: 0.,
            color: Color::hex(0x000000),
        },
        GradientStop {
            position: 1.,
            color: Color::hex(0xffffff),
        },
    ];
    let g = LinearGradient::to_corner(1, 1, stops);
    near(g.angle_in(100., 100.), 45.);
    // Perpendiculaire à la diagonale qui joint les deux autres coins.
    near(g.angle_in(200., 100.), 63.434948);
    near(
        LinearGradient::to_corner(-1, -1, stops).angle_in(100., 100.),
        -135.,
    );
    let _page: Element = ui! {@styles(css);
        <div class="bg-linear-to-br from-black to-white w-4 h-4"></div>
    };
}
