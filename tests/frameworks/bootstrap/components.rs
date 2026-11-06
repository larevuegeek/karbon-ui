//! Composants : boutons et alertes par variables, thème sombre, grille, collapse et
//! onglets, modale, groupe de boutons ; rapport de compatibilité de la feuille.
use super::bs;
use crate::support::*;
use karbon_ui::ui::{bootstrap, layout, prelude::*};

#[test]
fn bootstrap_buttons_resolve_their_variables() {
    let theme = "light";
    let mut root = ui! {@styles(bs);
        <body data-bs-theme={theme}>
            <div class="d-flex gap-2">
                <button id="primary" class="btn btn-primary" onclick={Msg::Noop}>Primary</button>
                <button id="outline" class="btn btn-outline-danger btn-lg" onclick={Msg::Noop}>Danger</button>
            </div>
            <div id="alert" class="alert alert-success">Alert</div>
        </body>
    };
    arrange(&mut root, 1000., 600.);
    let primary = computed(find(&root, "primary"));
    assert_eq!(primary.local.background, Some(Color::hex(0x0d6efd)));
    assert_eq!(primary.text.color, Some(Color::hex(0xffffff)));
    // padding: var(--bs-btn-padding-y) var(--bs-btn-padding-x) → .375rem .75rem.
    let padding = primary.local.padding.unwrap();
    assert_eq!((padding.top, padding.left), (6., 12.));
    let outline = computed(find(&root, "outline"));
    assert_eq!(outline.local.border, Some(Color::hex(0xdc3545)));
    assert_eq!(outline.local.padding.unwrap().left, 16.);
    // Couleur de texte de l'alerte : variable du thème (`--bs-success-text-emphasis`).
    let alert = computed(find(&root, "alert"));
    assert_eq!(alert.text.color, Some(Color::hex(0x0a3622)));
}

#[test]
fn bootstrap_dark_theme_attribute_overrides_root_variables() {
    let theme = "dark";
    let mut root = ui! {@styles(bs);
        <body id="body" data-bs-theme={theme}>
            <div id="card" class="card"><div class="card-body">Card</div></div>
        </body>
    };
    arrange(&mut root, 800., 600.);
    let body = computed(find(&root, "body"));
    assert_eq!(body.local.background, Some(Color::hex(0x212529)));
    assert_eq!(body.text.color, Some(Color::hex(0xdee2e6)));
}

#[test]
fn bootstrap_grid_columns_share_the_row() {
    let theme = "light";
    let mut root = ui! {@styles(bs);
        <body data-bs-theme={theme}>
            <div class="container">
                <div class="row">
                    <div id="a" class="col-4">A</div>
                    <div id="b" class="col-8">B</div>
                </div>
            </div>
        </body>
    };
    arrange(&mut root, 1000., 600.);
    let (a, b) = (find(&root, "a").bounds(), find(&root, "b").bounds());
    assert!((a.top - b.top).abs() < 0.5, "{a:?} {b:?}");
    assert!(
        (layout::width(b) - 2. * layout::width(a)).abs() < 1.,
        "{a:?} {b:?}"
    );
}

#[test]
fn collapse_and_tabs_follow_bootstrap_state() {
    bootstrap::reset();
    let build = || {
        ui! {@styles(bs);
            <body>
                <button class="btn" data-bs-toggle="collapse" data-bs-target="#c">Toggle</button>
                <div class="collapse" id="c"><div class="card card-body">Content</div></div>
                <ul class="nav nav-tabs">
                    <li class="nav-item"><button id="t1" class="nav-link active" data-bs-toggle="tab" data-bs-target="#p1">One</button></li>
                    <li class="nav-item"><button id="t2" class="nav-link" data-bs-toggle="tab" data-bs-target="#p2">Two</button></li>
                </ul>
                <div class="tab-content">
                    <div class="tab-pane fade show active" id="p1">One</div>
                    <div class="tab-pane fade" id="p2">Two</div>
                </div>
            </body>
        }
    };
    let hidden = |root: &Element, id: &str| layout::width(find(root, id).bounds()) == 0.;
    let mut root = build();
    arrange(&mut root, 800., 600.);
    assert!(hidden(&root, "c") && !hidden(&root, "p1") && hidden(&root, "p2"));
    bootstrap::apply(&bootstrap::BootstrapAction::Toggle {
        target: "c",
        default: false,
        group: &[],
    });
    bootstrap::apply(&bootstrap::BootstrapAction::Show {
        target: "p2",
        group: &["p1", "p2"],
    });
    let mut root = build();
    arrange(&mut root, 800., 600.);
    assert!(!hidden(&root, "c") && hidden(&root, "p1") && !hidden(&root, "p2"));
    bootstrap::reset();
}

#[test]
fn compatibility_report_is_exported() {
    let c = bs::COMPATIBILITY;
    assert!(c.rules > 1500 && c.selectors_percent() > 90.);
    assert!(bs::REPORT.contains("declarations"));
}

#[test]
fn bootstrap_modal_sits_at_the_top_unless_centered() {
    fn top() -> Element {
        ui! {@styles(bs);
            <body data-bs-theme="light">
                <button class="btn" data-bs-toggle="modal" data-bs-target="#m">Open</button>
                <div class="modal show" id="m">
                    <div class="modal-dialog"><div id="content" class="modal-content">
                        <div class="modal-body">Body</div>
                    </div></div>
                </div>
            </body>
        }
    }
    fn centered() -> Element {
        ui! {@styles(bs);
            <body data-bs-theme="light">
                <button class="btn" data-bs-toggle="modal" data-bs-target="#c">Open</button>
                <div class="modal show" id="c">
                    <div class="modal-dialog modal-dialog-centered"><div id="content" class="modal-content">
                        <div class="modal-body">Body</div>
                    </div></div>
                </div>
            </body>
        }
    }
    bootstrap::reset();
    let ui = runtime(top);
    let content = ui
        .root()
        .unwrap()
        .find(ElementId::named("content"))
        .unwrap()
        .bounds();
    // `margin: 1.75rem auto` : 28 px du haut, centrée horizontalement (500 px).
    near(content.top, 28.);
    near(layout::width(content), 500.);
    near(content.left, 150.);
    let ui = runtime(centered);
    let content = ui
        .root()
        .unwrap()
        .find(ElementId::named("content"))
        .unwrap()
        .bounds();
    assert!(
        (content.top + content.bottom - 600.).abs() < 1.,
        "{content:?}"
    );
    bootstrap::reset();
}

#[test]
fn button_group_squares_its_inner_corners() {
    let mut root = ui! {@styles(bs);
        <body data-bs-theme="light">
            <div class="btn-group">
                <button id="l" class="btn btn-outline-primary" onclick={Msg::Noop}>Left</button>
                <button id="m" class="btn btn-outline-primary" onclick={Msg::Noop}>Middle</button>
                <button id="r" class="btn btn-outline-primary" onclick={Msg::Noop}>Right</button>
            </div>
        </body>
    };
    arrange(&mut root, 800., 600.);
    let corners = |id| computed(find(&root, id)).local.corner_radii;
    assert_eq!(corners("l"), Some([6., 0., 0., 6.]));
    assert_eq!(corners("m"), Some([0.; 4]));
    assert_eq!(corners("r"), Some([0., 6., 6., 0.]));
    // Chevauchement des bordures : `margin-left: calc(var(--bs-border-width) * -1)`.
    let (l, m) = (find(&root, "l").bounds(), find(&root, "m").bounds());
    near(m.left, l.right - 1.);
}
