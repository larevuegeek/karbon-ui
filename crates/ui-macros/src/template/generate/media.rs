//! Images et SVG : `<img>`, `<svg src=…>` lu depuis un fichier, SVG en ligne.
use super::{Generator, R, tree::is_whitespace};
use crate::{
    Result, css,
    input::Source,
    template::{
        html::Kind,
        markup::{self, Attr, Element, Node, Value},
    },
};

impl Generator {
    pub(super) fn img(&mut self, e: &Element) -> Result<String> {
        self.check(e, Kind::Img, &["src", "alt"])?;
        if !e.children.is_empty() {
            return Err(e.loc.error("<img> cannot have children"));
        }
        let site = self.next();
        let id = self.id(e, site)?;
        let src = match e.attr("src") {
            Some(Attr {
                value: Value::Expr(t),
                ..
            }) => self.out.value(t),
            Some(a) => {
                return Err(a.loc.error(
                    "img src must be a Rust image: src={Arc<Image>} (files are decoded by the application, not at runtime by the template)",
                ));
            }
            None => return Err(e.loc.error("<img> needs src={Arc<Image>}")),
        };
        let alt = match e.attr("alt").or(e.attr("aria-label")) {
            Some(a) => self.text_attr(a)?,
            None => "\"\"".into(),
        };
        self.finish(
            e,
            Kind::Img,
            &id,
            "",
            &format!(
                "{R}::ui::template::Node::widget(__ui_id,{R}::ui::template::ImageWidget::new({src}).alt({alt}))"
            ),
        )
    }
    /// `<svg src="icone.svg" …>` : le fichier (relatif au template) fournit le dessin et
    /// les attributs de sa racine ; ceux de la balise les remplacent.
    pub(super) fn svg_file(&mut self, e: Element, source: &Source) -> Result<Element> {
        let src = e.attr("src").unwrap();
        let path = Self::literal(src)?.to_owned();
        if !path.ends_with(".svg") {
            return Err(src
                .loc
                .error("svg src must name a .svg file, e.g. src=\"icons/play.svg\""));
        }
        if e.children.iter().any(|c| !is_whitespace(c)) {
            return Err(e.loc.error("<svg src=\"…\"> cannot have children"));
        }
        self.reader.source = source.clone();
        let (text, nested, _) = self
            .reader
            .text(&path, src.loc.span)
            .map_err(|err| src.loc.error(err.message))?;
        let parsed = markup::parse_text(
            crate::svg::strip_prolog(&text),
            &nested.display,
            src.loc.span,
        )?;
        let mut roots = parsed.into_iter().filter(|n| !is_whitespace(n));
        let (Some(Node::Element(mut file)), None) = (roots.next(), roots.next()) else {
            return Err(src
                .loc
                .error(format!("{path} must contain a single <svg> root element")));
        };
        if file.name != "svg" {
            return Err(src
                .loc
                .error(format!("{path} must contain a single <svg> root element")));
        }
        for a in e.attrs {
            if a.name == "src" {
                continue;
            }
            file.attrs.retain(|existing| existing.name != a.name);
            file.attrs.push(a);
        }
        file.loc = e.loc;
        Ok(file)
    }
    /// `<svg>` en ligne : dessin compilé en données statiques, `currentColor` lu dans la
    /// couleur CSS de l'élément ; `width`/`height` deviennent la taille par défaut.
    pub(super) fn svg(&mut self, e: &Element) -> Result<String> {
        let mut extra = crate::svg::ROOT_ATTRIBUTES.to_vec();
        extra.extend(crate::svg::PRESENTATION);
        self.check(e, Kind::Svg, &extra)?;
        let compiled = crate::svg::compile(e)?;
        let site = self.next();
        let id = self.id(e, site)?;
        let label = match e.attr("aria-label") {
            Some(a) => self.text_attr(a)?,
            None => format!("{:?}", compiled.title.clone().unwrap_or_default()),
        };
        let (w, h) = compiled.intrinsic;
        let size = css::inline(
            &format!("width:{};height:{}", compiled.width, compiled.height),
            &mut self.out,
            "__ui_context",
            "__ui_node",
        )
        .map_err(|m| e.loc.error(m))?;
        let image = compiled.image;
        // Sans nom (ni aria-label ni <title>), l'icône est décorative : absente de
        // l'arbre d'accessibilité, comme `aria-hidden="true"`.
        let decorative;
        let e = if e.attr("aria-label").is_none()
            && compiled.title.is_none()
            && e.attr("aria-hidden").is_none()
            && e.attr("role").is_none()
        {
            let mut copy = e.clone();
            copy.children.clear();
            copy.attrs.push(Attr {
                name: "aria-hidden".into(),
                value: Value::Text("true".into()),
                loc: e.loc,
            });
            decorative = copy;
            &decorative
        } else {
            e
        };
        self.finish(
            e,
            Kind::Svg,
            &id,
            "",
            &format!(
                "{{let __ui_node={R}::ui::template::Node::widget(__ui_id,{R}::ui::widgets::Svg::new({image},{w:?}f32*__ui_context.scale,{h:?}f32*__ui_context.scale).label({label}));{size}__ui_node}}"
            ),
        )
    }
}
