//! Flex : retour à la ligne, croissance gelée par les limites, rétrécissement, base nulle
//! remesurée après distribution, facteurs fractionnaires.
use crate::support::*;
use karbon_ui::ui::{
    element::{SizeF, TextMeasurer},
    layout,
    prelude::*,
    scene::RectF,
    styling::{self, StyleEnvironment},
};
css_file!(mod css, "flex.css");

#[test]
fn flex_wrap_grow_freeze_and_shrink_have_deterministic_geometry() {
    let mut root = ui! {@styles(css);<div class="flex"><div class="freeze"></div><div class="basis"></div><div class="basis"></div></div>};
    arrange(&mut root, 300., 100.);
    near(layout::width(root.children[0].bounds()), 120.);
    near(layout::width(root.children[1].bounds()), 170.);
    near(root.children[1].bounds().left, 130.);
    near(root.children[2].bounds().top, 30.);
    near(layout::width(root.children[2].bounds()), 300.);
    let mut root = ui! {@styles(css);<div class="shrink"><div class="shrink-item"></div><div class="shrink-item"></div></div>};
    arrange(&mut root, 150., 100.);
    near(layout::width(root.children[0].bounds()), 75.);
    near(root.children[1].bounds().left, 75.);
}

#[test]
fn flex_zero_basis_remeasures_wrapped_text_after_distributing_width() {
    struct Wrapped;
    impl TextMeasurer for Wrapped {
        fn measure_text(
            &mut self,
            text: &str,
            size: f32,
            _: bool,
            max: f32,
        ) -> karbon_ui::Result<SizeF> {
            let width = text.chars().count() as f32 * size * 0.5;
            Ok(SizeF {
                width: width.min(max),
                height: (width / max.max(1.)).ceil().max(1.) * size,
            })
        }
    }
    let mut root = ui! {@styles(css);<div>
        <div class="sizing-row"><div class="sizing-copy"><span>Un titre et sa description</span></div><div class="sizing-action"></div></div>
        <div style="height:20px"></div>
    </div>};
    styling::resolve(&mut root, StyleEnvironment::default()).unwrap();
    let r = RectF::new(0., 0., 600., 200.);
    layout::arrange(&mut root, r, r, &mut Wrapped).unwrap();
    near(layout::height(root.children[0].bounds()), 30.);
    near(root.children[1].bounds().top, 30.);
    near(layout::width(root.children[0].children[0].bounds()), 500.);
}

#[test]
fn fractional_flex_factors_leave_unused_space_and_freeze_against_original_bases() {
    let mut grow = ui! {@styles(css);<div class="shrink"><div class="partial-grow"></div><div class="partial-grow"></div></div>};
    arrange(&mut grow, 400., 100.);
    near(layout::width(grow.children[0].bounds()), 150.);
    near(layout::width(grow.children[1].bounds()), 150.);
    let mut limited = ui! {@styles(css);<div class="shrink"><div class="partial-grow partial-limit"></div><div class="partial-grow"></div></div>};
    arrange(&mut limited, 400., 100.);
    near(layout::width(limited.children[0].bounds()), 110.);
    near(layout::width(limited.children[1].bounds()), 150.);
    let mut shrink = ui! {@styles(css);<div class="shrink"><div class="partial-shrink"></div><div class="partial-shrink"></div></div>};
    arrange(&mut shrink, 200., 100.);
    near(layout::width(shrink.children[0].bounds()), 150.);
    near(layout::width(shrink.children[1].bounds()), 150.);
}
