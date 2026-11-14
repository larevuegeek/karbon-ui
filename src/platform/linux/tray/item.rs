//! Objet `/StatusNotifierItem` (interface `org.kde.StatusNotifierItem`) : propriétés lues
//! par l'hôte (identité, titre, images, infobulle, chemin du menu) et clics.
use super::service::{Description, MENU_PATH, UNKNOWN_METHOD, properties_call, variant};
use crate::{
    platform::linux::ffi::dbus::{Message, Writer},
    window::TrayEvent,
};

pub(super) const INTERFACE: &str = "org.kde.StatusNotifierItem";

const NAMES: [&str; 15] = [
    "Category",
    "Id",
    "Title",
    "Status",
    "WindowId",
    "IconName",
    "IconPixmap",
    "OverlayIconName",
    "OverlayIconPixmap",
    "AttentionIconName",
    "AttentionIconPixmap",
    "AttentionMovieName",
    "ToolTip",
    "ItemIsMenu",
    "Menu",
];

pub(super) const INTROSPECTION: &str = "<interface name=\"org.kde.StatusNotifierItem\">\
    <property name=\"Category\" type=\"s\" access=\"read\"/>\
    <property name=\"Id\" type=\"s\" access=\"read\"/>\
    <property name=\"Title\" type=\"s\" access=\"read\"/>\
    <property name=\"Status\" type=\"s\" access=\"read\"/>\
    <property name=\"WindowId\" type=\"i\" access=\"read\"/>\
    <property name=\"IconName\" type=\"s\" access=\"read\"/>\
    <property name=\"IconPixmap\" type=\"a(iiay)\" access=\"read\"/>\
    <property name=\"OverlayIconName\" type=\"s\" access=\"read\"/>\
    <property name=\"OverlayIconPixmap\" type=\"a(iiay)\" access=\"read\"/>\
    <property name=\"AttentionIconName\" type=\"s\" access=\"read\"/>\
    <property name=\"AttentionIconPixmap\" type=\"a(iiay)\" access=\"read\"/>\
    <property name=\"AttentionMovieName\" type=\"s\" access=\"read\"/>\
    <property name=\"ToolTip\" type=\"(sa(iiay)ss)\" access=\"read\"/>\
    <property name=\"ItemIsMenu\" type=\"b\" access=\"read\"/>\
    <property name=\"Menu\" type=\"o\" access=\"read\"/>\
    <method name=\"ContextMenu\"><arg name=\"x\" type=\"i\" direction=\"in\"/><arg name=\"y\" type=\"i\" direction=\"in\"/></method>\
    <method name=\"Activate\"><arg name=\"x\" type=\"i\" direction=\"in\"/><arg name=\"y\" type=\"i\" direction=\"in\"/></method>\
    <method name=\"SecondaryActivate\"><arg name=\"x\" type=\"i\" direction=\"in\"/><arg name=\"y\" type=\"i\" direction=\"in\"/></method>\
    <method name=\"Scroll\"><arg name=\"delta\" type=\"i\" direction=\"in\"/><arg name=\"orientation\" type=\"s\" direction=\"in\"/></method>\
    <signal name=\"NewTitle\"/><signal name=\"NewIcon\"/><signal name=\"NewToolTip\"/>\
    </interface>\
    <interface name=\"com.canonical.dbusmenu\"/>";

/// Méthodes de l'interface : `Activate` (clic principal) remet `TrayEvent::Activated` ;
/// le menu passe par `com.canonical.dbusmenu` (propriété `Menu`), les autres méthodes
/// n'ont pas d'effet.
pub(super) fn call(message: &Message, notify: &dyn Fn(TrayEvent)) -> Option<Message> {
    match message.member().as_str() {
        "Activate" => notify(TrayEvent::Activated),
        "SecondaryActivate" | "ContextMenu" | "Scroll" | "ProvideXdgActivationToken" => {}
        _ => return message.error(UNKNOWN_METHOD, "unknown method"),
    }
    message.reply()
}

pub(super) fn properties(message: &Message, description: &Description) -> Option<Message> {
    properties_call(message, INTERFACE, &NAMES, |name, out| {
        write(name, description, out);
    })
}

/// Valeur de la propriété `name`, en variante.
fn write(name: &str, description: &Description, out: &mut Writer) {
    let text = |out: &mut Writer, value: &str| variant(out, "s", |w| _ = w.string(value));
    match name {
        "Category" => text(out, "ApplicationStatus"),
        "Id" => text(out, &description.id),
        "Title" => text(out, &description.title),
        "Status" => text(out, "Active"),
        "WindowId" => variant(out, "i", |w| _ = w.i32(0)),
        "IconPixmap" => variant(out, "a(iiay)", |w| pixmaps(w, description)),
        "OverlayIconPixmap" | "AttentionIconPixmap" => variant(out, "a(iiay)", |w| {
            w.open(b'a', Some("(iiay)")).close();
        }),
        // Infobulle : nom d'icône, images, titre, texte.
        "ToolTip" => variant(out, "(sa(iiay)ss)", |w| {
            w.open(b'r', None).string("");
            w.open(b'a', Some("(iiay)")).close();
            w.string(&description.title).string("").close();
        }),
        "ItemIsMenu" => variant(out, "b", |w| _ = w.bool(false)),
        "Menu" => variant(out, "o", |w| _ = w.path(MENU_PATH)),
        // IconName, OverlayIconName, AttentionIconName, AttentionMovieName : vides.
        _ => text(out, ""),
    }
}

/// `a(iiay)` : chaque taille de l'icône, ARGB dans l'ordre du réseau.
fn pixmaps(out: &mut Writer, description: &Description) {
    out.open(b'a', Some("(iiay)"));
    for pixmap in &description.pixmaps {
        out.open(b'r', None)
            .i32(pixmap.width)
            .i32(pixmap.height)
            .bytes(&pixmap.argb)
            .close();
    }
    out.close();
}
