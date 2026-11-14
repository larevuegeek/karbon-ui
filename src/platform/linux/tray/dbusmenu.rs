//! Objet `/MenuBar` (interface `com.canonical.dbusmenu`, version 3) : le menu de l'icône,
//! affiché par l'hôte. Racine d'identifiant 0, entrées numérotées par leur rang + 1 ; un
//! clic (`Event` ou `EventGroup`, événement `clicked`) remet `TrayEvent::MenuItem` avec
//! l'identifiant choisi par l'application.
use super::service::{Description, UNKNOWN_METHOD, properties_call, variant};
use crate::{
    platform::linux::ffi::dbus::{Message, Reader, Writer},
    window::{TrayEvent, TrayMenuItem},
};

pub(super) const INTERFACE: &str = "com.canonical.dbusmenu";

pub(super) const INTROSPECTION: &str = "<interface name=\"com.canonical.dbusmenu\">\
    <property name=\"Version\" type=\"u\" access=\"read\"/>\
    <property name=\"TextDirection\" type=\"s\" access=\"read\"/>\
    <property name=\"Status\" type=\"s\" access=\"read\"/>\
    <property name=\"IconThemePath\" type=\"as\" access=\"read\"/>\
    <method name=\"GetLayout\"><arg type=\"i\" name=\"parentId\" direction=\"in\"/><arg type=\"i\" name=\"recursionDepth\" direction=\"in\"/><arg type=\"as\" name=\"propertyNames\" direction=\"in\"/><arg type=\"u\" name=\"revision\" direction=\"out\"/><arg type=\"(ia{sv}av)\" name=\"layout\" direction=\"out\"/></method>\
    <method name=\"GetGroupProperties\"><arg type=\"ai\" name=\"ids\" direction=\"in\"/><arg type=\"as\" name=\"propertyNames\" direction=\"in\"/><arg type=\"a(ia{sv})\" name=\"properties\" direction=\"out\"/></method>\
    <method name=\"GetProperty\"><arg type=\"i\" name=\"id\" direction=\"in\"/><arg type=\"s\" name=\"name\" direction=\"in\"/><arg type=\"v\" name=\"value\" direction=\"out\"/></method>\
    <method name=\"Event\"><arg type=\"i\" name=\"id\" direction=\"in\"/><arg type=\"s\" name=\"eventId\" direction=\"in\"/><arg type=\"v\" name=\"data\" direction=\"in\"/><arg type=\"u\" name=\"timestamp\" direction=\"in\"/></method>\
    <method name=\"EventGroup\"><arg type=\"a(isvu)\" name=\"events\" direction=\"in\"/><arg type=\"ai\" name=\"idErrors\" direction=\"out\"/></method>\
    <method name=\"AboutToShow\"><arg type=\"i\" name=\"id\" direction=\"in\"/><arg type=\"b\" name=\"needUpdate\" direction=\"out\"/></method>\
    <method name=\"AboutToShowGroup\"><arg type=\"ai\" name=\"ids\" direction=\"in\"/><arg type=\"ai\" name=\"updatesNeeded\" direction=\"out\"/><arg type=\"ai\" name=\"idErrors\" direction=\"out\"/></method>\
    <signal name=\"ItemsPropertiesUpdated\"><arg type=\"a(ia{sv})\"/><arg type=\"a(ias)\"/></signal>\
    <signal name=\"LayoutUpdated\"><arg type=\"u\"/><arg type=\"i\"/></signal>\
    <signal name=\"ItemActivationRequested\"><arg type=\"i\"/><arg type=\"u\"/></signal>\
    </interface>";

pub(super) fn call(
    message: &Message,
    description: &Description,
    revision: u32,
    notify: &dyn Fn(TrayEvent),
) -> Option<Message> {
    let menu = &description.menu;
    let mut args = message.reader();
    match message.member().as_str() {
        "GetLayout" => {
            let parent = args.i32().unwrap_or(0);
            let mut reply = message.reply()?;
            {
                let mut out = reply.writer();
                out.u32(revision);
                layout(&mut out, menu, parent);
            }
            Some(reply)
        }
        "GetGroupProperties" => {
            let ids = integers(&mut args);
            let mut reply = message.reply()?;
            {
                let mut out = reply.writer();
                out.open(b'a', Some("(ia{sv})"));
                for id in ids {
                    out.open(b'r', None).i32(id);
                    entry_properties(&mut out, menu, id);
                    out.close();
                }
                out.close();
            }
            Some(reply)
        }
        "GetProperty" => {
            let (id, name) = (args.i32().unwrap_or(-1), args.string().unwrap_or_default());
            let mut reply = message.reply()?;
            {
                let mut out = reply.writer();
                match (entry(menu, id), name.as_str()) {
                    (Some(TrayMenuItem::Item { label, .. }), "label") => {
                        variant(&mut out, "s", |w| _ = w.string(&mnemonic_free(label)));
                    }
                    (Some(TrayMenuItem::Item { enabled, .. }), "enabled") => {
                        variant(&mut out, "b", |w| _ = w.bool(*enabled));
                    }
                    (Some(TrayMenuItem::Separator), "type") => {
                        variant(&mut out, "s", |w| _ = w.string("separator"));
                    }
                    _ => variant(&mut out, "s", |w| _ = w.string("")),
                }
            }
            Some(reply)
        }
        "Event" => {
            let (id, event) = (args.i32().unwrap_or(-1), args.string().unwrap_or_default());
            clicked(menu, id, &event, notify);
            message.reply()
        }
        "EventGroup" => {
            if let Some(mut events) = args.enter() {
                while let Some(mut event) = events.enter() {
                    let (id, name) = (
                        event.i32().unwrap_or(-1),
                        event.string().unwrap_or_default(),
                    );
                    clicked(menu, id, &name, notify);
                }
            }
            let mut reply = message.reply()?;
            reply.writer().open(b'a', Some("i")).close();
            Some(reply)
        }
        "AboutToShow" => {
            let mut reply = message.reply()?;
            reply.writer().bool(false);
            Some(reply)
        }
        "AboutToShowGroup" => {
            let mut reply = message.reply()?;
            {
                let mut out = reply.writer();
                out.open(b'a', Some("i")).close();
                out.open(b'a', Some("i")).close();
            }
            Some(reply)
        }
        _ => message.error(UNKNOWN_METHOD, "unknown method"),
    }
}

pub(super) fn properties(message: &Message) -> Option<Message> {
    let names = ["Version", "TextDirection", "Status", "IconThemePath"];
    properties_call(message, INTERFACE, &names, |name, out| match name {
        "Version" => variant(out, "u", |w| _ = w.u32(3)),
        "IconThemePath" => variant(out, "as", |w| {
            w.open(b'a', Some("s")).close();
        }),
        "TextDirection" => variant(out, "s", |w| _ = w.string("ltr")),
        _ => variant(out, "s", |w| _ = w.string("normal")),
    })
}

/// Entrée d'identifiant `id` (rang + 1) ; `None` pour la racine ou un identifiant inconnu.
pub(super) fn entry(menu: &[TrayMenuItem], id: i32) -> Option<&TrayMenuItem> {
    menu.get(usize::try_from(id).ok()?.checked_sub(1)?)
}

/// `clicked` sur une entrée active : son identifiant d'application.
pub(super) fn clicked(menu: &[TrayMenuItem], id: i32, event: &str, notify: &dyn Fn(TrayEvent)) {
    if event == "clicked"
        && let Some(TrayMenuItem::Item {
            id, enabled: true, ..
        }) = entry(menu, id)
    {
        notify(TrayEvent::MenuItem(*id));
    }
}

/// `(ia{sv}av)` de l'entrée `parent` : la racine avec toutes les entrées, ou une entrée
/// seule.
fn layout(out: &mut Writer, menu: &[TrayMenuItem], parent: i32) {
    out.open(b'r', None).i32(parent);
    entry_properties(out, menu, parent);
    out.open(b'a', Some("v"));
    if parent == 0 {
        for index in 0..menu.len() {
            let id = index as i32 + 1;
            variant(out, "(ia{sv}av)", |w| {
                w.open(b'r', None).i32(id);
                entry_properties(w, menu, id);
                w.open(b'a', Some("v")).close();
                w.close();
            });
        }
    }
    out.close();
    out.close();
}

/// `a{sv}` de l'entrée `id` (racine : sous-menu).
fn entry_properties(out: &mut Writer, menu: &[TrayMenuItem], id: i32) {
    out.open(b'a', Some("{sv}"));
    let mut property = |name: &str, signature: &str, write: &dyn Fn(&mut Writer)| {
        out.open(b'e', None).string(name);
        variant(out, signature, write);
        out.close();
    };
    match entry(menu, id) {
        None if id == 0 => property("children-display", "s", &|w| _ = w.string("submenu")),
        Some(TrayMenuItem::Item { label, enabled, .. }) => {
            let label = mnemonic_free(label);
            property("label", "s", &|w| _ = w.string(&label));
            property("enabled", "b", &|w| _ = w.bool(*enabled));
            property("visible", "b", &|w| _ = w.bool(true));
        }
        Some(TrayMenuItem::Separator) => property("type", "s", &|w| _ = w.string("separator")),
        None => {}
    }
    out.close();
}

/// Texte affiché tel quel : `_` marque un raccourci clavier dans dbusmenu, il est doublé.
pub(super) fn mnemonic_free(label: &str) -> String {
    label.replace('_', "__")
}

/// Tableau `ai`.
fn integers(args: &mut Reader) -> Vec<i32> {
    let mut values = Vec::new();
    if let Some(mut array) = args.enter() {
        while let Some(value) = array.i32() {
            values.push(value);
        }
    }
    values
}
