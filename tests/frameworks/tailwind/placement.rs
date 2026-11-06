//! Mise en page : espacements, tailles, flex, grilles (placement responsive, `auto-fill`,
//! `col-span-full`), `space-*`, limites en pourcentage et en expression, translations en
//! pourcentage, `inline` / `inline-block`.
use super::{arrange, arrange_in, css, environment, near};
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
