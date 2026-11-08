//! Boutons : `<button>` et boutons générés (contenu en rangée, `::after`).
use super::{
    Generator, R,
    tree::{inline, is_whitespace},
};
use crate::{
    Result,
    template::{
        html::Kind,
        markup::{Element, Node},
    },
};
use proc_macro::TokenStream;

impl Generator {
    pub(super) fn button(&mut self, e: &Element) -> Result<String> {
        // Contenu riche (icône + texte, éléments) : zone cliquable de rôle bouton.
        if e.children.iter().any(|c| {
            matches!(c, Node::Element(_))
                || matches!(c, Node::If { .. } | Node::Each(_)) && !inline(c)
        }) {
            // `disabled` est permis (zone cliquable désactivée) ; `type` reste réservé au
            // bouton texte, seul à envoyer un formulaire.
            if let Some(a) = e.attr("type") {
                return Err(a.loc.error(
                    "A <button> with element content accepts onclick, disabled and common attributes; use type with a text-only label",
                ));
            }
            if e.attr("onclick").is_none() {
                return Err(e.loc.error("<button> needs onclick={Msg::Variant}"));
            }
            return self.container(e, None);
        }
        self.check(e, Kind::Button, &["disabled", "type"])?;
        let site = self.next();
        let id = self.id(e, site)?;
        let label = self.plain_text(&e.children.iter().collect::<Vec<_>>(), "a <button> label")?;
        // Bouton vide avec `aria-label` : le nom sert de libellé de repli, masqué quand la
        // feuille dessine une icône de fond (`.btn-close`).
        let mut name = String::new();
        let label = match e.attr("aria-label") {
            Some(a) if label == "\"\"" => {
                name = ".fallback_label(true)".into();
                self.text_attr(a)?
            }
            _ => label,
        };
        let kind = match e.attr("type") {
            Some(a) => Self::literal(a)?.to_owned(),
            None => "button".into(),
        };
        if !["button", "submit", "reset"].contains(&kind.as_str()) {
            return Err(e.loc.error("button type must be button, submit or reset"));
        }
        let disabled = self.disabled(e)?;
        let message = match e.attr("onclick") {
            Some(a) => self.message(a)?,
            None => match (kind.as_str(), self.forms.iter().rev().flatten().next()) {
                ("submit", Some(form)) => form.clone(),
                _ if e.attr("disabled").is_some() => "()".into(),
                _ => {
                    return Err(e.loc.error(
                        "<button> needs onclick={Msg::Variant} (or type=\"submit\" inside <form onsubmit={…}>)",
                    ));
                }
            },
        };
        self.last_element = None;
        let widget = self.finish(
            e,
            Kind::Button,
            &id,
            "",
            &format!(
                "{R}::ui::template::Node::widget(__ui_id,{{let mut __ui_widget={R}::ui::widgets::Button::new({label},{message}){name};__ui_widget.enabled=!{disabled};__ui_widget}})"
            ),
        )?;
        match self.last_element.take() {
            Some(signature) => Ok(self.generated_button(e, signature, widget)),
            None => Ok(widget),
        }
    }
    /// Bouton texte dont la feuille de framework génère une boîte `::before` / `::after`
    /// (`.dropdown-toggle`, `.accordion-button`) : conteneur cliquable portant le texte et
    /// la boîte générée, choisi à la compilation par la constante `__ui_elmg_<n>` de la
    /// feuille ; le widget bouton reste la forme par défaut.
    fn generated_button(&mut self, e: &Element, signature: usize, widget: String) -> String {
        if e.attr("disabled").is_some() || e.attr("onclick").is_none() || e.attr("class").is_none()
        {
            return widget;
        }
        let mut children = Vec::new();
        for child in &e.children {
            match child {
                Node::Text(t, loc) => children.push(Node::Text(t.clone(), *loc)),
                _ => return widget,
            }
        }
        if children.iter().all(is_whitespace) {
            return widget;
        }
        let synthetic = Element {
            name: e.name.clone(),
            path: TokenStream::new(),
            attrs: e
                .attrs
                .iter()
                .filter(|a| a.name != "type")
                .cloned()
                .collect(),
            children,
            loc: e.loc,
        };
        self.inline_row = Some(&synthetic as *const Element as usize);
        let container = self.container(&synthetic, None);
        self.inline_row = None;
        match container {
            Ok(container) => {
                format!("if __ui_elmg_{signature} {{{container}}} else {{{widget}}}")
            }
            Err(_) => widget,
        }
    }
}
