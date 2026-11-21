//! La fenêtre de l'exemple : barre de menus, arbre à gauche, vue au centre, propriétés à
//! droite, ligne d'état ; et la question posée avant de fermer sans enregistrer.
use crate::{Editor, Msg, Panel, world::KINDS};
use karbon_ui::ui::{
    editor::{
        EditorPalette, Menu, MenuBar, MenuEntry, NumberField, Property, PropertyGrid,
        PropertySection, TreeView, VectorField, checkbox, choice, color_swatch, context_menu,
        label, read_only, text_field, tip,
    },
    prelude::*,
    runtime::PointerEvent,
    theme::Color,
};

use karbon_ui::ui::editor::ColorPicker;

const PALETTE: EditorPalette = EditorPalette::DARK;

/// Les menus, décrits d'après l'état (cases, entrées grisées).
pub fn menus(editor: &Editor) -> Vec<Menu<Msg>> {
    vec![
        Menu::new("&Fichier")
            .entry(MenuEntry::new("Nouveau niveau", Msg::New).shortcut("Ctrl+N"))
            .entry(MenuEntry::submenu(
                "Niveaux récents",
                vec![
                    MenuEntry::new("descente.niveau", Msg::About),
                    MenuEntry::new("col-du-loup.niveau", Msg::About),
                ],
            ))
            .separator()
            .entry(MenuEntry::new("Enregistrer", Msg::Save).shortcut("Ctrl+S"))
            .entry(MenuEntry::new("Quitter", Msg::Quit).shortcut("Alt+F4")),
        Menu::new("&Édition").entry(
            MenuEntry::new("Annuler", Msg::Undo)
                .shortcut("Ctrl+Z")
                .enabled(editor.level.can_undo()),
        ),
        Menu::new("&Affichage")
            .entry(MenuEntry::new("Grille", Msg::Grid).checked(editor.grid))
            .entry(MenuEntry::new("Ombres", Msg::Shadows).checked(editor.shadows))
            .separator()
            .entry(MenuEntry::new("Perspective", Msg::Projection(0)).radio(editor.projection == 0))
            .entry(
                MenuEntry::new("Vue de dessus", Msg::Projection(1)).radio(editor.projection == 1),
            ),
        Menu::new("Aid&e").entry(MenuEntry::new("À propos", Msg::About)),
    ]
}

pub fn editor(editor: &Editor, _: &ViewContext) -> Element {
    let (bar, mut overlays) = MenuBar::new("menus", &editor.menus, Msg::Menus, menus(editor))
        .palette(PALETTE)
        .build();
    let main = row(
        "main",
        [
            tree(editor).width(320),
            viewport(editor),
            properties(editor).width(380),
        ],
    )
    .height(FILL)
    .gap(1.);
    let status = label("status", editor.status.as_str(), &PALETTE)
        .height(22)
        .padding(3.);
    let page = column("page", [bar, main, status]).surface(PALETTE.border, 0.);
    if let Some(at) = editor.tree_menu {
        let entries = vec![
            MenuEntry::new("Renommer", Msg::Rename).shortcut("F2"),
            MenuEntry::new("Dupliquer", Msg::About)
                .shortcut("Ctrl+D")
                .enabled(false),
            MenuEntry::separator(),
            MenuEntry::new(
                "Visible",
                Msg::Visible(
                    !editor
                        .current()
                        .is_some_and(|k| editor.level.props(k).visible),
                ),
            )
            .checked(
                editor
                    .current()
                    .is_some_and(|k| editor.level.props(k).visible),
            ),
        ];
        overlays.push(context_menu(
            "tree-menu",
            at,
            entries,
            Msg::CloseMenu,
            PALETTE,
        ));
    }
    if editor.asking {
        overlays.push(ask_before_closing());
    }
    layers("root", page, overlays)
}

fn tree(editor: &Editor) -> Element {
    TreeView::new("tree", &editor.tree, Msg::Tree, |item| {
        let name = format!("Objet {:06}", item.key);
        let kind = KINDS[(item.key % 5) as usize].1;
        row(
            item.scope.child("line"),
            [
                label(item.scope.child("name"), name, &PALETTE),
                Text::new(kind)
                    .size(11.)
                    .color(PALETTE.muted)
                    .element(item.scope.child("kind")),
            ],
        )
        .gap(8.)
    })
    .palette(PALETTE)
    .focused(editor.panel == Panel::Tree)
    .element()
    .context_menu(|request| Msg::TreeMenu(request.position))
}

/// La place de la vue 3D : Ctrl+molette y règle le zoom.
fn viewport(editor: &Editor) -> Element {
    let text = format!(
        "Vue 3D · zoom {:.0} % · Ctrl+molette pour zoomer",
        editor.zoom * 100.
    );
    column("viewport", [label("viewport-label", text, &PALETTE)])
        .width(FILL)
        .padding(12.)
        .surface(Color::hex(0x15171A), 0.)
        .on_pointer_wheel(|event: PointerEvent| event.control.then_some(Msg::Zoom(event.wheel.1)))
}

fn properties(editor: &Editor) -> Element {
    let Some(key) = editor.current() else {
        return column(
            "no-selection",
            [label("hint", "Aucun objet choisi", &PALETTE)],
        );
    };
    let p = editor.level.props(key);
    let open = |section| !editor.folded.contains(&section);
    let f = &editor.fields;
    let object = PropertySection::new(1, "Objet", open(1))
        .row(Property::new(
            "Nom",
            text_field("name", p.name.as_str(), Msg::Name, &PALETTE),
        ))
        .row(
            Property::new(
                "Visible",
                checkbox("visible", Some(p.visible), Msg::Visible, &PALETTE),
            )
            .tooltip("Montre l'objet dans la vue et dans le jeu."),
        )
        .row(Property::new(
            "Identifiant",
            read_only("uid", format!("#{key}"), &PALETTE),
        ));
    let transform = PropertySection::new(2, "Transformation", open(2))
        .row(
            Property::new(
                "Position",
                VectorField::new(1, p.position, f, Msg::Field)
                    .step(0.01)
                    .unit(" m")
                    .element(),
            )
            .tooltip("Glisser pour changer, Maj : plus vite, Ctrl : plus fin ; clic pour taper."),
        )
        .row(Property::new(
            "Rotation",
            VectorField::new(2, p.rotation, f, Msg::Field)
                .step(0.5)
                .unit("°")
                .element(),
        ))
        .row(Property::new(
            "Échelle",
            VectorField::new(3, p.scale, f, Msg::Field)
                .range(0.001, 1000.)
                .step(0.01)
                .element(),
        ));
    let light = PropertySection::new(3, "Rendu", open(3))
        .row(Property::new(
            "Sorte",
            choice("kind", p.kind, &KINDS, Msg::Kind, &PALETTE),
        ))
        .row(
            Property::new(
                "Intensité",
                NumberField::new(4, p.intensity, f, Msg::Field)
                    .range(0., 100.)
                    .step(0.5)
                    .unit(" lx")
                    .element(),
            )
            .tooltip("Lumière donnée par l'objet, de 0 à 100 lux."),
        )
        .row(Property::new(
            "Couleur",
            color_swatch("color", Color::hex(0xFFC27A), Msg::PickColor, &PALETTE),
        ));
    let color = PropertySection::new(4, "Couleur", open(4)).row(
        Property::new(
            "Lumière",
            ColorPicker::new(1, editor.light, &editor.colors, Msg::Color)
                .palette(PALETTE)
                .element(),
        )
        .tooltip("Carré : saturation et clarté ; barre : teinte ; ou la valeur tapée."),
    );
    PropertyGrid::new("properties", Msg::Fold)
        .palette(PALETTE)
        .section(object)
        .section(transform)
        .section(light)
        .section(color)
        .element()
}

/// La question posée quand on ferme avec des changements non enregistrés.
fn ask_before_closing() -> Element {
    let buttons = row(
        "ask-buttons",
        [
            button("ask-save", "Enregistrer", Msg::SaveAndClose).tooltip_with(tip(
                "Enregistrer",
                "Écrit le niveau, puis ferme.",
                "",
                &PALETTE,
            )),
            button("ask-discard", "Ne pas enregistrer", Msg::Discard),
            button("ask-cancel", "Annuler", Msg::KeepOpen),
        ],
    )
    .gap(8.);
    let content = column(
        "ask-content",
        [
            label(
                "ask-text",
                "Le niveau a des changements non enregistrés.",
                &PALETTE,
            ),
            buttons,
        ],
    )
    .gap(14.)
    .padding(18.)
    .surface(PALETTE.field, 6.);
    modal(
        "ask",
        Modal::new("Enregistrer le niveau ?").on_dismiss(Msg::KeepOpen),
        content,
    )
}
