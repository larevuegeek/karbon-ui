//! `styles!` : recettes de styles nommées (`NOM { propriété: valeur; … }`) compilées
//! en fonctions de style typées, avec la même traduction des propriétés que les
//! templates (`codegen::property`).
use crate::{
    Result,
    codegen::{self, Output, StyleProp},
    input::{Cursor, Source},
};
use proc_macro::{Delimiter, TokenStream, TokenTree};
use std::collections::BTreeSet;
pub fn compile(root: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let source = Source::call(&input);
    let mut c = Cursor::new(input, &source);
    let mut out = Output::new(root);
    out.user_span = Some(c.span());
    let mut code = String::new();
    while !c.done() {
        let mut name = c.ident()?;
        let mut visibility = String::new();
        if name == "pub" {
            visibility = "pub ".into();
            if let Some(TokenTree::Group(g)) = c.tokens.get(c.at)
                && g.delimiter() == Delimiter::Parenthesis
            {
                visibility += &format!("{} ", g);
                c.at += 1;
            }
            name = c.ident()?;
        }
        let group = match c.next()? {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g,
            _ => return Err(c.error("Expected STYLE_NAME { property: value; }")),
        };
        let mut body = Cursor::new(group.stream(), &source);
        let mut seen = BTreeSet::new();
        let mut properties = String::new();
        while !body.done() {
            let span = body.span();
            let mut property = body.ident()?;
            while body.peek('-') {
                body.at += 1;
                property.push('_');
                property += &body.ident()?;
            }
            if !seen.insert(property.clone()) {
                return Err(body.error(&format!("Duplicate style property '{property}'")));
            }
            body.expect(':')?;
            let mut value = TokenStream::new();
            while !body.done() && !body.peek(';') {
                value.extend([body.next()?]);
            }
            body.expect(';')?;
            let mut iter = value.clone().into_iter();
            if let (Some(TokenTree::Group(g)), None) = (iter.next(), iter.next())
                && g.delimiter() == Delimiter::Brace
            {
                value = TokenStream::from(TokenTree::Group(proc_macro::Group::new(
                    Delimiter::Parenthesis,
                    g.stream(),
                )));
            }
            if value.is_empty() {
                return Err(body.error("A style value cannot be empty"));
            }
            if ["extends", "painter"].contains(&property.as_str()) {
                return Err(body.error("Use class={[BASE, OTHER]} to compose styles; painter belongs directly on a layout"));
            }
            properties += &codegen::property(
                &StyleProp {
                    name: property,
                    value,
                    span,
                    expression: false,
                },
                &mut out,
                "cx",
                "__ui_node",
            )?;
        }
        // La typographie est commune aux conteneurs et aux contrôles ; seules les
        // propriétés de mise en page demandent un conteneur.
        let bounds = if seen.contains("flex_direction")
            || seen.contains("columns")
            || seen.contains("grid_tracks")
        {
            "+__ui_root::ui::template::ContainerStyle"
        } else {
            ""
        };
        code += &format!(
            "#[allow(non_snake_case)] {visibility}fn {name}<R,W:__ui_root::ui::element::Widget+'static {bounds}>(__ui_node:__ui_root::ui::template::Node<W>,cx:&__ui_root::ui::template::TemplateContext<'_,R>)->__ui_root::ui::template::Node<W>{{{properties}__ui_node}}"
        );
    }
    out.finish(&code)
}
