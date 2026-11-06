//! Propriétés logiques traduites en côtés physiques et longhands couverts par une
//! propriété raccourcie (élimination des déclarations masquées dans la cascade).

/// Propriétés logiques (`padding-inline`, `margin-block-start`, `inset-inline`…)
/// traduites en côtés physiques pour un texte horizontal de gauche à droite ; les autres
/// propriétés sont rendues telles quelles.
pub fn logical(name: &str, value: &str) -> Vec<(String, String)> {
    let (prefix, rest) = if let Some(r) = name.strip_prefix("padding-") {
        ("padding", r)
    } else if let Some(r) = name.strip_prefix("margin-") {
        ("margin", r)
    } else if let Some(r) = name.strip_prefix("inset-") {
        ("", r)
    } else if let Some(r) = name
        .strip_prefix("border-")
        .and_then(|r| r.strip_suffix("-width"))
    {
        ("border-width", r)
    } else {
        return vec![(name.to_owned(), value.to_owned())];
    };
    let side = |s: &str| match prefix {
        "" => s.to_owned(),
        "border-width" => format!("border-{s}-width"),
        p => format!("{p}-{s}"),
    };
    let pair = |a: &str, b: &str| {
        let words = crate::css::values::words(value).unwrap_or_default();
        let (first, second) = match words.as_slice() {
            [x, y] => (x.clone(), y.clone()),
            _ => (value.to_owned(), value.to_owned()),
        };
        vec![(side(a), first), (side(b), second)]
    };
    match rest {
        "inline" => pair("left", "right"),
        "block" => pair("top", "bottom"),
        "inline-start" => vec![(side("left"), value.to_owned())],
        "inline-end" => vec![(side("right"), value.to_owned())],
        "block-start" => vec![(side("top"), value.to_owned())],
        "block-end" => vec![(side("bottom"), value.to_owned())],
        _ => vec![(name.to_owned(), value.to_owned())],
    }
}
/// Longhands couverts par une propriété (pour l'élimination des déclarations masquées).
pub fn coverage(name: &str) -> Vec<String> {
    let sides = |prefix: &str, suffix: &str| {
        ["top", "right", "bottom", "left"]
            .iter()
            .map(|s| format!("{prefix}-{s}{suffix}"))
            .collect::<Vec<_>>()
    };
    match name {
        "padding" | "margin" => sides(name, ""),
        "inset" => ["top", "right", "bottom", "left"]
            .map(String::from)
            .to_vec(),
        "border-width" => sides("border", "-width"),
        "border-color" => sides("border", "-color"),
        "border-style" => sides("border", "-style"),
        "border" => {
            let mut v = sides("border", "-width");
            v.extend(sides("border", "-color"));
            v.extend(sides("border", "-style"));
            v
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            ["-width", "-color", "-style"]
                .iter()
                .map(|s| format!("{name}{s}"))
                .collect()
        }
        "border-radius" => [
            "border-top-left-radius",
            "border-top-right-radius",
            "border-bottom-right-radius",
            "border-bottom-left-radius",
        ]
        .map(String::from)
        .to_vec(),
        "background" => [
            "background-color",
            "background-image",
            "background-position",
            "background-size",
            "background-repeat",
        ]
        .map(String::from)
        .to_vec(),
        "outline" => ["outline-width", "outline-color", "outline-style"]
            .map(String::from)
            .to_vec(),
        "gap" => ["row-gap", "column-gap"].map(String::from).to_vec(),
        "overflow" => ["overflow-x", "overflow-y"].map(String::from).to_vec(),
        "flex" => ["flex-grow", "flex-shrink", "flex-basis"]
            .map(String::from)
            .to_vec(),
        "font" => [
            "font-size",
            "font-family",
            "font-weight",
            "font-style",
            "line-height",
        ]
        .map(String::from)
        .to_vec(),
        "text-decoration" => ["text-decoration-line"].map(String::from).to_vec(),
        "transition" => [
            "transition-property",
            "transition-duration",
            "transition-timing-function",
            "transition-delay",
        ]
        .map(String::from)
        .to_vec(),
        "animation" => ["animation-name", "animation-duration"]
            .map(String::from)
            .to_vec(),
        "-webkit-line-clamp" => vec!["line-clamp".into()],
        "white-space" | "text-wrap" => vec!["white-space".into()],
        "-webkit-backdrop-filter" => vec!["backdrop-filter".into()],
        _ => vec![name.to_owned()],
    }
}
