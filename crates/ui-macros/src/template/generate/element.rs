//! Élément générique : choix du constructeur selon la balise, vérification des
//! attributs, identifiant, puis finition commune (styles, attributs, comportements).
use super::{
    Generator, R,
    inputs::{input_type, wraps_input},
};
use crate::{
    Result,
    codegen::{self, StyleProp},
    template::{
        html::{self, Kind},
        markup::{Attr, Element, Loc, Node, Value},
    },
};

pub(super) fn kind_of(e: &Element) -> Result<Kind> {
    html::kind(&e.name).map_err(|m| e.loc.error(m))
}
impl Generator {
    pub(super) fn check(&self, e: &Element, kind: Kind, extra: &[&str]) -> Result<()> {
        let input = if kind == Kind::Input {
            Some(input_type(e)?)
        } else {
            None
        };
        let events = html::events(kind, input.as_deref());
        for a in &e.attrs {
            let name = a.name.as_str();
            if html::EVENTS.contains(&name) {
                if !events.contains(&name) {
                    let tag = input.as_ref().map_or_else(
                        || format!("<{}>", e.name),
                        |t| format!("<input type=\"{t}\">"),
                    );
                    return Err(a.loc.error(format!(
                        "Event '{name}' does not apply to {tag}; valid events: {}",
                        events.join(", ")
                    )));
                }
                continue;
            }
            if html::COMMON.contains(&name)
                || extra.contains(&name)
                || name.starts_with("data-")
                || name.starts_with("class:")
                || name.starts_with("style:")
            {
                continue;
            }
            let mut known = html::COMMON.to_vec();
            known.extend(extra);
            let hint = match name {
                "href" => " Links emit messages: use onclick={Msg::Open}.".to_owned(),
                "on_click" | "on_change" | "on_input" => {
                    format!(" Write {} like HTML.", name.replace('_', ""))
                }
                _ => html::suggest(name, &known)
                    .map(|s| format!(" Did you mean '{s}'?"))
                    .unwrap_or_default(),
            };
            return Err(a
                .loc
                .error(format!("Unknown attribute '{name}' on <{}>.{hint}", e.name)));
        }
        if let Some(a) = e.attr("tabindex")
            && !matches!(&a.value, Value::Text(t) if t == "-1" || t == "0")
        {
            return Err(a
                .loc
                .error("Only tabindex=\"-1\" (skip in keyboard navigation) is supported"));
        }
        Ok(())
    }
    // -- Éléments ------------------------------------------------------------
    pub(super) fn id(&mut self, e: &Element, site: u64) -> Result<String> {
        Ok(match e.attr("id").map(|a| &a.value) {
            Some(Value::Text(s)) => format!("{R}::ui::element::ElementId::named({s:?})"),
            Some(Value::Expr(t)) => {
                format!("{R}::ui::element::ElementId::from({})", self.out.value(t))
            }
            Some(Value::Bool) => return Err(e.loc.error("id needs a value")),
            None => format!("__ui_scope.item({site})"),
        })
    }
    pub(super) fn element(&mut self, e: &Element) -> Result<String> {
        let kind = kind_of(e)?;
        // Faits de l'élément pour les sélecteurs de ses descendants.
        let facts = match kind {
            Kind::Component | Kind::Dialog | Kind::Popover | Kind::Slot => None,
            _ if self.styles.is_some() => Some(self.own_facts(e).0),
            _ => None,
        };
        self.ancestors.push(facts);
        let result = self.element_kind(e, kind);
        self.ancestors.pop();
        result
    }
    fn element_kind(&mut self, e: &Element, kind: Kind) -> Result<String> {
        match kind {
            Kind::Container(role) => self.container(e, role),
            Kind::TextBlock => self.text_block(e),
            Kind::Inline | Kind::LineBreak => {
                if e.name == "label" && wraps_input(e) {
                    return self.label_input(e);
                }
                let node = Node::Element(e.clone());
                self.text_group(&[&node])
            }
            Kind::Button => self.button(e),
            Kind::Input => self.input(e, None),
            Kind::TextArea => self.textarea(e),
            Kind::Select => self.select(e),
            Kind::Option => Err(e.loc.error("<option> belongs inside <select>")),
            Kind::Img => self.img(e),
            Kind::Hr => {
                self.check(e, kind, &[])?;
                if !e.children.is_empty() {
                    return Err(e.loc.error("<hr> cannot have children"));
                }
                let site = self.next();
                let id = self.id(e, site)?;
                self.finish(
                    e,
                    kind,
                    &id,
                    "",
                    &format!("{R}::ui::template::Node::widget(__ui_id,{R}::ui::template::Separator::new())"),
                )
            }
            Kind::Dialog => self.dialog(e),
            Kind::Split => self.split(e),
            Kind::Popover => self.popover(e),
            Kind::VirtualList => self.virtual_list(e),
            Kind::Svg => self.svg(e),
            Kind::Slot => Err(e
                .loc
                .error("<slot> must be placed among the children of an element")),
            Kind::Component => self.component(e),
        }
    }
    /// Pipeline commun : défauts, classes, styles, attributs, puis comportements.
    pub(super) fn finish(
        &mut self,
        e: &Element,
        kind: Kind,
        id: &str,
        before: &str,
        constructor: &str,
    ) -> Result<String> {
        let mut props = String::new();
        for (name, value) in html::defaults(&e.name) {
            props += &self.property(name, value, e.loc)?;
        }
        // Bouton converti en conteneur : contenu en ligne simulé par une rangée centrée,
        // sous les déclarations de la feuille.
        if self.inline_row == Some(e as *const Element as usize) {
            self.inline_row = None;
            for (name, value) in [
                ("flex_direction", "\"row\""),
                ("css_flex", "true"),
                ("align", "__ui_root::ui::element::Align::Center"),
                ("justify", "__ui_root::ui::element::Justify::Center"),
            ] {
                props += &self.property(name, value.into(), e.loc)?;
            }
        }
        props += &self.classes(e)?;
        props += &self.element_facts(e, kind)?;
        props += &self.styles(e)?;
        if let Some(title) = e.attr("title") {
            let value = self.text_attr(title)?;
            props += &format!("let __ui_node=__ui_node.tooltip({value});");
        }
        if let Some(click) = e.attr("onclick")
            && !matches!(kind, Kind::Button)
            && !(kind == Kind::Inline && e.name == "a")
        {
            let message = self.message(click)?;
            props += &format!("let __ui_node=__ui_node.on_click({message});");
            if e.name == "button" && e.attr("disabled").is_some() {
                let disabled = self.disabled(e)?;
                props += &format!("let __ui_node=__ui_node.disabled({disabled});");
            }
        } else if e.name == "label"
            && let Some(Attr {
                value: Value::Text(id),
                ..
            }) = e.attr("for")
            && let Some((checked, change)) = self.label_toggles.get(id).cloned()
        {
            // Libellé d'une case : le clic envoie la valeur inverse, comme la case.
            let checked = match &checked {
                Some(a) => self.boolean(a)?,
                None => "false".into(),
            };
            let change = self.message(&change)?;
            props += &format!("let __ui_node=__ui_node.on_click(({change})(!{checked}));");
        }
        if let Some(replay) = e.attr("data-replay") {
            let Value::Expr(t) = &replay.value else {
                return Err(replay
                    .loc
                    .error("data-replay expects a Rust value: data-replay={self.count}"));
            };
            let value = self.out.value(t);
            props += &format!(
                "let __ui_node=__ui_node.replay({R}::ui::template::replay_key(&({value})));"
            );
        }
        let mut post = String::new();
        if let Some(a) = e.attr("tabindex")
            && matches!(&a.value, Value::Text(t) if t == "-1")
        {
            post += "__ui_el=__ui_el.tab_stop(false);";
        }
        post += &self.aria_hidden(e)?;
        post += &self.aria_role(e)?;
        post += &self.drag_region(e)?;
        post += &self.resize_region(e)?;
        if let Some(a) = e.attr("draggable") {
            let value = match &a.value {
                Value::Expr(t) => self.out.value(t),
                _ => {
                    return Err(a.loc.error(
                        "draggable expects a drag source: draggable={(group, key)} or {DragSource::new(group, key)}",
                    ));
                }
            };
            post += &format!(
                "__ui_el=__ui_el.draggable(::core::convert::Into::<{R}::ui::drag_drop::DragSource>::into({value}));"
            );
        }
        if let Some(a) = e.attr("ondrop") {
            let handler = self.message(a)?;
            let key = match e.attr("dropzone").or(e.attr("draggable")) {
                Some(Attr {
                    value: Value::Expr(t),
                    ..
                }) => self.out.value(t),
                _ => {
                    return Err(a.loc.error(
                        "ondrop needs the target's group and key: dropzone={(group, key)} (or draggable on the same element)",
                    ));
                }
            };
            post += &format!(
                "__ui_el=__ui_el.drop_target({R}::ui::template::drop_target({key},{handler}));"
            );
        } else if let Some(a) = e.attr("dropzone") {
            return Err(a.loc.error("dropzone needs ondrop={Msg::Dropped}"));
        }
        if let Some(a) = e.attr("onscroll") {
            let handler = self.message(a)?;
            post += &format!("__ui_el=__ui_el.on_scroll({handler});");
        }
        if let Some(a) = e.attr("onwheel") {
            let handler = self.message(a)?;
            post += &format!("__ui_el=__ui_el.on_wheel({handler});");
        }
        post += &self.listeners(e)?;
        if let Some(a) = e.attr("data-exit") {
            post += &format!("__ui_el=__ui_el.exit_animation({});", self.exit(a)?);
        }
        if let Some(a) = e.attr("data-reveal")
            && let Some(transition) = self.reveal(a)?
        {
            post += &format!("__ui_el=__ui_el.reveal({transition});");
        }
        Ok(format!(
            "{{let __ui_id={id};{before}#[allow(unused_mut)] let __ui_node={constructor};{props}#[allow(unused_mut)] let mut __ui_el=__ui_node.build();{post}__ui_el}}"
        ))
    }
    fn property(&mut self, name: &str, value: String, loc: Loc) -> Result<String> {
        let attr = StyleProp {
            name: name.into(),
            value: self.out.finish(&value)?,
            span: loc.span,
            expression: false,
        };
        codegen::property(&attr, &mut self.out, "__ui_context", "__ui_node")
    }
}
