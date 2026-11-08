//! Mémoires du moteur (mesures d'une passe de layout, mesures de texte d'une image à
//! l'autre, résolution partielle des styles, cartes mémorisées `memo`) : la même suite
//! d'interactions donne, avec et sans elles, le même arbre placé, les mêmes textes
//! peints, le même arbre accessible et les mêmes pixels.
use crate::support::*;
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{prelude::*, runtime::UiEvent, scene::PointF},
};
use std::time::Duration;
css_file!(mod css, "incremental.css");

#[derive(Default)]
struct Page {
    narrow: bool,
    title: String,
    cards: usize,
}

const WORDS: [&str; 6] = [
    "Rumours",
    "The Dark Side of the Moon",
    "Abbey Road (Remastered 2019) — édition anniversaire",
    "Kind of Blue",
    "OK Computer OKNOTOK 1997 2017",
    "Blue Train",
];

impl Application for Page {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        let narrow = self.narrow;
        let title = self.title.as_str();
        let cards: Vec<&str> = (0..self.cards).map(|i| WORDS[i % WORDS.len()]).collect();
        ui! { @context(cx); @styles(css);
            <div class="page">
                <div class="side" class:narrow={narrow}>
                    {#each WORDS as (i, w) (i)}
                        <div class="item"><div class="dot"></div><p class="label">{w}</p></div>
                    {/each}
                </div>
                <div class="main">
                    <div class="head">
                        <h1 class="title">{title}</h1>
                        <p class="count">{"Bibliothèque · 451 titres"}</p>
                        <input class="search" value="recherche" readonly />
                    </div>
                    <div class="hero">
                        <div class="cover"></div>
                        <div class="text">
                            <p class="line clamp">{format!("{title} — un texte assez long pour se couper sur plusieurs lignes, puis se terminer par une ellipse quand il dépasse deux lignes.")}</p>
                            <div class="tags">
                                <span class="tag">Rock</span><span class="tag">Jazz</span>
                                <span class="tag">{"Musique de chambre"}</span><span class="tag">{"Électro"}</span>
                            </div>
                        </div>
                    </div>
                    <div class="grid">
                        {#each cards as (i, w) (i)}
                            <div class="card" memo={(i, w)}>
                                <div class="badge"></div>
                                <p class="name">{w}</p>
                                <p class="name">{format!("{i} titres, 42 min")}</p>
                            </div>
                        {/each}
                    </div>
                    <div class="panel">
                        <p class="cell">{"Conteneur interrogé par sa largeur"}</p>
                        <p class="cell">Seconde colonne du panneau</p>
                    </div>
                    <div class="columns">
                        <p class="col">Interminablement</p>
                        <p class="col">{"Une colonne souple qui reprend la place restante et se coupe"}</p>
                        <p class="col">{"Ajustée au contenu"}</p>
                    </div>
                    <p class="justified">{"Un paragraphe justifié dont les lignes s’étirent à la largeur de la boîte, sauf la dernière qui reste alignée au début."}</p>
                    <p class="centered">{"Texte centré, espacé d’un pixel"}</p>
                </div>
            </div>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

/// Bornes de chaque élément, textes peints, arbre accessible et, sur demande, pixels.
fn snapshot(app: &mut HeadlessApp<Page>, pixels: bool) -> Vec<String> {
    fn walk(node: &Element, out: &mut Vec<String>) {
        let b = node.bounds();
        out.push(format!(
            "{:?} {:?}",
            node.id,
            [b.left, b.top, b.right, b.bottom].map(f32::to_bits)
        ));
        for child in &node.children {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(app.root().unwrap(), &mut out);
    out.extend(app.texts().iter().map(|t| t.to_string()));
    out.extend(app.accessibility().iter().map(|e| {
        format!(
            "{:?} {} {:?} {:?} {:?}",
            e.id, e.name, e.bounds, e.clipped_bounds, e.value
        )
    }));
    if pixels {
        let image = app.render(1.).unwrap();
        out.push(format!("{:?}", image.pixels));
    }
    out
}

/// Joue la suite d'interactions et relève l'état après chaque étape.
fn play(incremental: bool) -> Vec<Vec<String>> {
    let mut app = HeadlessApp::with_options(
        Page {
            narrow: false,
            title: "Accueil".into(),
            cards: 7,
        },
        HeadlessOptions {
            width: 1100.,
            height: 800.,
            ..Default::default()
        },
    )
    .unwrap();
    app.runtime_mut().set_incremental(incremental);
    app.prepare().unwrap();
    let mut steps = vec![snapshot(&mut app, true)];
    let frame = Duration::from_millis(16);
    // Redimensionnement à travers la requête média (700 px) et celle du conteneur, avec
    // des largeurs fractionnaires.
    for i in 0..40 {
        let width = 1100. - 13.37 * i as f32;
        app.resize(width, 800. - (i % 3) as f32).unwrap();
        steps.push(snapshot(&mut app, i % 8 == 0));
    }
    for i in 0..40 {
        app.resize(565. + 14.1 * i as f32, 800.).unwrap();
        steps.push(snapshot(&mut app, i % 8 == 0));
    }
    // Transition de largeur de la barre latérale, aller et retour.
    for narrow in [true, false] {
        app.update_app(|page| page.narrow = narrow).unwrap();
        for i in 0..16 {
            app.advance(frame).unwrap();
            steps.push(snapshot(&mut app, i % 5 == 0));
        }
    }
    // Contenu modifié : titre plus long, cartes ajoutées puis retirées.
    for (title, cards) in [("Une très longue page d’accueil", 11), ("Court", 3)] {
        app.update_app(|page| {
            page.title = title.into();
            page.cards = cards;
        })
        .unwrap();
        steps.push(snapshot(&mut app, true));
        app.resize(780., 700.).unwrap();
        steps.push(snapshot(&mut app, true));
        app.resize(1100., 800.).unwrap();
        steps.push(snapshot(&mut app, false));
    }
    // Survol par petits pas : le plus souvent sur le même élément, parfois sur le
    // suivant (règles `:hover`, transition de couleur, découpe arrondie).
    for i in 0..90 {
        let (x, y) = if i < 60 {
            (240. + 9.3 * i as f32, 260. + 2. * (i % 7) as f32)
        } else {
            (30. + (i % 3) as f32, 20. + 7. * (i - 60) as f32)
        };
        app.event(UiEvent::PointerMove(PointF { x, y })).unwrap();
        app.advance(frame / 2).unwrap();
        steps.push(snapshot(&mut app, i % 15 == 0));
    }
    // Reconstructions (cartes mémorisées reprises) pendant que le pointeur passe de
    // carte en carte, avec des transitions en cours.
    for i in 0..24 {
        let (x, y) = (240. + 37. * (i % 12) as f32, 260. + 60. * (i / 12) as f32);
        app.event(UiEvent::PointerMove(PointF { x, y })).unwrap();
        app.update_app(|_| {}).unwrap();
        app.advance(frame / 2).unwrap();
        steps.push(snapshot(&mut app, i % 6 == 0));
    }
    // Avec les mémoires, le survol ne refait que les éléments touchés.
    assert_eq!(
        app.runtime_mut().stats().partial_styles > 0,
        incremental,
        "résolution partielle des styles"
    );
    // Défilement sous un pointeur immobile.
    for i in 0..10 {
        app.event(UiEvent::Scroll {
            position: PointF { x: 600., y: 400. },
            delta_x: 0.,
            delta_y: if i < 7 { 30. } else { -45. },
        })
        .unwrap();
        steps.push(snapshot(&mut app, i == 9));
    }
    steps
}

#[test]
fn caches_give_the_same_layout_text_and_pixels_as_a_full_computation() {
    let cached = play(true);
    let full = play(false);
    assert_eq!(cached.len(), full.len());
    for (step, (a, b)) in cached.iter().zip(&full).enumerate() {
        if a != b {
            let line = a.iter().zip(b).position(|(x, y)| x != y).unwrap_or(0);
            panic!(
                "step {step} differs: {:?} / {:?}",
                a.get(line).map(|s| &s[..s.len().min(200)]),
                b.get(line).map(|s| &s[..s.len().min(200)])
            );
        }
    }
}
