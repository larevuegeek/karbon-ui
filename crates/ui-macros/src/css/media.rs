//! Requêtes `@media` et `@container` (caractéristiques et syntaxe d'intervalle)
//! compilées en conditions `StyleState` sur les dimensions de la fenêtre ou du conteneur.
use super::{parser, values};
type Result<T> = std::result::Result<T, String>;
const STATE: &str = "__ui_root::ui::styling::StyleState";
/// Préfixe des conditions de conteneur dans la liste des conditions d'une règle.
pub(super) const CONTAINER: &str = "@container ";

/// Bornes inclusives d'une requête (largeur, hauteur) et orientation éventuelle.
struct Bounds {
    min_w: f32,
    max_w: f32,
    min_h: f32,
    max_h: f32,
    landscape: &'static str,
    reduced_motion: &'static str,
    dark: &'static str,
    high_contrast: &'static str,
}
impl Default for Bounds {
    fn default() -> Self {
        Self {
            min_w: 0.,
            max_w: f32::INFINITY,
            min_h: 0.,
            max_h: f32::INFINITY,
            landscape: "None",
            reduced_motion: "None",
            dark: "None",
            high_contrast: "None",
        }
    }
}
/// Longueur d'une requête : px, rem/em (16 px, comme un navigateur) ou 0.
fn length(value: &str) -> Result<f32> {
    let value = value.trim();
    let n = if let Some(n) = value
        .strip_suffix("rem")
        .or_else(|| value.strip_suffix("em"))
    {
        values::number(n)? * 16.
    } else {
        values::px(value, false)?
    };
    if n < 0. {
        return Err("Query lengths cannot be negative".into());
    }
    Ok(n)
}
/// Écart appliqué aux comparaisons strictes (`<`, `>`).
const STRICT: f32 = 0.01;
impl Bounds {
    fn apply(&mut self, feature: &str, op: &str, n: f32) -> Result<()> {
        let (min, max) = match feature {
            "width" | "inline-size" => (&mut self.min_w, &mut self.max_w),
            "height" | "block-size" => (&mut self.min_h, &mut self.max_h),
            _ => return Err(format!("Unsupported range feature '{feature}'")),
        };
        match op {
            ">=" => *min = min.max(n),
            ">" => *min = min.max(n + STRICT),
            "<=" => *max = max.min(n),
            "<" => *max = max.min(n - STRICT),
            "=" => {
                *min = min.max(n);
                *max = max.min(n);
            }
            _ => return Err(format!("Unsupported comparison '{op}'")),
        }
        Ok(())
    }
    /// Une condition entre parenthèses : `min-width: 600px`, `width >= 40rem`,
    /// `400px < width <= 800px`, `orientation: landscape` (media seulement).
    fn feature(&mut self, inner: &str, media: bool) -> Result<()> {
        let inner = inner.trim();
        if let Some((feature, value)) = inner.split_once(':') {
            let (feature, value) = (feature.trim(), value.trim());
            if feature == "orientation" {
                if !media {
                    return Err("orientation is not supported in container queries".into());
                }
                self.landscape = match value {
                    "landscape" => "Some(true)",
                    "portrait" => "Some(false)",
                    _ => return Err("Invalid media orientation".into()),
                };
                return Ok(());
            }
            if feature == "prefers-reduced-motion" {
                if !media {
                    return Err(
                        "prefers-reduced-motion is not supported in container queries".into(),
                    );
                }
                self.reduced_motion = match value {
                    "reduce" => "Some(true)",
                    "no-preference" => "Some(false)",
                    _ => return Err("Use prefers-reduced-motion: reduce or no-preference".into()),
                };
                return Ok(());
            }
            if feature == "prefers-color-scheme" || feature == "prefers-contrast" {
                if !media {
                    return Err(format!("{feature} is not supported in container queries"));
                }
                if feature == "prefers-color-scheme" {
                    self.dark = match value {
                        "dark" => "Some(true)",
                        "light" => "Some(false)",
                        _ => return Err("Use prefers-color-scheme: dark or light".into()),
                    };
                } else {
                    self.high_contrast = match value {
                        "more" => "Some(true)",
                        "no-preference" => "Some(false)",
                        _ => return Err("Use prefers-contrast: more or no-preference".into()),
                    };
                }
                return Ok(());
            }
            let n = length(value)?;
            let (op, feature) = match feature {
                f if f.starts_with("min-") => (">=", &f[4..]),
                f if f.starts_with("max-") => ("<=", &f[4..]),
                f => ("=", f),
            };
            return self
                .apply(feature, op, n)
                .map_err(|_| format!("Unsupported media feature '{inner}'"));
        }
        // Syntaxe de plage : jetons séparés par des comparateurs.
        let mut tokens = Vec::new();
        let mut rest = inner;
        while !rest.is_empty() {
            let at = rest.find(['<', '>', '=']).unwrap_or(rest.len());
            if at > 0 {
                tokens.push(rest[..at].trim());
            }
            rest = &rest[at..];
            if rest.is_empty() {
                break;
            }
            let op_len = if rest[1..].starts_with('=') { 2 } else { 1 };
            tokens.push(&rest[..op_len]);
            rest = rest[op_len..].trim_start();
        }
        let flip = |op: &str| {
            match op {
                "<" => ">",
                ">" => "<",
                "<=" => ">=",
                ">=" => "<=",
                other => other,
            }
            .to_string()
        };
        let is_feature = |t: &str| t.chars().all(|c| c.is_ascii_alphabetic() || c == '-');
        match tokens.as_slice() {
            [f, op, v] if is_feature(f) => self.apply(f, op, length(v)?),
            [v, op, f] if is_feature(f) => self.apply(f, &flip(op), length(v)?),
            [a, op1, f, op2, b]
                if is_feature(f) && op1.starts_with('<') == op2.starts_with('<') =>
            {
                self.apply(f, &flip(op1), length(a)?)?;
                self.apply(f, op2, length(b)?)
            }
            _ => Err(format!(
                "Unsupported query condition '({inner})'; use (min-width: 600px), (width >= 40rem) or (400px < width <= 800px)"
            )),
        }
    }
    /// Conditions reliées par `and`.
    fn conditions(&mut self, words: &[String], media: bool) -> Result<()> {
        for (i, part) in words.iter().enumerate() {
            if i % 2 == 1 {
                if part != "and" {
                    return Err(if part == "or" {
                        "or is not supported in queries; use separate rules".into()
                    } else {
                        "Query conditions are joined with and".into()
                    });
                }
                continue;
            }
            let inner = part
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .ok_or("Query features require parentheses")?;
            self.feature(inner, media)?;
        }
        if words.len().is_multiple_of(2) {
            return Err("Missing query condition after and".into());
        }
        Ok(())
    }
}
fn num(n: f32) -> String {
    if n.is_infinite() {
        "::core::primitive::f32::INFINITY".into()
    } else {
        format!("{n:?}")
    }
}
/// Exigences d'une requête `@media` portant uniquement sur le thème et le contraste
/// (`prefers-color-scheme`, `prefers-contrast`) : `(sombre, contraste élevé)`, `None`
/// pour une préférence non contrainte ; `Err` pour toute autre requête.
pub(super) fn color_preferences(query: &str) -> Result<(Option<bool>, Option<bool>)> {
    let refuse = || {
        "Conditional :root variables only support prefers-color-scheme and prefers-contrast"
            .to_string()
    };
    let parts = parser::split(query, ',')?;
    let [part] = parts.as_slice() else {
        return Err(refuse());
    };
    let mut words = values::words(part)?;
    if words.first().is_some_and(|s| s == "only") {
        words.remove(0);
    }
    if words.first().is_some_and(|s| s == "screen" || s == "all") {
        words.remove(0);
        if words.first().is_some_and(|s| s == "and") {
            words.remove(0);
        }
    }
    let mut b = Bounds::default();
    b.conditions(&words, true)?;
    let unbounded =
        b.min_w == 0. && b.min_h == 0. && b.max_w.is_infinite() && b.max_h.is_infinite();
    if !unbounded || b.landscape != "None" || b.reduced_motion != "None" {
        return Err(refuse());
    }
    let require = |v: &str| match v {
        "Some(true)" => Some(true),
        "Some(false)" => Some(false),
        _ => None,
    };
    Ok((require(b.dark), require(b.high_contrast)))
}
/// Code booléen d'une exigence de préférences, évalué à la construction de la vue.
pub(super) fn color_test(dark: Option<bool>, high_contrast: Option<bool>) -> String {
    let (d, h) = ("__ui_scheme.0", "__ui_scheme.1");
    let mut tests = Vec::new();
    match dark {
        Some(true) => tests.push(d.to_string()),
        Some(false) => tests.push(format!("!{d}")),
        None => {}
    }
    match high_contrast {
        Some(true) => tests.push(h.to_string()),
        Some(false) => tests.push(format!("!{h}")),
        None => {}
    }
    if tests.is_empty() {
        "true".into()
    } else {
        tests.join("&&")
    }
}
/// Si `predicate` (code d'une condition de règle) n'exige que le thème et le contraste :
/// le test correspondant, évaluable à la construction de la vue.
pub(in crate::css) fn scheme_only(predicate: &str) -> Option<String> {
    const OPEN: &str =
        "__ui_root::ui::styling::StyleState::Media(__ui_root::ui::styling::MediaQuery{";
    let media = predicate
        .strip_prefix("__ui_root::ui::styling::StyleState::All(&[")
        .and_then(|s| s.strip_suffix("])"))
        .unwrap_or(predicate);
    let fields = media.strip_prefix(OPEN)?.strip_suffix("})")?;
    if fields.contains(OPEN) {
        return None;
    }
    let field = |name: &str| {
        fields
            .split(',')
            .find_map(|f| f.trim().strip_prefix(&format!("{name}:")).map(str::trim))
    };
    let unbounded = field("min_width") == Some("0.0")
        && field("max_width") == Some("::core::primitive::f32::INFINITY")
        && field("min_height") == Some("0.0")
        && field("max_height") == Some("::core::primitive::f32::INFINITY")
        && field("landscape") == Some("None")
        && field("reduced_motion") == Some("None");
    if !unbounded {
        return None;
    }
    let require = |name: &str| match field(name)? {
        "Some(true)" => Some(Some(true)),
        "Some(false)" => Some(Some(false)),
        "None" => Some(None),
        _ => None,
    };
    let (dark, high_contrast) = (require("dark")?, require("high_contrast")?);
    (dark.is_some() || high_contrast.is_some()).then(|| color_test(dark, high_contrast))
}
/// Condition d'une règle : media query, ou requête de conteneur préfixée par
/// [`CONTAINER`].
pub(super) fn condition(query: &str) -> Result<String> {
    if let Some(container) = query.strip_prefix(CONTAINER) {
        return container_condition(container);
    }
    if query.trim().is_empty() {
        return Err("Media query cannot be empty".into());
    }
    let mut alternatives = Vec::new();
    for query in parser::split(query, ',')? {
        if query.trim().is_empty() {
            return Err("Media query cannot be empty".into());
        }
        let mut words = values::words(query)?;
        let invert = words.first().is_some_and(|s| s == "not");
        if invert || words.first().is_some_and(|s| s == "only") {
            words.remove(0);
        }
        let media_type = words.first().is_some_and(|s| s == "screen" || s == "all");
        if media_type {
            words.remove(0);
            if words.first().is_some_and(|s| s == "and") {
                words.remove(0);
                if words.is_empty() {
                    return Err("Missing media condition after and".into());
                }
            }
        }
        if words.is_empty() && !media_type {
            return Err("Missing media condition".into());
        }
        let mut b = Bounds::default();
        if !words.is_empty() {
            b.conditions(&words, true)?;
        }
        let c = format!(
            "{STATE}::Media(__ui_root::ui::styling::MediaQuery{{min_width: {},max_width: {},min_height: {},max_height: {},landscape:{},reduced_motion:{},dark:{},high_contrast:{}}})",
            num(b.min_w),
            num(b.max_w),
            num(b.min_h),
            num(b.max_h),
            b.landscape,
            b.reduced_motion,
            b.dark,
            b.high_contrast
        );
        alternatives.push(if invert {
            format!("{STATE}::Not(&{c})")
        } else {
            c
        });
    }
    Ok(if alternatives.len() == 1 {
        alternatives.remove(0)
    } else {
        format!("{STATE}::Any(&[{}])", alternatives.join(","))
    })
}
/// `@container [nom] [not] (condition) [and (condition)…]`.
fn container_condition(query: &str) -> Result<String> {
    let mut words = values::words(query)?;
    if words.iter().any(|w| w.starts_with("style(")) {
        return Err("Style queries (@container style(…)) are not supported".into());
    }
    let name = match words.first() {
        Some(w) if !w.starts_with('(') && w != "not" => {
            let name = words.remove(0);
            if !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                || name.starts_with(|c: char| c.is_ascii_digit())
            {
                return Err(format!("Invalid container name '{name}'"));
            }
            if ["none", "and", "or", "style"].contains(&name.as_str()) {
                return Err(format!("'{name}' cannot be a container name"));
            }
            Some(name)
        }
        _ => None,
    };
    let invert = words.first().is_some_and(|s| s == "not");
    if invert {
        words.remove(0);
    }
    if words.is_empty() {
        return Err("@container needs a size condition, e.g. @container (min-width: 400px)".into());
    }
    let mut b = Bounds::default();
    b.conditions(&words, false)?;
    let c = format!(
        "{STATE}::Container(__ui_root::ui::styling::ContainerQuery{{name: {},min_width: {},max_width: {},min_height: {},max_height: {}}})",
        name.map_or("None".into(), |n| format!("Some({n:?})")),
        num(b.min_w),
        num(b.max_w),
        num(b.min_h),
        num(b.max_h),
    );
    Ok(if invert {
        format!("{STATE}::Not(&{c})")
    } else {
        c
    })
}
