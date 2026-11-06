use crate::application::Resources;
use crate::ui::{
    element::{PaintContext, Semantics, SizeF, TextMeasurer},
    layout,
    prelude::*,
    runtime::{ChromeState, Key, UiEvent, UiRuntime},
    scene::{DrawCommand, Image, Scene},
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Change(String),
    Toggle(bool),
    Pick(u64),
    Volume(f64),
    Save,
}

struct Measure;
impl TextMeasurer for Measure {
    fn measure_text(&mut self, value: &str, size: f32, _: bool, max: f32) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: (value.chars().count() as f32 * size * 0.5).min(max),
            height: size * 1.4,
        })
    }
}
struct NoResources;
impl Resources for NoResources {
    fn decode_image(&self, _: &[u8]) -> crate::Result<Arc<Image>> {
        Err(crate::Error::Unsupported {
            reason: "no image decoder in this test",
        })
    }
}
fn prepare(ui: &mut UiRuntime) {
    ui.prepare(
        ViewContext {
            width: 600.,
            height: 400.,
            theme: Theme::DARK,
            transparent: false,
            backend: "test",
            scale: 1.,
        },
        ChromeState::default(),
        &mut Measure,
    )
    .unwrap();
}

#[test]
fn named_and_scoped_keys_are_stable_and_duplicate_keys_are_rejected() {
    const PANEL: ElementId = ElementId::named("panel");
    const ITEM: ElementId = PANEL.item(123);
    assert_eq!(ElementId::from("panel"), PANEL);
    assert_eq!(
        PANEL.child("field"),
        ElementId::named("panel").child("field")
    );
    assert_ne!(
        PANEL.child("field"),
        ElementId::named("other").child("field")
    );
    assert_eq!(ITEM, PANEL.item(123));
    assert_ne!(ITEM, PANEL.child("123"));
    assert!(
        layout::validate(&column(
            "root",
            [text(PANEL.child("title"), "A"), text("other", "B")]
        ))
        .is_ok()
    );
    assert!(crate::error::rejected(layout::validate(&row(
        "root",
        [text("same", "A"), text("same", "B")]
    ))));
}
#[test]
fn recipes_compose_without_resetting_unspecified_properties_and_use_css_padding_order() {
    const BASE: ViewStyle = ViewStyle::new()
        .width(240.)
        .padding(16.)
        .gap(12.)
        .background(Color::hex(0x123456));
    const COMPACT: ViewStyle = ViewStyle::new().gap(4.).radius(8.);
    let mut tree = column("root", [text("title", "Bonjour")])
        .style(BASE)
        .style(COMPACT)
        .padding([24, 12])
        .height(300);
    assert!(matches!(tree.style.width, Length::Px(240.)));
    assert_eq!(tree.style.gap, 4.);
    assert_eq!(tree.style.padding.top, 24.);
    assert_eq!(tree.style.padding.left, 12.);
    assert_eq!(tree.style.background, Some(Color::rgb(18, 52, 86)));
    tree = tree.padding([1, 2, 3, 4]);
    assert_eq!(
        (
            tree.style.padding.top,
            tree.style.padding.right,
            tree.style.padding.bottom,
            tree.style.padding.left
        ),
        (1., 2., 3., 4.)
    );
    layout::arrange(
        &mut tree,
        crate::ui::scene::RectF::new(0., 0., 240., 300.),
        crate::ui::scene::RectF::new(0., 0., 240., 300.),
        &mut Measure,
    )
    .unwrap();
    assert_eq!(tree.find("title".into()).unwrap().bounds().left, 4.);
    assert!(crate::error::rejected(layout::validate(
        &column("bad", []).width(-1)
    )));
}
#[test]
fn lazy_children_and_custom_widgets_use_the_same_contract_and_layout() {
    struct Custom;
    impl Widget for Custom {
        fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
            Ok(SizeF {
                width: 50.,
                height: 20.,
            })
        }
        fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
            scene.fill(ctx.rect, 0., Color::hex(0x123456));
        }
        fn semantics(&self) -> Semantics<'_> {
            Semantics {
                role: crate::ui::element::Role::Custom,
                name: "personnalisé",
                enabled: true,
            }
        }
    }
    let root = column("root", [])
        .child_if(false, || panic!("enfant caché construit"))
        .child_if(true, || Custom.element("custom"));
    assert_eq!(root.children.len(), 1);
    assert_eq!(
        root.children[0].widget.as_ref().unwrap().semantics().name,
        "personnalisé"
    );
    assert!(layout::validate(&root).is_ok());
}
#[test]
fn typed_messages_work_with_every_control_constructor() {
    let root = column(
        "root",
        [
            button("save", "Enregistrer", Msg::Save),
            Checkbox::new("Actif", false, Msg::Toggle).element("check"),
            input("query", "", Msg::Change),
            Select::new(Some(1), [SelectOption::new(1, "A")], Msg::Pick).element("select"),
            Slider::new(0.5, Msg::Volume)
                .on_commit(Msg::Volume)
                .element("slider"),
        ],
    );
    assert!(layout::validate(&root).is_ok());
    let emitted = |index: usize| {
        root.children[index]
            .widget
            .as_ref()
            .unwrap()
            .action()
            .unwrap()
            .emit()
            .downcast::<Msg>()
    };
    assert_eq!(emitted(0), Ok(Msg::Save));
    assert_eq!(emitted(1), Ok(Msg::Toggle(true)));
    let input = root.children[2].widget.as_ref().unwrap().text_input();
    assert_eq!(
        input
            .unwrap()
            .on_change
            .emit("été".into())
            .downcast::<Msg>(),
        Ok(Msg::Change("été".into()))
    );
    let select = root.children[3].widget.as_ref().unwrap().select().unwrap();
    assert_eq!(select.on_change.emit(1).downcast::<Msg>(), Ok(Msg::Pick(1)));
    let slider = root.children[4].widget.as_ref().unwrap().slider().unwrap();
    assert_eq!(
        slider
            .on_commit
            .as_ref()
            .unwrap()
            .emit(0.25)
            .downcast::<Msg>(),
        Ok(Msg::Volume(0.25))
    );
    // Un type de message étranger reste détectable : il n'est jamais converti en silence.
    assert_eq!(
        Action::new(7_u8).emit().downcast::<Msg>(),
        Err(std::any::type_name::<u8>())
    );
}
#[test]
fn keyed_input_preserves_edited_value_and_focus_when_the_view_reorders_children() {
    struct App {
        value: Rc<RefCell<String>>,
        reverse: bool,
    }
    impl Application for App {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            let mut children = [
                input("query", self.value.borrow().as_str(), Msg::Change),
                button("save", "Enregistrer", Msg::Save),
            ];
            if self.reverse {
                children.reverse();
            }
            column("root", children).gap(8.)
        }
        fn update(&mut self, message: Msg) -> bool {
            let Msg::Change(value) = message else {
                return false;
            };
            *self.value.borrow_mut() = value;
            self.reverse = true;
            true
        }
    }
    let value = Rc::new(RefCell::new(String::new()));
    let mut ui = UiRuntime::new(
        App {
            value: Rc::clone(&value),
            reverse: false,
        },
        &NoResources,
    )
    .unwrap();
    prepare(&mut ui);
    ui.event(UiEvent::KeyDown {
        key: Key::Tab,
        shift: false,
        repeat: false,
    });
    assert_eq!(ui.focused(), Some("query".into()));
    ui.event(UiEvent::Text("été 🎵".into()));
    prepare(&mut ui);
    assert_eq!(ui.focused(), Some("query".into()));
    assert_eq!(&*value.borrow(), "été 🎵");
    assert_eq!(ui.root().unwrap().children[0].id, "save".into());
    assert_eq!(&*ui.focused_input().unwrap().value, "été 🎵");
}
#[test]
fn custom_text_font_is_measured_and_emitted_with_wrapping_and_builder_precedence() {
    struct FontMeasure(bool);
    impl TextMeasurer for FontMeasure {
        fn measure_text(&mut self, _: &str, _: f32, _: bool, _: f32) -> crate::Result<SizeF> {
            panic!("mesure de police perdue")
        }
        fn measure_font(
            &mut self,
            _: &str,
            size: f32,
            font: Font,
            max: f32,
        ) -> crate::Result<SizeF> {
            self.0 = true;
            assert_eq!(font, Font::new("Inter", 700));
            assert_eq!(size, 22.);
            Ok(SizeF {
                width: max,
                height: 60.,
            })
        }
    }
    let title = Text::new("Une longue phrase avec des accents é è")
        .font("Inter")
        .weight(600)
        .bold()
        .size(22.);
    let mut measure = FontMeasure(false);
    let size = title
        .measure(
            SizeF {
                width: 80.,
                height: 100.,
            },
            &mut measure,
        )
        .unwrap();
    assert!(measure.0);
    let mut scene = Scene::new(Color::hex(0));
    title.paint(
        &PaintContext {
            rect: crate::ui::scene::RectF::new(0., 0., size.width, size.height),
            intrinsic: size,
            theme: Theme::DARK,
            state: Default::default(),
        },
        &mut scene,
    );
    assert!(matches!(
        scene.commands[0],
        DrawCommand::FontText {
            wrap: true,
            font: Font { weight: 700, .. },
            ..
        }
    ));
    assert!(Text::new("incorrect").weight(0).validate().is_err());
}
