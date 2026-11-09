//! Conteneurs et mises en page : boîtes génériques, contexte flex/grille,
//! séparateur redimensionnable, liste virtuelle.
use super::{Dest, Generator, Item, R, inputs::radio_names, text::text_content, tree::flatten};
use crate::{
    Result,
    template::{
        html::Kind,
        markup::{Attr, Element, Node, Value},
    },
};
use proc_macro::{Span, TokenStream};

fn markup_relocate(tokens: TokenStream, span: Span) -> TokenStream {
    tokens
        .into_iter()
        .map(|mut t| {
            t.set_span(span);
            t
        })
        .collect()
}
impl Generator {
    /// Condition « conteneur flex ou grille » : `display` de `style="…"` (connu ici), sinon
    /// celui des classes littérales (constantes `__display_*` du module de styles, la
    /// dernière qui déclare `display` l'emporte). Les variantes conditionnelles
    /// (`@media`, `:hover`, `class:x`) ne comptent pas.
    fn flex_condition(&mut self, e: &Element) -> Result<Option<String>> {
        if let Some(Attr {
            value: Value::Text(style),
            ..
        }) = e.attr("style")
        {
            for declaration in style.split(';') {
                if let Some((name, value)) = declaration.split_once(':')
                    && name.trim() == "display"
                {
                    return Ok(Some(
                        matches!(
                            value.trim(),
                            "flex" | "inline-flex" | "grid" | "inline-grid"
                        )
                        .to_string(),
                    ));
                }
            }
        }
        let Some(Attr {
            value: Value::Text(names),
            loc,
            ..
        }) = e.attr("class")
        else {
            return Ok(None);
        };
        let module = self.module(*loc)?;
        let (id, count) = self.class_set(module, names, *loc);
        let displays = (0..count)
            .map(|i| format!("__ui_clsd_{id}_{i}"))
            .collect::<Vec<_>>();
        Ok((!displays.is_empty())
            .then(|| format!("{R}::ui::template::blockifies(&[{}])", displays.join(","))))
    }
    pub(super) fn container(&mut self, e: &Element, role: Option<&'static str>) -> Result<String> {
        let kind = Kind::Container(role);
        // Bouton à contenu riche : `disabled` désactive sa zone cliquable.
        let extra: &[&str] = if e.name == "button" {
            &["disabled", "memo"]
        } else {
            &["memo"]
        };
        self.check(e, kind, extra)?;
        let site = self.next();
        let id = self.id(e, site)?;
        // `memo={clé}` : sous-arbre repris de la vue précédente si la clé n'a pas changé.
        let memo = match e.attr("memo") {
            Some(Attr {
                value: Value::Expr(t),
                ..
            }) => Some(self.out.value(t)),
            Some(a) => {
                return Err(a.loc.error(
                    "memo expects a Rust value in braces covering what the subtree reads, e.g. memo={(&row, selected)}",
                ));
            }
            None => None,
        };
        let submit = match e.attr("onsubmit") {
            Some(a) if e.name == "form" => Some(self.message(a)?),
            Some(a) => return Err(a.loc.error("onsubmit applies to <form>")),
            None => None,
        };
        self.forms.push(submit);
        let flex = self.flex_condition(e)?;
        self.flex.push(flex);
        let capacity = self.capacity(&e.children);
        let children = self.children(
            &e.children,
            Dest {
                var: "__ui_children",
                slots: false,
            },
            None,
        );
        self.flex.pop();
        self.forms.pop();
        let children = children?;
        let stagger = self.stagger(e)?;
        let before = format!(
            "#[allow(unused_mut)] let mut __ui_children=::std::vec::Vec::<{R}::ui::element::Element>::with_capacity({capacity});{children}{stagger}"
        );
        let name = match e.attr("aria-label") {
            Some(a) => self.text_attr(a)?,
            None => "\"\"".into(),
        };
        let radios = radio_names(&e.children);
        let painter = if radios.len() > 1 {
            return Err(e.loc.error(format!(
                "Radio buttons named {} share this container; wrap each group in its own <div>",
                radios.join(", ")
            )));
        } else if let Some(group) = radios.first() {
            let label = if e.attr("aria-label").is_some() {
                name
            } else {
                format!("{group:?}")
            };
            format!(".painter({R}::ui::template::radio_group({label}))")
        } else if role.is_some() || e.attr("aria-label").is_some() {
            format!(
                ".painter({R}::ui::template::Semantic::new({R}::ui::element::Role::{},{name}))",
                role.unwrap_or("Group")
            )
        } else if e.attr("onclick").is_some()
            && let Some(text) = text_content(&e.children).filter(|t| !t.is_empty())
        {
            // Bouton riche (icône + libellé) : son texte devient son nom accessible.
            format!(
                ".painter({R}::ui::template::Semantic::new({R}::ui::element::Role::Group,{text:?}))"
            )
        } else {
            String::new()
        };
        let code = self.finish(
            e,
            kind,
            &id,
            &before,
            &format!(
                "{R}::ui::template::Node::element({R}::ui::element::Element::column(__ui_id.0,__ui_children)){painter}"
            ),
        )?;
        Ok(match memo {
            Some(key) => format!(
                "{{let __ui_memo_id={id};let __ui_memo={R}::ui::view::memo_key(&({key}));if {R}::ui::view::memo_reusable(__ui_memo_id,__ui_memo){{{R}::ui::view::memo_placeholder(__ui_memo_id,__ui_memo)}}else{{({code}).memo(__ui_memo)}}}}"
            ),
            None => code,
        })
    }
    pub(super) fn split(&mut self, e: &Element) -> Result<String> {
        self.check(
            e,
            Kind::Split,
            &["axis", "fraction", "min-first", "min-second", "resizable"],
        )?;
        let site = self.next();
        let id = self.id(e, site)?;
        let mut flat = Vec::new();
        flatten(&e.children, None, &mut flat);
        let items = self.group(&flat)?;
        if items.len() != 2 {
            return Err(e.loc.error("<k-split> needs exactly two child elements"));
        }
        let mut panes = Vec::new();
        for item in &items {
            panes.push(match item {
                Item::Block(Node::Element(c), _) => self.element(c)?,
                Item::Inline(group, _) => self.text_group(group)?,
                Item::Block(n, _) => {
                    return Err(n.loc().unwrap().error("<k-split> panes must be elements"));
                }
            });
        }
        let axis = match e.attr("axis").map(|a| &a.value) {
            None => "Horizontal",
            Some(Value::Text(t)) if t == "horizontal" || t == "row" => "Horizontal",
            Some(Value::Text(t)) if t == "vertical" || t == "column" => "Vertical",
            _ => return Err(e.loc.error("axis must be \"horizontal\" or \"vertical\"")),
        };
        let fraction = match e.attr("fraction") {
            Some(a) => self.number(a)?,
            None => "0.5f64".into(),
        };
        let first = match e.attr("min-first") {
            Some(a) => self.number(a)?,
            None => "0f64".into(),
        };
        let second = match e.attr("min-second") {
            Some(a) => self.number(a)?,
            None => "0f64".into(),
        };
        let resizable = match e.attr("resizable") {
            Some(a) => self.boolean(a)?,
            None => "true".into(),
        };
        self.finish(
            e,
            Kind::Split,
            &id,
            "",
            &format!(
                "{R}::ui::template::Node::element({R}::ui::element::Element::split(__ui_id.0,{R}::ui::element::Split{{axis:{R}::ui::element::Axis::{axis},fraction:{fraction} as f32,min_first:({first} as f32)*__ui_context.scale,min_second:({second} as f32)*__ui_context.scale,resizable:{resizable}}},{},{}))",
                panes[0], panes[1]
            ),
        )
    }
    pub(super) fn virtual_list(&mut self, e: &Element) -> Result<String> {
        self.check(e, Kind::VirtualList, &["items", "item", "row-height"])?;
        let site = self.next();
        let id = self.id(e, site)?;
        let items = match e.attr("items") {
            Some(Attr {
                value: Value::Expr(t),
                ..
            }) => self.out.value(t),
            _ => {
                return Err(e
                    .loc
                    .error("<k-virtual-list> needs items={owned collection} (e.g. Rc<Vec<T>>)"));
            }
        };
        let item = match e.attr("item") {
            Some(a) => {
                let name = Self::literal(a)?;
                if !name.chars().all(|c| c.is_alphanumeric() || c == '_') || name.is_empty() {
                    return Err(a.loc.error("item must be a Rust identifier"));
                }
                let tokens = name
                    .parse::<TokenStream>()
                    .map_err(|_| a.loc.error("item must be a Rust identifier"))?;
                self.out.value(&markup_relocate(tokens, a.loc.span))
            }
            None => {
                return Err(e
                    .loc
                    .error("<k-virtual-list> needs item=\"name\" for the row binding"));
            }
        };
        let height = match e.attr("row-height") {
            Some(a) => self.number(a)?,
            None => return Err(e.loc.error("<k-virtual-list> needs row-height=\"40\"")),
        };
        let mut flat = Vec::new();
        flatten(&e.children, None, &mut flat);
        let rows = self.group(&flat)?;
        let row = match rows.as_slice() {
            [Item::Block(Node::Element(c), _)] => self.element(c)?,
            [Item::Inline(group, _)] => self.text_group(group)?,
            _ => {
                return Err(e
                    .loc
                    .error("<k-virtual-list> needs exactly one row template element"));
            }
        };
        let row_site = self.next();
        self.finish(
            e,
            Kind::VirtualList,
            &id,
            "",
            &format!(
                "{R}::ui::template::Node::element({R}::ui::element::Element::virtual_list(__ui_id.0,{{let __ui_items={items};let __ui_scale=__ui_context.scale;let __ui_font=__ui_context.font;let __ui_rows=__ui_scope.item({row_site});{R}::ui::virtual_list::VirtualList::new(__ui_items.len(),({height} as f32)*__ui_scale,move|__ui_index|{{let __ui_context={R}::ui::template::TemplateContext::default().with_scale(__ui_scale).with_font(__ui_font);let __ui_scope={R}::ui::template::keyed(__ui_rows,__ui_index);let {item}=&__ui_items[__ui_index];{row}}})}}))"
            ),
        )
    }
}
