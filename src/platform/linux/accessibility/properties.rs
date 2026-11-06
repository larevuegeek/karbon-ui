//! Propriétés D-Bus des objets (`org.freedesktop.DBus.Properties` : `Get`, `GetAll`,
//! `Set`) pour les interfaces Accessible, Application, Action, Value et Text.
use super::{
    Bridge, Host,
    interfaces::act,
    tree::{self, Object},
};
use crate::{
    platform::linux::ffi::dbus::{Reader, Writer},
    ui::runtime::AccessibilityAction,
};

/// Valeur d'une propriété.
pub(super) enum Prop {
    Str(String),
    Int(i32),
    F64(f64),
    Ref(String, String),
}
impl Prop {
    fn signature(&self) -> &'static str {
        match self {
            Self::Str(_) => "s",
            Self::Int(_) => "i",
            Self::F64(_) => "d",
            Self::Ref(..) => "(so)",
        }
    }
    pub fn write_variant(&self, w: &mut Writer<'_>) {
        w.open(b'v', Some(self.signature()));
        match self {
            Self::Str(s) => w.string(s),
            Self::Int(i) => w.i32(*i),
            Self::F64(d) => w.f64(*d),
            Self::Ref(bus, path) => w.reference(bus, path),
        };
        w.close();
    }
}
impl Bridge {
    /// Propriété `name` de l'interface `interface`.
    pub(super) fn property(&self, object: Object, interface: &str, name: &str) -> Option<Prop> {
        let e = self.element(object);
        Some(match (interface, name) {
            ("org.a11y.atspi.Accessible", "Name") => Prop::Str(self.name(object)),
            ("org.a11y.atspi.Accessible", "Description" | "HelpText") => {
                Prop::Str(e.map_or(String::new(), |e| e.help.clone()))
            }
            ("org.a11y.atspi.Accessible", "Parent") => {
                let (bus, path) = self.parent(object);
                Prop::Ref(bus, path)
            }
            ("org.a11y.atspi.Accessible", "ChildCount") => {
                Prop::Int(self.children(object).len() as i32)
            }
            ("org.a11y.atspi.Accessible", "Locale") => Prop::Str(String::new()),
            ("org.a11y.atspi.Accessible", "AccessibleId") => Prop::Str(match object {
                Object::Element(_, id) => id.0.to_string(),
                _ => String::new(),
            }),
            ("org.a11y.atspi.Application", "ToolkitName") => Prop::Str("karbon-ui".into()),
            ("org.a11y.atspi.Application", "Version") => {
                Prop::Str(env!("CARGO_PKG_VERSION").into())
            }
            ("org.a11y.atspi.Application", "AtspiVersion") => Prop::Str("2.1".into()),
            ("org.a11y.atspi.Application", "Id") => Prop::Int(0),
            ("org.a11y.atspi.Action", "NActions") => {
                Prop::Int(i32::from(e.and_then(tree::press).is_some()))
            }
            ("org.a11y.atspi.Value", _) => {
                let r = e?.range.as_ref()?;
                match name {
                    "MinimumValue" => Prop::F64(r.min),
                    "MaximumValue" => Prop::F64(r.max),
                    "MinimumIncrement" => Prop::F64(r.small_change),
                    "CurrentValue" => Prop::F64(r.value),
                    "Text" => Prop::Str(String::new()),
                    _ => return None,
                }
            }
            ("org.a11y.atspi.Text", _) => return super::text::property(e?, name),
            _ => return None,
        })
    }
    fn properties(interface: &str) -> &'static [&'static str] {
        match interface {
            "org.a11y.atspi.Accessible" => &[
                "Name",
                "Description",
                "Parent",
                "ChildCount",
                "Locale",
                "AccessibleId",
                "HelpText",
            ],
            "org.a11y.atspi.Application" => &["ToolkitName", "Version", "AtspiVersion", "Id"],
            "org.a11y.atspi.Action" => &["NActions"],
            "org.a11y.atspi.Value" => &[
                "MinimumValue",
                "MaximumValue",
                "MinimumIncrement",
                "CurrentValue",
                "Text",
            ],
            "org.a11y.atspi.Text" => &["CharacterCount", "CaretOffset"],
            _ => &[],
        }
    }
    pub(super) fn properties_call<H: Host>(
        &mut self,
        object: Object,
        member: &str,
        args: &mut Reader<'_>,
        out: &mut Writer<'_>,
        windows: &mut [H],
    ) -> bool {
        let interface = args.string().unwrap_or_default();
        match member {
            "Get" => {
                let name = args.string().unwrap_or_default();
                match self.property(object, &interface, &name) {
                    Some(value) => {
                        value.write_variant(out);
                        true
                    }
                    None => false,
                }
            }
            "GetAll" => {
                out.open(b'a', Some("{sv}"));
                for name in Self::properties(&interface) {
                    if let Some(value) = self.property(object, &interface, name) {
                        out.open(b'e', None).string(name);
                        value.write_variant(out);
                        out.close();
                    }
                }
                out.close();
                true
            }
            "Set" => {
                let name = args.string().unwrap_or_default();
                let value = args.enter().and_then(|mut v| v.f64());
                match (object, interface.as_str(), name.as_str(), value) {
                    (
                        Object::Element(w, id),
                        "org.a11y.atspi.Value",
                        "CurrentValue",
                        Some(value),
                        // Valeur refusée (hors bornes, NaN, curseur désactivé) : erreur
                        // `InvalidArgs`, le client ne croit pas la valeur appliquée.
                    ) => act(windows, w)(AccessibilityAction::SetNumber { id, value }),
                    _ => false,
                }
            }
            _ => false,
        }
    }
}
