//! Réponses aux appels D-Bus : interfaces Accessible, Application, Component, Action et
//! Value, propriétés (`org.freedesktop.DBus.Properties`), introspection. Le texte des
//! champs est dans `text.rs`. Coordonnées : repère logique de la fenêtre (Wayland ne
//! donne pas la position des fenêtres à l'écran), pour tous les types de coordonnées.
use super::{
    Bridge, Host,
    tree::{self, NULL, Object, ROLE_APPLICATION, ROLE_FRAME, ROOT, state},
};
use crate::{
    platform::linux::ffi::dbus::{Message, Reader, Writer},
    ui::{
        element::ElementId,
        runtime::{AccessibilityAction, AccessibleElement},
        scene::RectF,
    },
};

const UNKNOWN_METHOD: &str = "org.freedesktop.DBus.Error.UnknownMethod";
const UNKNOWN_OBJECT: &str = "org.freedesktop.DBus.Error.UnknownObject";
/// Propriété inconnue, ou valeur refusée par le moteur (`Properties.Set`).
const INVALID_ARGS: &str = "org.freedesktop.DBus.Error.InvalidArgs";

fn rect(r: RectF) -> [i32; 4] {
    [
        r.left.floor() as i32,
        r.top.floor() as i32,
        (r.right - r.left).ceil().max(0.) as i32,
        (r.bottom - r.top).ceil().max(0.) as i32,
    ]
}
impl Bridge {
    fn reference(&self, object: Object) -> (String, String) {
        (self.name.clone(), object.path())
    }
    pub(super) fn element(&self, object: Object) -> Option<&AccessibleElement> {
        match object {
            Object::Element(w, e) => self.trees.get(&w)?.get(e),
            _ => None,
        }
    }
    /// Parent, enfants et index d'un objet.
    pub(super) fn parent(&self, object: Object) -> (String, String) {
        match object {
            Object::Root => self.desktop.clone(),
            Object::Window(_) => self.reference(Object::Root),
            Object::Element(w, e) => {
                let parent = self.trees.get(&w).and_then(|t| t.parent(e));
                self.reference(parent.map_or(Object::Window(w), |p| Object::Element(w, p)))
            }
        }
    }
    pub(super) fn children(&self, object: Object) -> Vec<Object> {
        match object {
            Object::Root => {
                // Ordre stable d'un appel à l'autre (`GetChildAtIndex`, `GetIndexInParent`),
                // que celui du `HashMap` ne garantit pas.
                let mut ids: Vec<_> = self.trees.keys().copied().collect();
                ids.sort_by_key(|w| w.0);
                ids.into_iter().map(Object::Window).collect()
            }
            Object::Window(w) | Object::Element(w, _) => {
                let parent = match object {
                    Object::Element(_, e) => Some(e),
                    _ => None,
                };
                self.trees.get(&w).map_or(Vec::new(), |t| {
                    t.children(parent)
                        .iter()
                        .map(|e| Object::Element(w, *e))
                        .collect()
                })
            }
        }
    }
    fn exists(&self, object: Object) -> bool {
        match object {
            Object::Root => true,
            Object::Window(w) => self.trees.contains_key(&w),
            Object::Element(..) => self.element(object).is_some(),
        }
    }
    pub(super) fn name(&self, object: Object) -> String {
        match object {
            Object::Root => self.app.clone(),
            Object::Window(w) => self
                .trees
                .get(&w)
                .map_or(String::new(), |t| t.title.clone()),
            Object::Element(..) => self
                .element(object)
                .map_or(String::new(), |e| e.name.clone()),
        }
    }
    fn role(&self, object: Object) -> (u32, &'static str) {
        match object {
            Object::Root => ROLE_APPLICATION,
            Object::Window(_) => ROLE_FRAME,
            Object::Element(..) => self.element(object).map_or((67, "unknown"), tree::role),
        }
    }
    pub(super) fn states(
        &self,
        object: Object,
        active: Option<crate::window::WindowId>,
    ) -> [u32; 2] {
        match object {
            Object::Root => tree::states(&[]),
            Object::Window(w) => {
                let mut bits = vec![
                    state::ENABLED,
                    state::SENSITIVE,
                    state::VISIBLE,
                    state::SHOWING,
                    state::RESIZABLE,
                ];
                if active == Some(w) {
                    bits.push(state::ACTIVE);
                }
                tree::states(&bits)
            }
            Object::Element(..) => self.element(object).map_or([0; 2], tree::element_states),
        }
    }
    /// Bornes d'un objet dans le repère de sa fenêtre.
    fn extents(&self, object: Object) -> [i32; 4] {
        match object {
            Object::Root => [0; 4],
            Object::Window(w) => self
                .trees
                .get(&w)
                .map_or([0; 4], |t| [0, 0, t.size.0, t.size.1]),
            Object::Element(..) => self
                .element(object)
                .map_or([0; 4], |e| rect(e.clipped_bounds)),
        }
    }
    fn interfaces(&self, object: Object) -> Vec<&'static str> {
        match object {
            Object::Root => vec!["org.a11y.atspi.Accessible", "org.a11y.atspi.Application"],
            Object::Window(_) => vec!["org.a11y.atspi.Accessible", "org.a11y.atspi.Component"],
            Object::Element(..) => self.element(object).map_or(Vec::new(), tree::interfaces),
        }
    }
    /// Objet le plus petit qui contient `(x, y)` parmi les descendants de `object`.
    fn at_point(&self, object: Object, x: i32, y: i32) -> Option<Object> {
        let (Object::Window(w) | Object::Element(w, _)) = object else {
            return None;
        };
        let tree = self.trees.get(&w)?;
        let inside = |e: &AccessibleElement| {
            let [l, t, width, height] = rect(e.clipped_bounds);
            !e.offscreen && x >= l && y >= t && x < l + width && y < t + height
        };
        let under = |mut id: ElementId| -> bool {
            let Object::Element(_, ancestor) = object else {
                return true;
            };
            while let Some(parent) = tree.parent(id) {
                if parent == ancestor {
                    return true;
                }
                id = parent;
            }
            false
        };
        tree.elements
            .iter()
            .filter(|e| inside(e) && under(e.id))
            .min_by(|a, b| {
                let area = |e: &AccessibleElement| {
                    let [.., w, h] = rect(e.clipped_bounds);
                    i64::from(w) * i64::from(h)
                };
                area(a).cmp(&area(b))
            })
            .map(|e| Object::Element(w, e.id))
    }
    /// Répond à un appel ; `None` si l'appelant n'attend pas de réponse.
    pub(super) fn call<H: Host>(
        &mut self,
        message: &Message,
        windows: &mut [H],
    ) -> Option<Message> {
        let path = message.path();
        let (interface, member) = (message.interface(), message.member());
        if path == "/org/a11y/atspi/cache" && interface == "org.a11y.atspi.Cache" {
            // Cache vide (signature d'at-spi2-core 2.52) : les clients interrogent alors
            // chaque objet.
            let mut reply = message.reply()?;
            reply
                .writer()
                .open(b'a', Some("((so)(so)(so)iiassusau)"))
                .close();
            return Some(reply);
        }
        let Some(object) = Object::parse(&path).filter(|o| self.exists(*o)) else {
            return message.error(UNKNOWN_OBJECT, &path);
        };
        let mut args = message.reader();
        let mut reply = message.reply()?;
        let active = windows.iter().find(|w| w.active()).map(|w| w.id());
        let handled = {
            let mut out = reply.writer();
            match interface.as_str() {
                "org.freedesktop.DBus.Properties" => {
                    self.properties_call(object, &member, &mut args, &mut out, windows)
                }
                "org.freedesktop.DBus.Introspectable" if member == "Introspect" => {
                    out.string(&introspection(&self.interfaces(object)));
                    true
                }
                "org.a11y.atspi.Accessible" => {
                    self.accessible(object, &member, &mut args, &mut out, active)
                }
                "org.a11y.atspi.Application" if member == "GetLocale" => {
                    out.string("");
                    true
                }
                "org.a11y.atspi.Component" => {
                    self.component(object, &member, &mut args, &mut out, windows)
                }
                "org.a11y.atspi.Action" => {
                    self.action(object, &member, &mut args, &mut out, windows)
                }
                "org.a11y.atspi.Text" | "org.a11y.atspi.EditableText" => {
                    let element = self.element(object).cloned();
                    match (element, object) {
                        (Some(e), Object::Element(w, _)) => super::text::call(
                            &e,
                            &interface,
                            &member,
                            &mut args,
                            &mut out,
                            act(windows, w),
                        ),
                        _ => false,
                    }
                }
                _ => false,
            }
        };
        if handled {
            Some(reply)
        } else {
            let name = if interface == "org.freedesktop.DBus.Properties" {
                INVALID_ARGS
            } else {
                UNKNOWN_METHOD
            };
            message.error(name, &format!("{interface}.{member}"))
        }
    }
    fn accessible(
        &self,
        object: Object,
        member: &str,
        args: &mut Reader<'_>,
        out: &mut Writer<'_>,
        active: Option<crate::window::WindowId>,
    ) -> bool {
        match member {
            "GetChildAtIndex" => {
                let index = args.i32().unwrap_or(-1);
                let child = usize::try_from(index)
                    .ok()
                    .and_then(|i| self.children(object).get(i).copied());
                match child {
                    Some(child) => out.reference(&self.name, &child.path()),
                    None => out.reference("", NULL),
                };
            }
            "GetChildren" => {
                out.open(b'a', Some("(so)"));
                for child in self.children(object) {
                    out.reference(&self.name, &child.path());
                }
                out.close();
            }
            "GetIndexInParent" => {
                let parent = Object::parse(&self.parent(object).1);
                let index = parent
                    .map(|p| self.children(p))
                    .and_then(|siblings| siblings.iter().position(|o| *o == object));
                out.i32(index.map_or(-1, |i| i as i32));
            }
            "GetRelationSet" => {
                out.open(b'a', Some("(ua(so))")).close();
            }
            "GetRole" => {
                out.u32(self.role(object).0);
            }
            "GetRoleName" | "GetLocalizedRoleName" => {
                out.string(self.role(object).1);
            }
            "GetState" => {
                let [low, high] = self.states(object, active);
                out.open(b'a', Some("u")).u32(low).u32(high).close();
            }
            "GetAttributes" => {
                out.open(b'a', Some("{ss}"));
                let attributes = match self.element(object) {
                    Some(e) => tree::attributes(e),
                    None => vec![("toolkit", "karbon-ui")],
                };
                for (key, value) in attributes {
                    out.open(b'e', None).string(key).string(value).close();
                }
                out.close();
            }
            "GetApplication" => {
                out.reference(&self.name, ROOT);
            }
            "GetInterfaces" => {
                out.open(b'a', Some("s"));
                for interface in self.interfaces(object) {
                    out.string(interface);
                }
                out.close();
            }
            _ => return false,
        }
        true
    }
    fn component<H: Host>(
        &self,
        object: Object,
        member: &str,
        args: &mut Reader<'_>,
        out: &mut Writer<'_>,
        windows: &mut [H],
    ) -> bool {
        let [x, y, w, h] = self.extents(object);
        match member {
            "GetExtents" => {
                out.open(b'r', None).i32(x).i32(y).i32(w).i32(h).close();
            }
            // Deux sorties séparées (`ii`), pas une structure : libatspi les lit ainsi.
            "GetPosition" => {
                out.i32(x).i32(y);
            }
            "GetSize" => {
                out.i32(w).i32(h);
            }
            "Contains" => {
                let (px, py) = (args.i32().unwrap_or(-1), args.i32().unwrap_or(-1));
                out.bool(px >= x && py >= y && px < x + w && py < y + h);
            }
            "GetAccessibleAtPoint" => {
                let (px, py) = (args.i32().unwrap_or(-1), args.i32().unwrap_or(-1));
                match self.at_point(object, px, py) {
                    Some(found) => out.reference(&self.name, &found.path()),
                    None => out.reference("", NULL),
                };
            }
            "GetLayer" => {
                out.u32(if matches!(object, Object::Window(_)) {
                    7
                } else {
                    3
                });
            }
            "GetMDIZOrder" => {
                out.i16(0);
            }
            "GetAlpha" => {
                out.f64(1.);
            }
            "GrabFocus" => {
                let done = match object {
                    Object::Element(w, id) => act(windows, w)(AccessibilityAction::Focus(id)),
                    _ => false,
                };
                out.bool(done);
            }
            "ScrollTo" | "ScrollToPoint" | "SetExtents" | "SetPosition" | "SetSize" => {
                out.bool(false);
            }
            _ => return false,
        }
        true
    }
    fn action<H: Host>(
        &self,
        object: Object,
        member: &str,
        args: &mut Reader<'_>,
        out: &mut Writer<'_>,
        windows: &mut [H],
    ) -> bool {
        let press = self.element(object).and_then(tree::press);
        let index = args.i32().unwrap_or(0);
        let name = press
            .as_ref()
            .filter(|_| index == 0)
            .map_or("", |(n, _)| *n);
        match member {
            "GetName" | "GetLocalizedName" => {
                out.string(name);
            }
            "GetDescription" | "GetKeyBinding" => {
                out.string("");
            }
            "GetActions" => {
                out.open(b'a', Some("(sss)"));
                if let Some((name, _)) = &press {
                    out.open(b'r', None)
                        .string(name)
                        .string("")
                        .string("")
                        .close();
                }
                out.close();
            }
            "DoAction" => {
                let done = match (press, object) {
                    (Some((_, action)), Object::Element(w, _)) if index == 0 => {
                        act(windows, w)(action)
                    }
                    _ => false,
                };
                out.bool(done);
            }
            _ => return false,
        }
        true
    }
}
/// Fonction qui applique une action à la fenêtre `id` ; `false` si elle échoue.
pub(super) fn act<H: Host>(
    windows: &mut [H],
    id: crate::window::WindowId,
) -> impl FnMut(AccessibilityAction) -> bool + '_ {
    move |action| {
        windows
            .iter_mut()
            .find(|w| w.id() == id)
            .is_some_and(|w| w.act(action).unwrap_or(false))
    }
}
/// Introspection minimale : les interfaces de l'objet.
fn introspection(interfaces: &[&str]) -> String {
    let mut xml = String::from(
        "<!DOCTYPE node PUBLIC \"-//freedesktop//DTD D-BUS Object Introspection 1.0//EN\" \
         \"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd\">\n<node>\n",
    );
    for interface in interfaces {
        xml.push_str(&format!("  <interface name=\"{interface}\"/>\n"));
    }
    xml.push_str("</node>\n");
    xml
}
