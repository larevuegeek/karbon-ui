//! Texte enrichi : suite de portions (texte, valeurs, liens, blocs) dont les blancs
//! sont regroupés comme en HTML avant d'émettre le code des segments.
use super::{Generator, R, element::kind_of};
use crate::{
    Result,
    template::{html::Kind, markup::Node},
};

/// Portion d'un texte enrichi, avant regroupement des blancs.
pub(super) enum Run {
    Text(String, String),
    Break,
    Value(String, String),
    Link(String, String, String),
    Prelude(String),
    If(Vec<(String, Vec<Run>)>, Option<Vec<Run>>),
    Each(String, Vec<Run>),
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
            Run::Each(_, body) => {
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
            Run::Text(text, style) => {
                code += &format!("__ui_rich=__ui_rich.styled({text:?},{style});")
            }
            Run::Break => {
                code += &format!(
                    "__ui_rich=__ui_rich.styled(\"\\n\",{R}::ui::styling::text::SpanStyle::new());"
                )
            }
            Run::Value(value, style) => {
                code += &format!(
                    "__ui_rich=__ui_rich.styled(::std::string::ToString::to_string(&({value})),{style});"
                )
            }
            Run::Link(text, style, message) => {
                code += &format!("__ui_rich=__ui_rich.styled_link({text},{style},{message});")
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
            Run::Each(header, body) => code += &format!("{header}{{{}}}", emit(body)),
        }
    }
    code
}
impl Generator {
    pub(super) fn runs(&mut self, node: &Node, style: &str, out: &mut Vec<Run>) -> Result<()> {
        match node {
            Node::Text(t, _) => out.push(Run::Text(t.clone(), style.into())),
            Node::Expr(t, _) => {
                let value = self.out.value(t);
                out.push(Run::Value(value, style.into()));
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
                        let child_style = self.span_style(e, style)?;
                        let prelude_var = if child_style.starts_with('{') {
                            let n = self.next();
                            out.push(Run::Prelude(format!("let __ui_span_{n}={child_style};")));
                            format!("__ui_span_{n}")
                        } else {
                            child_style
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
                            inner.push(Run::Link(text, prelude_var, message));
                        } else {
                            for child in &e.children {
                                self.inline_only(child, &e.name)?;
                                self.runs(child, &prelude_var, &mut inner)?;
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
                    let cond = self.out.value(cond);
                    let mut inner = Vec::new();
                    for n in body {
                        self.runs(n, style, &mut inner)?;
                    }
                    list.push((cond, inner));
                }
                let otherwise = match otherwise {
                    Some(body) => {
                        let mut inner = Vec::new();
                        for n in body {
                            self.runs(n, style, &mut inner)?;
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
                let header = self.each_header(each);
                let mut inner = Vec::new();
                for n in &each.body {
                    self.runs(n, style, &mut inner)?;
                }
                out.push(Run::Each(header, inner));
            }
            Node::Fragment { nodes, .. } => {
                for n in nodes {
                    self.runs(n, style, out)?;
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
