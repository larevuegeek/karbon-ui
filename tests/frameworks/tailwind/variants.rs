//! Couleurs et variantes : thème, opacité, variables, états, media, `group-*`, `dark:`,
//! `data-*` / `aria-*` / balises, `placeholder:`, `motion-*` ; priorité des classes de
//! la feuille et ordre de Tailwind.
use super::{arrange, arrange_in, css, environment, near};
use karbon_ui::ui::{
    layout,
    prelude::*,
    scene::PointF,
    styling::{self, StyleEnvironment},
};

#[test]
fn colors_use_the_theme_opacity_modifiers_and_variables() {
    let mut root = ui! {@styles(css);
        <div class="w-[10px]">
            <div class="bg-brand-500 h-1"></div>
            <div class="bg-white/50 h-1"></div>
            <div class="bg-(--panel) h-1"></div>
            <div class="bg-accent-500 h-1"></div>
            <div class="bg-[#123]/50 h-1"></div>
        </div>
    };
    arrange(&mut root, 100., 100.);
    let bg = |i: usize| root.children[i].style.background.unwrap();
    assert_eq!(bg(0), Color::hex(0x336699));
    let half = bg(1);
    assert_eq!((half.red, half.green, half.blue), (255, 255, 255));
    assert!((127..=128).contains(&half.alpha), "{half:?}");
    assert_eq!(bg(2), Color::hex(0x102030));
    assert_eq!(bg(3), Color::hex(0x22c55e));
    let mixed = bg(4);
    assert_eq!((mixed.red, mixed.green, mixed.blue), (0x11, 0x22, 0x33));
}

#[test]
fn state_media_group_and_dark_variants_apply_at_resolution() {
    let mut page = ui! {@styles(css);
        <div><div class="w-[100px] h-[100px] bg-black hover:bg-white md:w-[300px]">
            <div class="group h-[20px]">
                <div class="h-[10px] bg-red-500 group-hover:bg-blue-500"></div>
            </div>
            <div class="dark h-[10px]"><div class="h-[10px] bg-black dark:bg-white"></div></div>
            <div class="h-[10px] bg-black dark:bg-white"></div>
        </div></div>
    };
    arrange_in(&mut page, environment(500., 600., None), 1000., 600.);
    let root = &page.children[0];
    assert_eq!(root.style.background, Some(Color::hex(0)));
    near(layout::width(root.bounds()), 100.);
    let inner = |root: &Element| root.children[0].children[0].style.background.unwrap();
    let red = inner(root);
    arrange_in(
        &mut page,
        environment(900., 600., Some(PointF { x: 5., y: 5. })),
        1000.,
        600.,
    );
    let root = &page.children[0];
    assert_eq!(root.style.background, Some(Color::hex(0xffffff)));
    near(layout::width(root.bounds()), 300.);
    assert_ne!(inner(root), red, "group-hover");
    assert_eq!(
        root.children[1].children[0].style.background,
        Some(Color::hex(0xffffff))
    );
    assert_eq!(root.children[2].style.background, Some(Color::hex(0)));
    assert!(root.children[0].classes().contains(&"group"));
}

#[test]
fn stylesheet_classes_win_over_utilities_and_tailwind_order_ignores_attribute_order() {
    let mut root = ui! {@styles(css);
        <div class="flex flex-col w-[200px]">
            <div class="card p-2"><div class="size-1"></div></div>
            <div class="card !p-2"><div class="size-1"></div></div>
            <div class="px-2 p-4"><div class="size-1"></div></div>
            <div class="truncate pl-6"><div class="size-1"></div></div>
        </div>
    };
    arrange(&mut root, 400., 400.);
    let left = |i: usize| root.children[i].children[0].bounds().left;
    near(left(0), 20.);
    near(left(1), 8.);
    near(left(2), 8.);
    near(left(3), 3.);
}

#[test]
fn data_aria_and_tag_variants_read_the_attributes_written_in_the_template() {
    let open = true;
    let state = "closed";
    let mut page = ui! {@styles(css);
        <div class="flex flex-col w-[100px]">
            <div data-variant="error" class="h-4 data-[variant=error]:bg-black data-[variant=info]:bg-white"></div>
            <div data-state={state} class="h-4 bg-white data-[state=open]:bg-black"></div>
            <div aria-expanded={open} class="h-4 aria-expanded:bg-black"></div>
            <div data-active class="group h-4"><div class="h-4 group-data-active:bg-black"></div></div>
            <button onclick={()}><div>Bouton</div></button>
            <div class="h-4 [button+&]:bg-black"></div>
            <div class="flex [&>div:last-child]:bg-black *:h-2"><div></div><div></div></div>
            <div class="h-4 not-data-[x]:bg-black in-data-[variant=error]:bg-white"></div>
        </div>
    };
    arrange(&mut page, 200., 200.);
    let black = Some(Color::hex(0x000000));
    let c = &page.children;
    assert_eq!(c[0].style.background, black);
    assert_eq!(c[1].style.background, Some(Color::hex(0xffffff)));
    assert_eq!(c[2].style.background, black);
    assert_eq!(c[3].children[0].style.background, black);
    assert_eq!(c[5].style.background, black);
    let row = &c[6].children;
    assert_ne!(row[0].style.background, black);
    assert_eq!(row[1].style.background, black);
    near(layout::height(row[0].bounds()), 8.);
    assert_eq!(c[7].style.background, black);
}

#[test]
fn stylesheet_selectors_match_tags_and_attributes() {
    let mut page = ui! {@styles(css);
        <div class="liste flex flex-col">
            <p class="note">Un</p>
            <div data-rang="2" class="rang h-4"></div>
            <div class="note h-4"></div>
        </div>
    };
    arrange(&mut page, 200., 200.);
    let c = &page.children;
    near(c[0].style.padding.top, 2.);
    near(c[1].style.padding.top, 6.);
    near(c[2].style.padding.top, 0.);
}

#[test]
fn placeholder_variant_colors_the_placeholder_text() {
    let mut page = ui! {@styles(css);
        <div class="flex flex-col">
            <input type="text" value="" readonly placeholder="Rechercher" class="text-white placeholder:text-[#ff0000] focus:placeholder:text-[#00ff00]">
        </div>
    };
    arrange(&mut page, 200., 200.);
    let style = page.children[0].computed_style().unwrap();
    assert_eq!(style.local.placeholder_color, Some(Color::hex(0xff0000)));
    assert_eq!(style.text.color, Some(Color::hex(0xffffff)));
}

#[test]
fn motion_variants_follow_the_reduced_motion_setting() {
    let build = || {
        ui! {@styles(css);
            <div><div class="w-4 h-4 motion-safe:bg-black motion-reduce:bg-white"></div></div>
        }
    };
    let mut page = build();
    arrange(&mut page, 200., 200.);
    assert_eq!(
        page.children[0].style.background,
        Some(Color::hex(0x000000))
    );
    let mut page = build();
    styling::resolve(
        &mut page,
        StyleEnvironment {
            reduced_motion: true,
            ..environment(200., 200., None)
        },
    )
    .unwrap();
    assert_eq!(
        page.children[0].style.background,
        Some(Color::hex(0xffffff))
    );
}
