//! Transitions et animations : spinners, hauteur des volets de l'accordéon, carte
//! repliable qui pousse la page, couleurs et chevron interpolés, onglets (seuls ceux qui
//! changent s'animent).
use super::{Demo, demo, panes, scroll_to, settled_panes, with_class};
use karbon_ui::{platform::headless::HeadlessApp, ui::element::Element};

#[test]
fn spinners_animate() {
    // `animation: var(--bs-spinner-animation-speed) linear infinite var(--bs-spinner-animation-name)`
    // résolue par élément ; `spinner-grow` reprend l'opacité de l'élément (0) à ses
    // étapes extrêmes, comme un navigateur.
    let mut app = demo();
    let spots = ["spinner-border", "spinner-grow"].map(|class| {
        let p = scroll_to(&mut app, class, 0);
        (p.x as u32, p.y as u32)
    });
    let crop = |img: &karbon_ui::ui::scene::Image, (cx, cy): (u32, u32)| {
        let mut v = Vec::new();
        for y in cy - 16..cy + 16 {
            for x in cx - 16..cx + 16 {
                let i = ((y * img.width + x) * 4) as usize;
                v.extend_from_slice(&img.pixels[i..i + 4]);
            }
        }
        v
    };
    let before = app.render(1.).unwrap();
    app.advance(std::time::Duration::from_millis(200)).unwrap();
    let after = app.render(1.).unwrap();
    for (class, spot) in ["spinner-border", "spinner-grow"].into_iter().zip(spots) {
        assert_ne!(crop(&before, spot), crop(&after, spot), "{class} is still");
    }
}

#[test]
fn accordion_panes_animate_their_height() {
    // `.collapsing { height: 0; overflow: hidden; transition: height .35s ease }` : un
    // volet s'ouvre pendant que l'autre (même `data-bs-parent`) se referme.
    let open = settled_panes();
    let mut app = demo();
    let before = panes(&app);
    assert!(before[0] > 0. && before[1] == 0., "{before:?}");
    let second = scroll_to(&mut app, "accordion-button", 1);
    app.click_at(second).unwrap();
    app.advance(std::time::Duration::from_millis(120)).unwrap();
    let mid = panes(&app);
    assert!(
        0. < mid[0] && mid[0] < before[0],
        "the closing pane shrinks: {mid:?}"
    );
    assert!(
        0. < mid[1] && mid[1] < open[1],
        "the opening pane grows: {mid:?}"
    );
    // Le contenu garde sa hauteur naturelle, rogné à la boîte animée.
    let root = app.root().unwrap();
    let pane = with_class(root, "accordion-collapse")[1];
    let body = with_class(pane, "accordion-body")[0];
    assert_eq!(body.bounds().bottom - body.bounds().top, open[1]);
    assert!(pane.bounds().bottom < body.bounds().bottom);
    // Le volet reste collé à son en-tête.
    let header = with_class(root, "accordion-header")[1];
    assert_eq!(pane.bounds().top, header.bounds().bottom);
    for _ in 0..30 {
        app.advance(std::time::Duration::from_millis(16)).unwrap();
    }
    assert_eq!(panes(&app), open, "settles at the natural height");
    // Refermer : de la hauteur naturelle vers 0, puis `display: none`.
    let second = scroll_to(&mut app, "accordion-button", 1);
    app.click_at(second).unwrap();
    app.advance(std::time::Duration::from_millis(175)).unwrap();
    let closing = panes(&app);
    assert!(0. < closing[1] && closing[1] < open[1], "{closing:?}");
    assert_eq!(closing[0], 0.);
    app.advance(std::time::Duration::from_millis(400)).unwrap();
    assert_eq!(panes(&app), [0., 0.]);
}

#[test]
fn collapse_card_pushes_the_page_down_smoothly() {
    // « Toggle collapse » : `.collapse.mb-3` grandit, les onglets qui suivent descendent
    // avec lui.
    let mut app = demo();
    let target = |app: &HeadlessApp<Demo>| {
        let root = app.root().unwrap();
        let collapse = with_class(root, "collapse")
            .into_iter()
            .find(|e| e.classes().contains(&"mb-3"))
            .unwrap()
            .bounds();
        let tabs = with_class(root, "nav-tabs")[0].bounds();
        (collapse.bottom - collapse.top, tabs.top - collapse.bottom)
    };
    assert_eq!(target(&app).0, 0.);
    let toggle = scroll_to(&mut app, "btn-outline-primary", 4);
    app.click_at(toggle).unwrap();
    app.advance(std::time::Duration::from_millis(100)).unwrap();
    let (mid, mid_gap) = target(&app);
    for _ in 0..30 {
        app.advance(std::time::Duration::from_millis(16)).unwrap();
    }
    let (open, open_gap) = target(&app);
    assert!(0. < mid && mid < open, "{mid} of {open}");
    // La marge `mb-3` (1rem) sépare la carte des onglets pendant l'animation.
    assert_eq!((mid_gap, open_gap), (16., 16.));
}

#[test]
fn accordion_button_colours_and_chevron_are_interpolated() {
    // `transition: var(--bs-accordion-transition)` (liste composée) et
    // `::after { transition: var(--bs-accordion-btn-icon-transition) }`.
    use karbon_ui::ui::scene::DrawCommand;
    let mut app = demo();
    let second = scroll_to(&mut app, "accordion-button", 1);
    let background = |app: &HeadlessApp<Demo>| {
        let button = with_class(app.root().unwrap(), "accordion-button")[1];
        button.computed_style().unwrap().local.background.unwrap()
    };
    // Rotation présentée du chevron du second bouton (`cos` de l'angle) : 1 replié. Le
    // chevron est la rotation dont le centre (point fixe) est dans le bouton.
    let chevron = |app: &HeadlessApp<Demo>| {
        let button = with_class(app.root().unwrap(), "accordion-button")[1].bounds();
        app.scene()
            .commands
            .iter()
            .find_map(|c| match c {
                DrawCommand::PushEffect { transform: m, .. } => {
                    // x = m11 x + m21 y + dx ; y = m12 x + m22 y + dy.
                    let (a, b, c, d) = (1. - m.m11, -m.m21, -m.m12, 1. - m.m22);
                    let det = a * d - b * c;
                    if det.abs() < 1e-6 {
                        return None;
                    }
                    let x = (m.dx * d - b * m.dy) / det;
                    let y = (a * m.dy - c * m.dx) / det;
                    (x > button.left && x < button.right && y > button.top && y < button.bottom)
                        .then_some(m.m11)
                }
                _ => None,
            })
            .unwrap_or(1.)
    };
    let collapsed = background(&app);
    assert_eq!(chevron(&app), 1.);
    app.click_at(second).unwrap();
    app.advance(std::time::Duration::from_millis(70)).unwrap();
    let mid = background(&app);
    let turn = chevron(&app);
    app.advance(std::time::Duration::from_millis(400)).unwrap();
    let open = background(&app);
    assert_ne!(collapsed, open);
    let between = |a: u8, m: u8, b: u8| a.min(b) <= m && m <= a.max(b);
    assert!(
        mid != collapsed
            && mid != open
            && between(collapsed.red, mid.red, open.red)
            && between(collapsed.green, mid.green, open.green),
        "{collapsed:?} → {mid:?} → {open:?}"
    );
    assert!(-1. < turn && turn < 1., "chevron mid-rotation: {turn}");
    assert!(
        (chevron(&app) + 1.).abs() < 1e-4,
        "open chevron turned 180°"
    );
}

type Colour = karbon_ui::ui::theme::Color;

/// Couleurs présentées (texte, fond, bordure) de chaque élément stylé, par identifiant.
fn colours(
    app: &HeadlessApp<Demo>,
) -> std::collections::HashMap<u64, (String, [Option<Colour>; 3])> {
    fn walk(e: &Element, out: &mut std::collections::HashMap<u64, (String, [Option<Colour>; 3])>) {
        if let Some(style) = e.computed_style() {
            out.insert(
                e.id.0,
                (
                    e.classes().join("."),
                    [style.text.color, style.local.background, style.local.border],
                ),
            );
        }
        for child in &e.children {
            walk(child, out);
        }
    }
    let mut out = std::collections::HashMap::new();
    walk(app.root().unwrap(), &mut out);
    out
}

#[test]
fn only_the_tabs_that_change_transition() {
    // `.nav-link { transition: color .15s, background-color .15s, border-color .15s }` :
    // un clic sur « Profile » anime « Profile » (qui devient actif) et « Home » (qui perd
    // `active`) ; aucun autre élément de la page (« Contact », liens de la barre de
    // navigation, boutons, accordéon) ne change de couleur malgré la reconstruction de
    // la vue.
    let mut app = demo();
    let tabs = |app: &HeadlessApp<Demo>| {
        let nav = with_class(app.root().unwrap(), "nav-tabs")[0];
        with_class(nav, "nav-link")
            .into_iter()
            .map(|e| e.id.0)
            .collect::<Vec<_>>()
    };
    let navbar = with_class(with_class(app.root().unwrap(), "navbar-nav")[0], "nav-link").len();
    let profile = scroll_to(&mut app, "nav-link", navbar + 1);
    app.advance(std::time::Duration::from_millis(400)).unwrap();
    let ids = tabs(&app);
    let before = colours(&app);
    app.click_at(profile).unwrap();
    app.advance(std::time::Duration::from_millis(50)).unwrap();
    assert_eq!(
        tabs(&app),
        ids,
        "the tabs keep their identity across the rebuild"
    );
    let mid = colours(&app);
    app.advance(std::time::Duration::from_millis(400)).unwrap();
    let after = colours(&app);
    let mut changed = mid
        .iter()
        .filter(|(id, (_, c))| before.get(id).is_some_and(|(_, b)| b != c))
        .map(|(id, (class, _))| (*id, class.clone()))
        .collect::<Vec<_>>();
    changed.sort_by_key(|(id, _)| ids.iter().position(|i| i == id));
    assert_eq!(
        changed.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        ids[..2],
        "only Home and Profile change mid-transition: {changed:?}"
    );
    assert_eq!(after[&ids[2]], before[&ids[2]], "Contact never changes");
    // Le nouvel onglet actif (et l'ancien) sont en cours d'interpolation.
    let transparent = Colour::rgba(0, 0, 0, 0);
    let within = |a: u8, m: u8, b: u8| a.min(b) <= m && m <= a.max(b);
    for (tab, name) in [(1, "Profile"), (0, "Home")] {
        let (b, m, a) = (before[&ids[tab]].1, mid[&ids[tab]].1, after[&ids[tab]].1);
        assert_ne!(b, a, "{name} changes");
        let [b_text, m_text, a_text] = [b[0], m[0], a[0]].map(Option::unwrap);
        assert!(
            m_text != b_text
                && m_text != a_text
                && within(b_text.red, m_text.red, a_text.red)
                && within(b_text.green, m_text.green, a_text.green)
                && within(b_text.blue, m_text.blue, a_text.blue),
            "{name} text: {b_text:?} → {m_text:?} → {a_text:?}"
        );
        // Fond blanc ↔ transparent : interpolé en alpha prémultiplié comme dans les
        // navigateurs, seule l'opacité change (pas de gris intermédiaire).
        let [b_bg, m_bg, a_bg] = [b[1], m[1], a[1]].map(|c| c.unwrap_or(transparent));
        assert!(
            0 < m_bg.alpha && m_bg.alpha < 255 && within(b_bg.alpha, m_bg.alpha, a_bg.alpha),
            "{name} background fades: {b_bg:?} → {m_bg:?} → {a_bg:?}"
        );
        assert_eq!(
            (m_bg.red, m_bg.green, m_bg.blue),
            (255, 255, 255),
            "{name} background stays white while it fades"
        );
    }
}
