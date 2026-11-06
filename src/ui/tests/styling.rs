//! Styles partagés : couleurs de boutons selon le thème et l'état, surfaces, styles de
//! texte et validation des valeurs.
use crate::ui::tests::support::*;
use crate::ui::{
    style::*,
    widgets::{ButtonVariant, Text},
};

fn paint(button: &Button, theme: Theme, state: VisualState) -> Scene {
    let mut measure = Measure::default();
    let intrinsic = button
        .measure(
            SizeF {
                width: 200.,
                height: 100.,
            },
            &mut measure,
        )
        .unwrap();
    let mut scene = Scene::new(theme.background);
    button.paint(
        &PaintContext {
            rect: RectF::new(0., 0., 200., 100.),
            intrinsic,
            theme,
            state,
        },
        &mut scene,
    );
    scene.validate().unwrap();
    scene
}
fn fill(scene: &Scene) -> crate::ui::theme::Color {
    scene
        .commands
        .iter()
        .find_map(|c| match c {
            crate::ui::scene::DrawCommand::Fill { color, .. } => Some(*color),
            _ => None,
        })
        .unwrap()
}
fn text_color(scene: &Scene) -> crate::ui::theme::Color {
    scene
        .commands
        .iter()
        .find_map(|c| match c {
            crate::ui::scene::DrawCommand::Text { color, .. } => Some(*color),
            _ => None,
        })
        .unwrap()
}

#[test]
fn shared_button_colors_follow_the_theme_at_paint_time_and_fixed_colors_stay_fixed() {
    let fixed = crate::ui::theme::Color::rgb(22, 44, 66);
    let recipe = Arc::new(ButtonStyle {
        normal: Some(ButtonColors::new(ThemeColor::Accent, ThemeColor::OnAccent)),
        hovered: Some(ButtonColors::new(fixed, ThemeColor::Text)),
        ..ButtonStyle::default()
    });
    let a = Button::new("Lire", Msg::Click(1)).style(recipe.clone());
    let b = Button::new("Autre", Msg::Click(2)).style(recipe.clone());
    assert!(Arc::ptr_eq(
        a.style.as_ref().unwrap(),
        b.style.as_ref().unwrap()
    ));
    for theme in [
        Theme::DARK,
        Theme::LIGHT,
        Theme {
            accent: fixed,
            on_accent: crate::ui::theme::Color::rgb(1, 2, 3),
            ..Theme::DARK
        },
    ] {
        let scene = paint(&a, theme, VisualState::default());
        assert_eq!(fill(&scene), theme.accent);
        assert_eq!(text_color(&scene), theme.on_accent);
        let scene = paint(
            &b,
            theme,
            VisualState {
                hovered: true,
                ..VisualState::default()
            },
        );
        assert_eq!(fill(&scene), fixed);
        assert_eq!(text_color(&scene), theme.text);
    }
}

#[test]
fn button_state_priority_respects_disabled_pressed_hovered_and_custom_focus() {
    use crate::ui::theme::Color;
    let recipe = ButtonStyle {
        normal: Some(ButtonColors::new(Color::rgb(10, 0, 0), Color::rgb(1, 1, 1))),
        hovered: Some(ButtonColors::new(Color::rgb(20, 0, 0), Color::rgb(2, 2, 2))),
        pressed: Some(ButtonColors::new(Color::rgb(30, 0, 0), Color::rgb(3, 3, 3))),
        disabled: Some(ButtonColors::new(Color::rgb(40, 0, 0), Color::rgb(4, 4, 4))),
        focus: Some(Color::rgb(8, 9, 10).into()),
        ..ButtonStyle::default()
    };
    let button = Button::new("Action", Msg::Click(1)).style(recipe);
    for (hovered, pressed, red) in [(false, false, 10), (true, false, 20), (true, true, 30)] {
        let scene = paint(
            &button,
            Theme::DARK,
            VisualState {
                hovered,
                pressed,
                focused: true,
            },
        );
        assert_eq!(fill(&scene).red, red);
        assert!(scene.commands.iter().any(|c| matches!(c, crate::ui::scene::DrawCommand::Stroke {color,..} if *color == Color::rgb(8,9,10))));
    }
    let disabled = button.disabled();
    let scene = paint(
        &disabled,
        Theme::DARK,
        VisualState {
            hovered: true,
            pressed: true,
            focused: true,
        },
    );
    assert_eq!(fill(&scene).red, 40);
    assert_eq!(text_color(&scene), Color::rgb(4, 4, 4));
    assert!(
        !scene
            .commands
            .iter()
            .any(|c| matches!(c, crate::ui::scene::DrawCommand::Stroke { .. }))
    );
    assert!(disabled.action().is_none());
}

#[test]
fn styled_buttons_center_native_label_metrics_even_with_a_large_minimum_height() {
    struct TallFont;
    impl TextMeasurer for TallFont {
        fn measure_text(&mut self, _: &str, size: f32, bold: bool, _: f32) -> crate::Result<SizeF> {
            assert_eq!(size, 20.);
            assert!(!bold);
            Ok(SizeF {
                width: 64.,
                height: 47.,
            })
        }
    }
    let b = Button::new("Titre", Msg::Click(1)).style(ButtonStyle {
        font_size: 20.,
        bold: false,
        padding_x: 12.,
        padding_y: 6.,
        min_height: 80.,
        ..ButtonStyle::default()
    });
    let intrinsic = b.measure(SizeF::default(), &mut TallFont).unwrap();
    near(intrinsic.width, 88.);
    near(intrinsic.height, 80.);
    let mut scene = Scene::new(Theme::DARK.background);
    b.paint(
        &PaintContext {
            rect: RectF::new(10., 20., 160., 100.),
            intrinsic,
            theme: Theme::DARK,
            state: VisualState::default(),
        },
        &mut scene,
    );
    let rect = scene
        .commands
        .iter()
        .find_map(|c| match c {
            crate::ui::scene::DrawCommand::Text { rect, .. } => Some(*rect),
            _ => None,
        })
        .unwrap();
    near(rect.left, 22.);
    near(rect.top, 46.5);
}

#[test]
fn ghost_and_outline_keep_transparent_backgrounds_and_theme_hover() {
    for variant in [ButtonVariant::Ghost, ButtonVariant::Outline] {
        let button = Button::new("Action", Msg::Click(1)).variant(variant);
        let normal = paint(&button, Theme::LIGHT, VisualState::default());
        assert_eq!(fill(&normal).alpha, 0);
        assert_eq!(text_color(&normal), Theme::LIGHT.text);
        let strokes = normal
            .commands
            .iter()
            .filter(|c| matches!(c, crate::ui::scene::DrawCommand::Stroke { .. }))
            .count();
        assert_eq!(
            strokes,
            usize::from(matches!(variant, ButtonVariant::Outline))
        );
        let hover = paint(
            &button,
            Theme::LIGHT,
            VisualState {
                hovered: true,
                ..VisualState::default()
            },
        );
        assert_eq!(fill(&hover), Theme::LIGHT.hover);
    }
}

#[test]
fn surface_recipes_apply_layout_and_local_overrides_follow_builder_order() {
    use crate::ui::theme::Color;
    let style = Arc::new(SurfaceStyle {
        padding: Insets::all(20.),
        gap: 7.,
        radius: 12.,
        ..SurfaceStyle::default()
    });
    let a = Element::column(1, vec![])
        .width(Length::Px(200.))
        .styled(style.clone())
        .padding(3.)
        .radius(4.)
        .surface(Color::rgb(1, 2, 3), 6.);
    near(a.style.padding.left, 3.);
    near(a.style.radius, 6.);
    near(a.style.gap, 7.);
    assert_eq!(a.style.background, Some(Color::rgb(1, 2, 3)));
    let b = a.styled(style.clone());
    near(b.style.padding.top, 20.);
    near(b.style.radius, 12.);
    assert!(b.style.background.is_none());
    assert!(matches!(b.style.width, Length::Px(200.)));
    assert!(Arc::ptr_eq(b.surface_style.as_ref().unwrap(), &style));
}

#[test]
fn shared_surfaces_follow_theme_preserve_alpha_and_release_all_references() {
    let recipe = Arc::new(SurfaceStyle {
        background: Some(ThemeColor::Surface.with_alpha(180)),
        ..SurfaceStyle::default()
    });
    let captured = recipe.clone();
    let mut ui = view_runtime(move || {
        Element::column(
            1,
            vec![Element::column(2, vec![widgets::text(3, "Contenu")]).styled(captured.clone())],
        )
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    let counts = Arc::strong_count(&recipe);
    let stats = ui.stats();
    let p = center(&ui, 3);
    ui.event(UiEvent::PointerMove(p));
    prepare(&mut ui, &mut measure);
    assert_eq!(Arc::strong_count(&recipe), counts);
    assert_eq!(ui.stats().layouts, stats.layouts);
    for theme in [Theme::LIGHT, Theme::DARK] {
        let mut ctx = context();
        ctx.theme = theme;
        ui.prepare(ctx, ChromeState::default(), &mut measure)
            .unwrap();
        assert!(ui.scene().commands.iter().any(|c| matches!(c, crate::ui::scene::DrawCommand::Fill {color,..} if *color == crate::ui::theme::Color {alpha:180, ..theme.surface})));
    }
    drop(ui);
    assert_eq!(Arc::strong_count(&recipe), 1);
}

#[test]
fn reusable_text_style_can_be_overridden_locally_without_changing_other_text() {
    use crate::ui::theme::Color;
    let shared = Arc::new(TextStyle {
        font_size: 24.,
        bold: true,
        color: Some(ThemeColor::Accent.into()),
    });
    let first = Text::new("Titre")
        .style(shared.clone())
        .size(18.)
        .color(Color::rgb(8, 9, 10));
    let second = Text::new("Autre").style(shared.clone());
    assert_eq!(first.size, 18.);
    assert_eq!(second.size, 24.);
    let mut scene = Scene::new(Theme::LIGHT.background);
    let ctx = PaintContext {
        rect: RectF::new(0., 0., 300., 100.),
        intrinsic: SizeF::default(),
        theme: Theme::LIGHT,
        state: VisualState::default(),
    };
    first.paint(&ctx, &mut scene);
    assert_eq!(text_color(&scene), Color::rgb(8, 9, 10));
    scene.commands.clear();
    second.paint(&ctx, &mut scene);
    assert_eq!(text_color(&scene), Theme::LIGHT.accent);
}

#[test]
fn style_changes_preserve_button_identity_actions_and_keyboard_focus() {
    struct App {
        clicks: Rc<Cell<usize>>,
        recipe: Arc<ButtonStyle>,
    }
    impl Application for App {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            Element::column(
                1,
                vec![Element::custom(
                    2,
                    Button::new(format!("{} clics", self.clicks.get()), Msg::Click(1))
                        .style(self.recipe.clone()),
                )],
            )
        }
        fn update(&mut self, message: Msg) -> bool {
            assert_eq!(message, Msg::Click(1));
            self.clicks.set(self.clicks.get() + 1);
            true
        }
    }
    let clicks = Rc::new(Cell::new(0));
    let mut ui = UiRuntime::new(
        App {
            clicks: clicks.clone(),
            recipe: Arc::new(ButtonStyle {
                radius: 18.,
                ..ButtonStyle::default()
            }),
        },
        &NoResources,
    )
    .unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    ui.accessibility_action(AccessibilityAction::Focus(ElementId(2)))
        .unwrap();
    prepare(&mut ui, &mut measure);
    ui.event(UiEvent::KeyDown {
        key: Key::Enter,
        shift: false,
        repeat: false,
    });
    prepare(&mut ui, &mut measure);
    assert_eq!(clicks.get(), 1);
    assert_eq!(ui.focused(), Some(ElementId(2)));
    let mut ctx = context();
    ctx.theme = Theme::LIGHT;
    ui.prepare(ctx, ChromeState::default(), &mut measure)
        .unwrap();
    assert_eq!(ui.focused(), Some(ElementId(2)));
    assert!(ui.accessibility()[0].invokable);
}

#[test]
fn style_validation_and_widget_validation_prevent_invalid_values_before_measurement() {
    for style in [
        ButtonStyle {
            font_size: f32::NAN,
            ..ButtonStyle::default()
        },
        ButtonStyle {
            padding_x: -1.,
            ..ButtonStyle::default()
        },
        ButtonStyle {
            border_width: f32::INFINITY,
            ..ButtonStyle::default()
        },
    ] {
        assert!(crate::error::rejected(layout::validate(&Element::custom(
            1,
            Button::new("Test", Msg::Click(1)).style(style)
        ))));
    }
    assert!(crate::error::rejected(layout::validate(
        &Element::column(1, vec![]).styled(SurfaceStyle {
            gap: f32::NAN,
            ..SurfaceStyle::default()
        })
    )));
    assert!(crate::error::rejected(layout::validate(&Element::custom(
        1,
        Text::new("T").style(TextStyle {
            font_size: 0.,
            ..TextStyle::default()
        })
    ))));
    struct Invalid;
    impl Widget for Invalid {
        fn validate(&self) -> crate::Result<()> {
            Err(crate::Error::invalid_input("invalid custom widget"))
        }
        fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
            panic!("Ne doit pas être mesuré")
        }
        fn paint(&self, _: &PaintContext, _: &mut Scene) {}
    }
    assert!(crate::error::rejected(layout::validate(&Element::custom(
        1, Invalid
    ))));
}

#[test]
fn transparent_layer_recipe_disables_default_fill_and_keeps_borders_inside_small_rects() {
    use crate::ui::{modal::Modal, scene::DrawCommand};
    let mut ui = view_runtime(|| {
        Element::layers(
            1,
            Element::column(2, vec![]),
            vec![
                Element::modal(3, Modal::new("Dialogue"), widgets::text(4, "Texte")).styled(
                    SurfaceStyle {
                        background: None,
                        border: None,
                        ..SurfaceStyle::default()
                    },
                ),
            ],
        )
    });
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    assert!(
        !ui.scene()
            .commands
            .iter()
            .any(|c| matches!(c,DrawCommand::Fill {color,..} if *color == Theme::DARK.surface))
    );
    let mut scene = Scene::new(Theme::DARK.background);
    crate::ui::style::border(
        &mut scene,
        RectF::new(0., 0., 16., 12.),
        18.,
        50.,
        Theme::DARK.border,
    );
    let DrawCommand::Stroke { rect, width, .. } = &scene.commands[0] else {
        panic!()
    };
    assert!(rect.left - width / 2. >= 0. && rect.top - width / 2. >= 0.);
    assert!(rect.right + width / 2. <= 16. && rect.bottom + width / 2. <= 12.);
    scene.validate().unwrap();
}
