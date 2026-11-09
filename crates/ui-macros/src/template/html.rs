//! Tables du balisage HTML : catégories de balises, rôles sémantiques, feuille de
//! style par défaut (appliquée avant les classes) et événements valides.

/// Catégorie d'une balise.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Conteneur bloc vertical, avec son rôle d'accessibilité éventuel.
    Container(Option<&'static str>),
    /// Paragraphe ou titre : un texte enrichi.
    TextBlock,
    /// Portion de texte en ligne.
    Inline,
    LineBreak,
    Button,
    Input,
    /// `<textarea>` : champ de saisie multiligne.
    TextArea,
    Select,
    Option,
    Img,
    Hr,
    Dialog,
    Split,
    Popover,
    VirtualList,
    Svg,
    Slot,
    Component,
}
pub const CONTAINERS: &[(&str, Option<&str>)] = &[
    ("div", None),
    ("body", None),
    ("section", Some("Group")),
    ("main", Some("Main")),
    ("header", Some("Banner")),
    ("footer", Some("ContentInfo")),
    ("nav", Some("Navigation")),
    ("article", Some("Group")),
    ("aside", Some("Complementary")),
    ("form", Some("Group")),
    ("ul", Some("List")),
    ("ol", Some("List")),
    ("li", Some("ListItem")),
];
pub const TEXT_BLOCKS: &[&str] = &["p", "h1", "h2", "h3", "h4", "h5", "h6", "blockquote", "pre"];
/// Balises en ligne ; `code`, `small` et `label` seules dans un conteneur deviennent
/// un paragraphe (promotion), comme dans le tableau de la spécification.
pub const INLINE: &[&str] = &[
    "span", "b", "strong", "i", "em", "u", "s", "a", "code", "small", "label",
];
const UNSUPPORTED: &[&str] = &[
    "table", "thead", "tbody", "tfoot", "tr", "td", "th", "caption", "colgroup", "col", "video",
    "audio", "iframe", "canvas", "script", "details", "summary", "style", "object", "embed",
    "picture", "math", "template", "fieldset", "legend", "datalist", "progress", "meter", "output",
];
pub fn kind(name: &str) -> Result<Kind, String> {
    if name.starts_with(|c: char| c.is_ascii_uppercase()) || name.contains("::") {
        return Ok(Kind::Component);
    }
    if let Some((_, role)) = CONTAINERS.iter().find(|(n, _)| *n == name) {
        return Ok(Kind::Container(*role));
    }
    if TEXT_BLOCKS.contains(&name) {
        return Ok(Kind::TextBlock);
    }
    if INLINE.contains(&name) {
        return Ok(Kind::Inline);
    }
    Ok(match name {
        "br" => Kind::LineBreak,
        "button" => Kind::Button,
        "input" => Kind::Input,
        "textarea" => Kind::TextArea,
        "select" => Kind::Select,
        "option" => Kind::Option,
        "img" => Kind::Img,
        "hr" => Kind::Hr,
        "dialog" => Kind::Dialog,
        "k-split" => Kind::Split,
        "k-popover" => Kind::Popover,
        "k-virtual-list" => Kind::VirtualList,
        "svg" => Kind::Svg,
        "slot" => Kind::Slot,
        _ if UNSUPPORTED.contains(&name) => {
            let hint = match name {
                "style" => {
                    " Use css_file!/styles! and class=\"...\"; <style> in .ui files is planned."
                }
                "table" | "tr" | "td" | "th" | "thead" | "tbody" => {
                    " Use <div style=\"display:grid; grid-template-columns:…\">."
                }
                _ => "",
            };
            return Err(format!("<{name}> is not supported.{hint}"));
        }
        _ => {
            let known = all_tags();
            let hint = suggest(name, &known)
                .map(|s| format!(" Did you mean <{s}>?"))
                .unwrap_or_default();
            return Err(format!(
                "Unknown tag <{name}>.{hint} Lowercase tags are HTML; Rust components start with an uppercase letter."
            ));
        }
    })
}
fn all_tags() -> Vec<&'static str> {
    let mut tags = CONTAINERS.iter().map(|(n, _)| *n).collect::<Vec<_>>();
    tags.extend(TEXT_BLOCKS);
    tags.extend(INLINE);
    tags.extend([
        "br",
        "button",
        "input",
        "textarea",
        "select",
        "option",
        "img",
        "hr",
        "dialog",
        "k-split",
        "k-popover",
        "k-virtual-list",
        "svg",
        "slot",
    ]);
    tags
}
/// Distance d'édition (pour les suggestions).
pub fn distance(a: &str, b: &str) -> usize {
    let a = a.chars().collect::<Vec<_>>();
    let b = b.chars().collect::<Vec<_>>();
    let mut row = (0..=b.len()).collect::<Vec<_>>();
    for i in 1..=a.len() {
        let mut previous = row[0];
        row[0] = i;
        for j in 1..=b.len() {
            let current = row[j];
            row[j] = if a[i - 1] == b[j - 1] {
                previous
            } else {
                1 + previous.min(row[j]).min(row[j - 1])
            };
            previous = current;
        }
    }
    row[b.len()]
}
/// Suggestion la plus proche, si elle est assez proche.
pub fn suggest<'a>(name: &str, candidates: &[&'a str]) -> Option<&'a str> {
    candidates
        .iter()
        .map(|c| (distance(name, c), *c))
        .filter(|(d, c)| *d <= 2.max(c.len() / 3) && *d < c.len())
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

/// Feuille de style par défaut d'une balise : propriétés de nœud (nom, code Rust)
/// appliquées avant les classes de l'application. Les pixels sont multipliés par
/// l'échelle du contexte par le générateur de propriétés.
pub fn defaults(name: &str) -> Vec<(&'static str, String)> {
    let heading = |size: u32| {
        vec![
            ("font_size", format!("{size}f32")),
            ("weight", "700u16".into()),
        ]
    };
    match name {
        "h1" => heading(30),
        "h2" => heading(24),
        "h3" => heading(20),
        "h4" => heading(17),
        "h5" => heading(15),
        "h6" => heading(13),
        "p" => vec![("margin_bottom", "12f32".into())],
        "ul" | "ol" => vec![("padding_left", "24f32".into())],
        "blockquote" => vec![("padding_left", "20f32".into())],
        "b" | "strong" => vec![("weight", "700u16".into())],
        "small" => vec![("font_size", "13f32".into())],
        "code" | "pre" => vec![(
            "font_family",
            "__ui_root::ui::font::MONOSPACE".into(),
        )],
        "u" => vec![(
            "text_decoration",
            "__ui_root::ui::styling::TextDecoration{underline:true,line_through:false,overline:false}"
                .into(),
        )],
        "s" => vec![(
            "text_decoration",
            "__ui_root::ui::styling::TextDecoration{underline:false,line_through:true,overline:false}"
                .into(),
        )],
        "hr" => vec![
            ("margin_top", "8f32".into()),
            ("margin_bottom", "8f32".into()),
        ],
        "button" => vec![("width", "__ui_root::ui::view::AUTO".into())],
        _ => Vec::new(),
    }
}
/// Style de portion par défaut d'une balise en ligne (code d'un `SpanStyle`).
pub fn span_defaults(name: &str) -> String {
    let base = "__ui_root::ui::styling::text::SpanStyle::new()";
    match name {
        "b" | "strong" => format!("{base}.bold()"),
        "i" | "em" | "blockquote" => format!("{base}.italic()"),
        "u" | "a" => format!("{base}.underline()"),
        "s" => format!("{base}.strike()"),
        "code" => format!("{base}.family(__ui_root::ui::font::MONOSPACE)"),
        "small" => format!("{base}.size(13f32*__ui_context.scale)"),
        _ => base.into(),
    }
}

/// Événements valides d'une balise (pour les messages d'erreur).
pub fn events(kind: Kind, input: Option<&str>) -> Vec<&'static str> {
    let mut list = match (kind, input) {
        (Kind::Input, Some("checkbox" | "radio")) => vec!["onchange"],
        (Kind::Input, Some("range")) => vec!["oninput", "onchange"],
        (Kind::Input, _) => vec!["oninput", "onchange", "onkeydown"],
        (Kind::TextArea, _) => vec!["oninput", "onkeydown"],
        (Kind::Select, _) => vec!["onchange"],
        (Kind::Dialog | Kind::Popover, _) => vec!["onclose"],
        (Kind::Button, _) => vec!["onclick"],
        (Kind::Container(Some("Group")), _) => vec!["onclick", "onsubmit"],
        _ => vec!["onclick"],
    };
    list.extend([
        "oncontextmenu",
        "ondrop",
        "onscroll",
        "onwheel",
        "onhover",
        "onpress",
        "onfocus",
        "ondblclick",
    ]);
    list
}
pub const EVENTS: &[&str] = &[
    "onclick",
    "oninput",
    "onchange",
    "onclose",
    "oncontextmenu",
    "ondrop",
    "onscroll",
    "onwheel",
    "onsubmit",
    "onkeydown",
    "onhover",
    "onpress",
    "onfocus",
    "ondblclick",
];
/// Attributs communs acceptés sur tout élément bloc.
pub const COMMON: &[&str] = &[
    "id",
    "class",
    "style",
    "title",
    "hidden",
    "aria-label",
    "aria-hidden",
    "aria-expanded",
    "aria-haspopup",
    "aria-pressed",
    "aria-checked",
    "aria-selected",
    "aria-current",
    "role",
    "tabindex",
    "draggable",
    "dropzone",
    "slot",
    "data-exit",
    "data-stagger",
    "data-replay",
    "data-reveal",
];

/// Animations de sortie prédéfinies : (nom, code `Animation`).
pub const EXIT_PRESETS: &[(&str, &str)] = &[
    ("fade-out", "Animation::fade_out()"),
    (
        "slide-out-up",
        "Animation::from_to(Effect::NONE,Effect::new().translate(0.,-24.).opacity(0.))",
    ),
    (
        "slide-out-down",
        "Animation::from_to(Effect::NONE,Effect::new().translate(0.,24.).opacity(0.))",
    ),
    (
        "slide-out-left",
        "Animation::from_to(Effect::NONE,Effect::new().translate(-24.,0.).opacity(0.))",
    ),
    (
        "slide-out-right",
        "Animation::from_to(Effect::NONE,Effect::new().translate(24.,0.).opacity(0.))",
    ),
    (
        "scale-out",
        "Animation::from_to(Effect::NONE,Effect::new().scale(0.9).opacity(0.))",
    ),
];
/// Durée CSS (`200ms`, `0.3s`) en millisecondes.
pub fn duration(value: &str) -> Option<u32> {
    let n = if let Some(n) = value.strip_suffix("ms") {
        n.parse::<f64>().ok()?
    } else {
        value.strip_suffix('s')?.parse::<f64>().ok()? * 1000.
    };
    (n.is_finite() && (0. ..=600_000.).contains(&n)).then(|| n.round() as u32)
}
pub fn easing(value: &str) -> Option<&'static str> {
    Some(match value {
        "linear" => "Linear",
        "ease" => "Ease",
        "ease-in" => "EaseIn",
        "ease-out" => "EaseOut",
        "ease-in-out" => "EaseInOut",
        _ => return None,
    })
}
