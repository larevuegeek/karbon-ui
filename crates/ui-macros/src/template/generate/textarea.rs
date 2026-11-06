//! `<textarea>` : champ de saisie multiligne (`TextInput::multiline`), contrôlé comme
//! `<input type="text">` par `value` et `oninput`.
use super::{Generator, R};
use crate::{
    Result,
    template::{
        html::Kind,
        markup::{Element, Node, Value},
    },
};

impl Generator {
    pub(super) fn textarea(&mut self, e: &Element) -> Result<String> {
        self.check(
            e,
            Kind::TextArea,
            &[
                "value",
                "placeholder",
                "disabled",
                "readonly",
                "maxlength",
                "rows",
            ],
        )?;
        if e.children
            .iter()
            .any(|c| !matches!(c, Node::Text(t, _) if t.trim().is_empty()))
        {
            return Err(e
                .loc
                .error("<textarea> takes its text from value={…}, not from its content"));
        }
        let site = self.next();
        let id = self.id(e, site)?;
        let disabled = self.disabled(e)?;
        let aria = match e.attr("aria-label") {
            Some(a) => Some(self.text_attr(a)?),
            None => None,
        };
        let for_label = match e.attr("id").map(|a| &a.value) {
            Some(Value::Text(id)) => self.labels.get(id).map(|t| format!("{t:?}")),
            _ => None,
        };
        let value = match e.attr("value") {
            Some(a) => self.text_attr(a)?,
            None => "\"\"".into(),
        };
        let readonly = match e.attr("readonly") {
            Some(a) => self.boolean(a)?,
            None => "false".into(),
        };
        let change = match e.attr("oninput") {
            Some(a) => self.message(a)?,
            None if e.attr("readonly").is_some() || e.attr("disabled").is_some() => {
                "|_: ::std::string::String|()".into()
            }
            None => {
                return Err(e.loc.error(
                    "<textarea> needs oninput={Msg::Variant} (a Variant(String) message), or readonly",
                ));
            }
        };
        let mut widget = format!("{R}::ui::widgets::TextInput::new({value},{change}).multiline()");
        if let Some(a) = e.attr("placeholder") {
            widget += &format!(".placeholder({})", self.text_attr(a)?);
        }
        if let Some(label) = aria.or(for_label) {
            widget += &format!(".label({label})");
        }
        if let Some(a) = e.attr("maxlength") {
            widget += &format!(".max_chars(({}) as usize)", self.number(a)?);
        }
        if let Some(a) = e.attr("rows") {
            widget += &format!(".rows(({}) as u16)", self.number(a)?);
        }
        if let Some(a) = e.attr("onkeydown") {
            widget += &format!(".on_key({})", self.message(a)?);
        }
        let constructor = format!(
            "{R}::ui::template::Node::widget(__ui_id,{{let mut __ui_widget={widget};__ui_widget.enabled=!{disabled};__ui_widget.read_only={readonly};__ui_widget}})"
        );
        self.finish(e, Kind::TextArea, &id, "", &constructor)
    }
}
