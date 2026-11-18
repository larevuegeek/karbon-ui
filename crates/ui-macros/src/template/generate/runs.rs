//! Texte enrichi : suite de portions (texte, valeurs, liens, blocs) dont les blancs
//! sont regroupés comme en HTML avant d'émettre le code des segments.
use super::{Generator, R, element::kind_of};
use crate::{
    Result,
    template::{html::Kind, markup::Node},
};

/// Style d'une portion : code de son `SpanStyle`, et code de sa couleur lue dans des
/// variables (`Option<&'static ColorExpression>`), vide sans couleur variable.
#[derive(Clone, Default)]
pub(super) struct Span {
    style: String,
    color: String,
}
impl Span {
    /// Couleur variable posée sur la portion émise juste avant.
    fn color(&self) -> String {
        if self.color.is_empty() {
            String::new()
        } else {
            format!("__ui_rich=__ui_rich.variable_color({});", self.color)
        }
    }
}
/// Portion d'un texte enrichi, avant regroupement des blancs.
pub(super) enum Run {
    Text(String, Span),
    Break,
    Value(String, Span),
    Link(String, Span, String),
    Prelude(String),
    If(Vec<(String, Vec<Run>)>, Option<Vec<Run>>),
    /// Ouverture, corps et fermeture d'un `{#each}` ou d'un `{@const}`.
    Each(String, Vec<Run>, &'static str),
}
/// Regroupe les blancs comme en HTML ; renvoie `true` si la suite finit par un blanc.
pub(super) fn collapse(runs: &mut Vec<Run>, mut space: bool) -> bool {
    for run in runs.iter_mut() {
        match run {
            Run::Text(text, _) => {
                let mut out = String::with_capacity(text.len());
                for c in text.chars() {
                    if c.is_whitespace() && c != '\u{a0}' {
                        if !space {
                            out.push(' ');
                            space = true;
                        }
                    } else {
                        out.push(c);
                        space = false;
                    }
                }
                *text = out;
            }
            Run::Break => space = true,
            Run::Value(..) | Run::Link(..) => space = false,
            Run::Prelude(_) => {}
            Run::If(branches, otherwise) => {
                for (_, b) in branches.iter_mut() {
                    collapse(b, space);
                }
                if let Some(b) = otherwise {
                    collapse(b, space);
                }
                space = false;
            }
            Run::Each(_, body, _) => {
                collapse(body, space);
                space = false;
            }
        }
    }
    runs.retain(|r| !matches!(r, Run::Text(t, _) if t.is_empty()));
    space
}
pub(super) fn trim_end(runs: &mut Vec<Run>) {
    while let Some(index) = runs.iter().rposition(|r| !matches!(r, Run::Prelude(_))) {
        match &mut runs[index] {
            Run::Text(text, _) => {
                let trimmed = text.trim_end_matches(' ').len();
                text.truncate(trimmed);
                if text.is_empty() {
                    runs.remove(index);
                    continue;
                }
                return;
            }
            _ => return,
        }
    }
}
pub(super) fn emit(runs: &[Run]) -> String {
    let mut code = String::new();
    for run in runs {
        match run {
            Run::Text(text, span) => {
                code += &format!(
                    "__ui_rich=__ui_rich.styled({text:?},{});{}",
                    span.style,
                    span.color()
                )
            }
            Run::Break => {
                code += &format!(
                    "__ui_rich=__ui_rich.styled(\"\\n\",{R}::ui::styling::text::SpanStyle::new());"
                )
            }
            Run::Value(value, span) => {
                code += &format!(
                    "__ui_rich=__ui_rich.styled(::std::string::ToString::to_string(&({value})),{});{}",
                    span.style,
                    span.color()
                )
            }
            Run::Link(text, span, message) => {
                code += &format!(
                    "__ui_rich=__ui_rich.styled_link({text},{},{message});{}",
                    span.style,
                    span.color()
                )
            }
            Run::Prelude(p) => code += p,
            Run::If(branches, otherwise) => {
                for (i, (cond, body)) in branches.iter().enumerate() {
                    if i > 0 {
                        code += "else ";
                    }
                    code += &format!("if {cond}{{{}}}", emit(body));
                }
                if let Some(body) = otherwise {
                    code += &format!("else{{{}}}", emit(body));
                }
            }
            Run::Each(open, body, close) => code += &format!("{open}{{{}}}{close}", emit(body)),
        }
    }
    code
}
impl Generator {
    /// Portions de `node`, dans le style `style` (code d'un `SpanStyle`).
    pub(super) fn runs(&mut self, node: &Node, style: &str, out: &mut Vec<Run>) -> Result<()> {
        let span = Span {
            style: style.into(),
            color: String::new(),
        };
        self.spans(node, &span, out)
    }
    fn spans(&mut self, node: &Node, span: &Span, out: &mut Vec<Run>) -> Result<()> {
        let style = span.style.as_str();
        match node {
            Node::Text(t, _) => out.push(Run::Text(t.clone(), span.clone())),
            Node::Expr(t, _) => {
                let value = self.out.value(t);
                out.push(Run::Value(value, span.clone()));
            }
            Node::Element(e) => {
                let kind = kind_of(e)?;
                match kind {
                    Kind::LineBreak => {
                        if !e.attrs.is_empty() || !e.children.is_empty() {
                            return Err(e.loc.error("<br> takes no attributes or children"));
                        }
                        out.push(Run::Break)
                    }
                    Kind::Inline => {
                        for a in &e.attrs {
                            let ok = matches!(a.name.as_str(), "class" | "style")
                                || a.name.starts_with("class:")
                                || a.name.starts_with("style:")
                                || a.name.starts_with("data-")
                                || (e.name == "a" && a.name == "onclick")
                                || (e.name == "label" && a.name == "for")
                                || a.name == "hidden";
                            if !ok {
                                let hint = match a.name.as_str() {
                                    "href" => " Links emit messages: use onclick={Msg::Open}.",
                                    "onclick" => {
                                        " Use <a onclick={…}> for clickable text inside a paragraph."
                                    }
                                    "id" | "title" | "aria-label" => {
                                        " Put it on the paragraph or wrap the text in its own element."
                                    }
                                    _ => "",
                                };
                                return Err(a.loc.error(format!(
                                    "Attribute '{}' is not supported on inline <{}> inside text.{hint}",
                                    a.name, e.name
                                )));
                            }
                        }
                        let (child_style, styled) = self.span_style(e, style)?;
                        let child = if styled {
                            // Couleur lue dans des variables : la sienne, sinon celle du
                            // span englobant.
                            let n = self.next();
                            let inherited = if span.color.is_empty() {
                                "::core::option::Option::None".to_owned()
                            } else {
                                span.color.clone()
                            };
                            out.push(Run::Prelude(format!(
                                "let (__ui_span_{n},__ui_spancolor_{n})={child_style};let __ui_spancolor_{n}=__ui_spancolor_{n}.or({inherited});"
                            )));
                            Span {
                                style: format!("__ui_span_{n}"),
                                color: format!("__ui_spancolor_{n}"),
                            }
                        } else {
                            Span {
                                style: child_style,
                                color: span.color.clone(),
                            }
                        };
                        let hidden = match e.attr("hidden") {
                            Some(a) => Some(self.boolean(a)?),
                            None => None,
                        };
                        let mut inner = Vec::new();
                        if e.name == "a"
                            && let Some(click) = e.attr("onclick")
                        {
                            let message = self.message(click)?;
                            let text = self
                                .plain_text(&e.children.iter().collect::<Vec<_>>(), "link text")?;
                            inner.push(Run::Link(text, child, message));
                        } else {
                            for c in &e.children {
                                self.inline_only(c, &e.name)?;
                                self.spans(c, &child, &mut inner)?;
                            }
                        }
                        match hidden {
                            Some(cond) => {
                                out.push(Run::If(vec![(format!("!{cond}"), inner)], None))
                            }
                            None => out.extend(inner),
                        }
                    }
                    _ => {
                        return Err(e.loc.error(format!(
                            "<{}> cannot appear inside text: text elements accept inline content only",
                            e.name
                        )));
                    }
                }
            }
            Node::If {
                branches,
                otherwise,
                ..
            } => {
                let mut list = Vec::new();
                for (cond, body) in branches {
                    let cond = self.out.condition(cond);
                    let mut inner = Vec::new();
                    for n in body {
                        self.spans(n, span, &mut inner)?;
                    }
                    list.push((cond, inner));
                }
                let otherwise = match otherwise {
                    Some(body) => {
                        let mut inner = Vec::new();
                        for n in body {
                            self.spans(n, span, &mut inner)?;
                        }
                        Some(inner)
                    }
                    None => None,
                };
                out.push(Run::If(list, otherwise));
            }
            Node::Each(each) => {
                if each.otherwise.is_some() {
                    return Err(each
                        .loc
                        .error("{:else} in {#each} is not supported inside text"));
                }
                let (open, close) = self.each_header(each);
                let mut inner = Vec::new();
                for n in &each.body {
                    self.spans(n, span, &mut inner)?;
                }
                out.push(Run::Each(open, inner, close));
            }
            Node::Fragment { nodes, .. } => {
                for n in nodes {
                    self.spans(n, span, out)?;
                }
            }
            Node::Embed(_, loc) => {
                return Err(loc.error("{@element} cannot appear inside text"));
            }
            Node::Include { .. } => unreachable!("expanded"),
        }
        Ok(())
    }
}
