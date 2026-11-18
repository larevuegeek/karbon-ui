//! Mise en page : espacements, tailles, flex, grilles (placement responsive, `auto-fill`,
//! `col-span-full`), `space-*`, limites en pourcentage et en expression, translations en
//! pourcentage, `inline` / `inline-block`.
use super::{arrange, arrange_in, css, environment, near};
use crate::support::{Msg, find};
use karbon_ui::ui::{layout, prelude::*};

#[test]
fn spacing_sizes_and_flex_utilities_lay_out_like_tailwind() {
    let mut page = ui! {@styles(css);
        <div><div class="flex items-center gap-2 p-4 w-[200px] h-[100px]">
            <div class="size-8 shrink-0"></div>
            <div class="flex-1 h-4"></div>
        </div></div>
    };
    arrange(&mut page, 400., 300.);
    let root = &page.children[0];
    near(layout::width(root.bounds()), 200.);
    let (a, b) = (root.children[0].bounds(), root.children[1].bounds());
    near(a.left, 16.);
    near(layout::width(a), 32.);
    near(a.top, 34.);
    near(b.left, 56.);
    near(layout::width(b), 128.);
    near(layout::height(b), 16.);
}

#[test]
fn place_items_center_centers_grid_items_and_text() {
    let mut page = ui! {@styles(css);
        <div><div class="grid place-items-center w-[30px] h-[30px] text-[10px] leading-none">D</div></div>
    };
    arrange(&mut page, 400., 300.);
    let avatar = &page.children[0];
    let text = avatar.children[0].bounds();
    assert!(layout::width(text) < 10., "shrinks to the text");
    near(text.left + layout::width(text) / 2., 15.);
    near(text.top + layout::height(text) / 2., 15.);
}

#[test]
fn space_utilities_set_margins_on_all_children_but_the_last() {
    let mut page = ui! {@styles(css);
        <div>
            <div class="flex flex-col space-y-2 w-[50px]">
                <div class="h-4"></div><div class="h-4"></div><div class="h-4 mt-1"></div>
            </div>
            <div class="flex space-x-3 md:space-x-1 h-[10px]">
                <div class="w-4"></div><div class="w-4"></div>
            </div>
        </div>
    };
    arrange(&mut page, 400., 300.);
    let column = &page.children[0];
    let tops = column
        .children
        .iter()
        .map(|c| c.bounds().top)
        .collect::<Vec<_>>();
    near(tops[1] - tops[0], 24.);
    // Le dernier enfant n'a pas de marge basse ; sa marge haute reste la sienne.
    near(tops[2] - tops[1], 28.);
    near(layout::height(column.bounds()), 3. * 16. + 2. * 8. + 4.);
    let row = &page.children[1];
    near(
        row.children[1].bounds().left - row.children[0].bounds().right,
        12.,
    );
    arrange_in(&mut page, environment(900., 600., None), 900., 300.);
    let row = &page.children[1];
    near(
        row.children[1].bounds().left - row.children[0].bounds().right,
        4.,
    );
}

#[test]
fn responsive_grid_placement_and_justify_self_follow_the_viewport() {
    let mut page = ui! {@styles(css);
        <div><div class="grid grid-cols-2 w-[100px]">
            <div class="h-[10px] md:col-start-2 md:row-start-1"></div>
            <div class="h-[10px] w-[10px] md:justify-self-end"></div>
        </div></div>
    };
    let place = |page: &mut Element, width: f32| {
        arrange_in(page, environment(width, 600., None), width, 600.);
        let grid = &page.children[0];
        (grid.children[0].bounds(), grid.children[1].bounds())
    };
    let (a, b) = place(&mut page, 500.);
    near(a.left, 0.);
    near(b.left, 50.);
    let (a, b) = place(&mut page, 900.);
    near(a.left, 50.);
    near(a.top, 0.);
    near(b.left, 40.);
    // La base revient quand la media query ne correspond plus.
    let (a, b) = place(&mut page, 500.);
    near(a.left, 0.);
    near(b.left, 50.);
}

#[test]
fn percentage_and_expression_limits_resolve_against_the_parent() {
    let mut page = ui! {@styles(css);
        <div><div class="flex flex-col w-[400px] h-[300px]">
            <div class="max-w-[40%] h-4"></div>
            <div class="flex w-[200px]"><div class="w-[500px] max-w-full h-4"></div></div>
            <div class="w-[50px] min-w-full h-4"></div>
            <div class="h-[500px] shrink-0 max-h-[calc(50vh-20px)]"></div>
            <div class="max-w-[50px] md:max-w-[50%] h-4"></div>
        </div></div>
    };
    arrange(&mut page, 800., 600.);
    let c = &page.children[0].children;
    near(layout::width(c[0].bounds()), 160.);
    near(layout::width(c[1].children[0].bounds()), 200.);
    near(layout::width(c[2].bounds()), 400.);
    near(layout::height(c[3].bounds()), 280.);
    // Variante : la limite relative remplace la limite en px de la base.
    near(layout::width(c[4].bounds()), 200.);
    arrange(&mut page, 600., 600.);
    near(layout::width(page.children[0].children[4].bounds()), 50.);
}

#[test]
fn percentage_translations_move_by_the_element_size() {
    let mut page = ui! {@styles(css);
        <div><div class="relative w-[200px] h-[100px]">
            <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-10 h-4"></div>
            <div class="-translate-x-full translate-y-2 w-10 h-4"></div>
        </div></div>
    };
    arrange(&mut page, 400., 300.);
    let c = &page.children[0].children;
    let e = c[0].base_effect();
    near(e.x_percent, -50.);
    near(e.y_percent, -50.);
    let e = c[1].base_effect();
    near(e.x_percent, -100.);
    near(e.y, 8.);
}

#[test]
fn auto_fill_and_auto_fit_repeat_the_track_and_col_span_full_spans_every_column() {
    let mut page = ui! {@styles(css);
        <div><div class="flex flex-col w-[330px]">
            <div class="grid grid-cols-[repeat(auto-fill,minmax(100px,1fr))] gap-[10px]">
                <div class="h-4"></div><div class="h-4"></div><div class="h-4"></div>
                <div class="h-4"></div><div class="col-span-full h-4"></div>
            </div>
            <div class="grid grid-cols-[repeat(auto-fit,minmax(100px,1fr))] gap-[10px]">
                <div class="h-4"></div><div class="h-4"></div>
            </div>
        </div></div>
    };
    arrange(&mut page, 400., 300.);
    let fill = &page.children[0].children[0].children;
    near(layout::width(fill[0].bounds()), (330. - 20.) / 3.);
    near(fill[3].bounds().left, 0.);
    near(fill[3].bounds().top, fill[0].bounds().top + 26.);
    near(layout::width(fill[4].bounds()), 330.);
    let fit = &page.children[0].children[1].children;
    near(layout::width(fit[0].bounds()), 160.);
}

#[test]
fn inline_display_shows_the_element_as_a_box() {
    let build = || {
        ui! {@styles(css);
            <div><div class="flex items-baseline gap-2">
                <p class="text-[16px]">Titre</p>
                <span class="hidden md:inline text-sm">Description</span>
                <div class="inline-block h-4 w-4"></div>
            </div></div>
        }
    };
    let mut page = build();
    arrange(&mut page, 600., 300.);
    let row = &page.children[0].children;
    near(layout::width(row[1].bounds()), 0.);
    let mut page = build();
    arrange(&mut page, 900., 300.);
    let row = &page.children[0].children;
    assert!(layout::width(row[1].bounds()) > 50.);
    near(layout::width(row[2].bounds()), 16.);
}

#[test]
fn block_items_of_a_flex_column_take_their_content_width_unless_stretched() {
    let mut page = ui! {@styles(css);
        <div>
            <div class="flex flex-col items-center w-[400px]"><div id="badge" class="px-2"><div class="w-[40px] h-[10px]"></div></div></div>
            <div class="flex flex-wrap w-[1000px]"><div id="title" class="flex flex-col"><div class="w-[104px] h-[10px]"></div></div><div id="tools" class="ml-auto w-[166px] h-[10px]"></div></div>
            <div class="flex w-[400px]"><div class="flex-1 min-w-0 h-[10px]"></div><div id="side" class="shrink-0 flex flex-col items-end"><div id="short" class="w-[24px] h-[10px]"></div><div class="w-[48px] h-[10px]"></div></div></div>
        </div>
    };
    arrange(&mut page, 1000., 300.);
    let badge = find(&page, "badge").bounds();
    near(layout::width(badge), 56.);
    near(badge.left, 172.);
    // Une colonne flexible dans une rangée qui passe à la ligne : sa largeur de contenu,
    // les contrôles restent sur la même ligne.
    near(layout::width(find(&page, "title").bounds()), 104.);
    near(
        find(&page, "tools").bounds().top,
        find(&page, "title").bounds().top,
    );
    near(layout::width(find(&page, "side").bounds()), 48.);
    near(find(&page, "short").bounds().right, 400.);
}

#[test]
fn h_full_in_a_grid_cell_follows_its_row() {
    let mut page = ui! {@styles(css);
        <div><div class="grid grid-rows-2 h-[200px] w-[100px]"><div id="cell" class="h-full"></div><div></div></div></div>
    };
    arrange(&mut page, 400., 300.);
    near(layout::height(find(&page, "cell").bounds()), 100.);
}

#[test]
fn flex_1_in_an_auto_height_column_takes_its_content_up_to_the_max_height() {
    let mut page = ui! {@styles(css);
        <div>
            <div class="flex flex-col max-h-[300px] w-[200px]"><div class="h-[20px] shrink-0"></div><div id="short" class="flex-1 min-h-0 overflow-y-auto"><div class="h-[100px]"></div></div></div>
            <div class="flex flex-col max-h-[300px] w-[200px]"><div class="h-[20px] shrink-0"></div><div id="long" class="flex-1 min-h-0 overflow-y-auto"><div class="h-[500px]"></div></div></div>
        </div>
    };
    arrange(&mut page, 400., 1000.);
    near(layout::height(find(&page, "short").bounds()), 100.);
    near(layout::height(find(&page, "long").bounds()), 280.);
}

#[test]
fn min_h_0_lets_a_column_item_shrink_under_a_max_height_parent() {
    let mut page = ui! {@styles(css);
        <div><div class="flex flex-col max-h-[300px] w-[200px]"><div class="h-[69px] shrink-0"></div><div id="body" class="min-h-0 flex flex-col"><div id="inner" class="h-full max-h-[1000px] overflow-hidden"><div class="h-[800px]"></div></div></div></div></div>
    };
    arrange(&mut page, 400., 1000.);
    near(layout::height(find(&page, "body").bounds()), 231.);
    near(layout::height(find(&page, "inner").bounds()), 231.);
}

#[test]
fn a_scroll_area_reserves_its_bar_only_when_the_content_overflows() {
    let mut page = ui! {@styles(css);
        <div>
            <div id="auto" class="max-h-[300px] overflow-y-auto w-[200px]"><div class="flex flex-wrap"><div class="w-[50px] h-[50px]"></div><div class="w-[50px] h-[50px]"></div><div class="w-[50px] h-[50px]"></div><div class="w-[50px] h-[50px]"></div></div></div>
            <div id="none" class="max-h-[300px] overflow-y-auto w-[200px] [scrollbar-width:none]"><div class="flex flex-wrap"><div class="w-[50px] h-[50px]"></div><div class="w-[50px] h-[50px]"></div><div class="w-[50px] h-[50px]"></div><div class="w-[50px] h-[50px]"></div></div></div>
            <div id="scroll" class="max-h-[300px] overflow-y-scroll w-[200px]"><div class="flex flex-wrap"><div class="w-[50px] h-[50px]"></div><div class="w-[50px] h-[50px]"></div><div class="w-[50px] h-[50px]"></div><div class="w-[50px] h-[50px]"></div></div></div>
        </div>
    };
    arrange(&mut page, 400., 1000.);
    // Les quatre cases tiennent sur une ligne : pas de gouttière, ni à la mesure ni au
    // placement ; `overflow: scroll` la garde.
    near(layout::height(find(&page, "auto").bounds()), 50.);
    near(layout::height(find(&page, "none").bounds()), 50.);
    near(layout::height(find(&page, "scroll").bounds()), 100.);
}

#[test]
fn an_absolute_box_wider_than_the_room_left_overflows() {
    let mut page = ui! {@styles(css);
        <div><div class="relative w-[400px] h-[100px]"><div id="bar" class="absolute left-1/2"><div class="w-[480px] h-[10px]"></div></div><div id="label" class="absolute left-1/2"><div class="w-[50px] h-[10px]"></div></div></div></div>
    };
    arrange(&mut page, 400., 300.);
    near(layout::width(find(&page, "bar").bounds()), 480.);
    near(layout::width(find(&page, "label").bounds()), 50.);
}

#[test]
fn the_content_of_a_tall_button_is_centered_vertically() {
    let mut page = ui! {@styles(css);
        <div>
            <button class="h-[40px] w-[100px]" onclick={Msg::Click}><div id="block" class="h-[10px] w-[10px]"></div></button>
            <button class="flex h-[40px] w-[100px]" onclick={Msg::Click}><div id="flex" class="h-[10px] w-[10px]"></div></button>
        </div>
    };
    arrange(&mut page, 400., 300.);
    near(find(&page, "block").bounds().top, 15.);
    // `display: flex` : les éléments partent du haut, comme dans un navigateur.
    near(find(&page, "flex").bounds().top, 40.);
}
