//! Surcouches : `<dialog>` et `<k-popover>` (menus, infobulles riches).
use super::{Dest, Generator, R, text::static_text};
use crate::{
    Result, css,
    template::{
        html::Kind,
        markup::{Attr, Element, Node, Value},
    },
};

impl Generator {
    pub(super) fn dialog(&mut self, e: &Element) -> Result<String> {
        self.check(
            e,
            Kind::Dialog,
            &["open", "backdrop", "placement", "autofocus"],
        )?;
        let site = self.next();
        let id = self.id(e, site)?;
        let label = match e.attr("aria-label") {
            Some(a) => self.text_attr(a)?,
            None => {
                let heading = e.children.iter().find_map(|c| match c {
                    Node::Element(h) if h.name.len() == 2 && h.name.starts_with('h') => {
                        static_text(&h.children)
                    }
                    _ => None,
                });
                match heading {
                    Some(t) => format!("{t:?}"),
                    None => {
                        return Err(e.loc.error(
                            "<dialog> needs an accessible name: aria-label=\"…\" or a heading (<h2>…</h2>) as a direct child",
                        ));
                    }
                }
            }
        };
        let mut dismiss = match e.attr("onclose") {
            Some(a) => format!(".on_dismiss({})", self.message(a)?),
            None => String::new(),
        };
        // `backdrop="rgba(0,0,0,.5)"` : couleur constante du voile.
        if let Some(a) = e.attr("backdrop") {
            let text = Self::literal(a)?;
            let [r, g, b, alpha] = css::svg_color(text)
                .ok()
                .flatten()
                .ok_or_else(|| a.loc.error("backdrop expects a constant CSS colour"))?;
            let q = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
            dismiss += &format!(
                ".backdrop({R}::ui::theme::Color::rgba({},{},{},{}))",
                q(r),
                q(g),
                q(b),
                q(alpha)
            );
        }
        // `placement="top"` : contre le haut de la fenêtre (marges du contenu) ; `left` et
        // `right` : tiroir contre un bord, sur toute la hauteur.
        if let Some(a) = e.attr("placement") {
            dismiss += match Self::literal(a)? {
                "top" => ".placement(__ui_root::ui::modal::ModalPlacement::Top)",
                "left" => ".placement(__ui_root::ui::modal::ModalPlacement::Left)",
                "right" => ".placement(__ui_root::ui::modal::ModalPlacement::Right)",
                "center" => "",
                _ => {
                    return Err(a.loc.error("placement expects center, top, left or right"));
                }
            };
        }
        // `autofocus` sur le dialogue : le dialogue lui-même reçoit le focus.
        if e.attr("autofocus").is_some() {
            dismiss += ".initial_focus(__ui_id)";
        }
        self.flex.push(None);
        let capacity = self.capacity(&e.children);
        let children = self.children(
            &e.children,
            Dest {
                var: "__ui_children",
                slots: false,
            },
            None,
        )?;
        self.flex.pop();
        let stagger = self.stagger(e)?;
        let before = format!(
            "#[allow(unused_mut)] let mut __ui_children=::std::vec::Vec::<{R}::ui::element::Element>::with_capacity({capacity});{children}{stagger}"
        );
        self.finish(
            e,
            Kind::Dialog,
            &id,
            &before,
            &format!(
                "{R}::ui::template::Node::element({R}::ui::element::Element::modal(__ui_id.0,{R}::ui::modal::Modal::new({label}){dismiss},{R}::ui::element::Element::column(__ui_id.child(\"content\").0,__ui_children)))"
            ),
        )
    }
    pub(super) fn popover(&mut self, e: &Element) -> Result<String> {
        self.check(
            e,
            Kind::Popover,
            &[
                "anchor",
                "menu",
                "listbox",
                "combobox",
                "placement",
                "offset",
                "cross-offset",
                "match-width",
            ],
        )?;
        let site = self.next();
        let id = self.id(e, site)?;
        let anchor = match e.attr("anchor") {
            Some(Attr {
                value: Value::Text(t),
                ..
            }) => format!(
                "{R}::ui::popover::Anchor::Element({R}::ui::element::ElementId::named({t:?}))"
            ),
            Some(Attr {
                value: Value::Expr(t),
                ..
            }) => self.out.value(t),
            _ => {
                return Err(e
                    .loc
                    .error("<k-popover> needs anchor=\"element-id\" (or anchor={Anchor::…})"));
            }
        };
        let label = match e.attr("aria-label") {
            Some(a) => self.text_attr(a)?,
            None => return Err(e.loc.error("<k-popover> needs aria-label=\"…\"")),
        };
        let close = match e.attr("onclose") {
            Some(a) => self.message(a)?,
            None => return Err(e.loc.error("<k-popover> needs onclose={Msg::Close}")),
        };
        let kinds = [
            ("menu", ".menu()"),
            ("listbox", ".listbox()"),
            ("combobox", ".combobox()"),
        ]
        .into_iter()
        .filter_map(|(name, call)| e.attr(name).map(|a| (a, call)))
        .collect::<Vec<_>>();
        let menu = match kinds.as_slice() {
            [] => String::new(),
            [(_, call)] => (*call).to_owned(),
            [_, (a, _), ..] => {
                return Err(a
                    .loc
                    .error("<k-popover> is either a menu, a listbox or a combobox"));
            }
        };
        // placement="bottom-end", offset="8", cross-offset="16", match-width.
        let mut menu = menu;
        if ["placement", "offset", "cross-offset", "match-width"]
            .iter()
            .any(|n| e.attr(n).is_some())
        {
            let (side, align) = match e.attr("placement") {
                Some(a) => {
                    let value = Self::literal(a)?;
                    let (side, align) = value.split_once('-').unwrap_or((value, "start"));
                    let side = match side {
                        "top" => "Top",
                        "right" => "Right",
                        "bottom" => "Bottom",
                        "left" => "Left",
                        _ => {
                            return Err(a.loc.error(format!(
                                "placement=\"{value}\": expected top, right, bottom or left, optionally followed by -start, -center or -end"
                            )));
                        }
                    };
                    let align = match align {
                        "start" => "Start",
                        "center" => "Center",
                        "end" => "End",
                        _ => {
                            return Err(a.loc.error(format!(
                                "placement=\"{value}\": the alignment is start, center or end"
                            )));
                        }
                    };
                    (side, align)
                }
                None => ("Bottom", "Start"),
            };
            let offset = match e.attr("offset") {
                // `offset={expr}` : distance calculée en px (`f32`), bornée comme le littéral.
                Some(Attr {
                    value: Value::Expr(t),
                    ..
                }) => {
                    let value = self.out.value(t);
                    format!("(({value}) as f32).clamp(0.,256.)*__ui_context.scale")
                }
                Some(a) => {
                    let text = Self::literal(a)?;
                    let n = text
                        .trim()
                        .trim_end_matches("px")
                        .parse::<f32>()
                        .ok()
                        .filter(|n| n.is_finite() && *n >= 0. && *n <= 256.)
                        .ok_or_else(|| {
                            a.loc.error(format!(
                                "offset=\"{text}\" must be a distance in px between 0 and 256"
                            ))
                        })?;
                    format!("{n:?}f32*__ui_context.scale")
                }
                None => "4f32*__ui_context.scale".into(),
            };
            let cross = match e.attr("cross-offset") {
                Some(Attr {
                    value: Value::Expr(t),
                    ..
                }) => {
                    let value = self.out.value(t);
                    format!("(({value}) as f32).clamp(-256.,256.)*__ui_context.scale")
                }
                Some(a) => {
                    let text = Self::literal(a)?;
                    let n = text
                        .trim()
                        .trim_end_matches("px")
                        .parse::<f32>()
                        .ok()
                        .filter(|n| n.is_finite() && (-256. ..=256.).contains(n))
                        .ok_or_else(|| {
                            a.loc.error(format!(
                                "cross-offset=\"{text}\" must be a shift in px between -256 and 256"
                            ))
                        })?;
                    format!("{n:?}f32*__ui_context.scale")
                }
                None => "0f32".into(),
            };
            let matched = match e.attr("match-width") {
                Some(a) => self.boolean(a)?,
                None => "false".into(),
            };
            menu += &format!(
                ".placement({R}::ui::popover::PopoverPlacement{{side:{R}::ui::popover::Side::{side},align:{R}::ui::popover::SideAlign::{align},offset:{offset},match_anchor:{matched},cross_offset:{cross}}})"
            );
        }
        self.flex.push(None);
        let capacity = self.capacity(&e.children);
        let children = self.children(
            &e.children,
            Dest {
                var: "__ui_children",
                slots: false,
            },
            None,
        )?;
        self.flex.pop();
        let before = format!(
            "#[allow(unused_mut)] let mut __ui_children=::std::vec::Vec::<{R}::ui::element::Element>::with_capacity({capacity});{children}"
        );
        self.finish(
            e,
            Kind::Popover,
            &id,
            &before,
            &format!(
                "{R}::ui::template::Node::element({R}::ui::element::Element::popover(__ui_id.0,{R}::ui::popover::Popover::new({label},{anchor},{close}){menu},{R}::ui::element::Element::column(__ui_id.child(\"content\").0,__ui_children)))"
            ),
        )
    }
}
