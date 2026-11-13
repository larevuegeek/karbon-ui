//! Gestes AppKit de la sonde : apparence imposée, parcours accessible, événements souris
//! et clavier remis à la fenêtre hors de tout rappel de l'application.
use crate::probe::deferred;
use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSWindow};
use objc2_foundation::{NSPoint, NSString};

/// Impose l'apparence sombre ou claire à `window`.
pub fn appearance(window: &NSWindow, dark: bool) {
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    };
    // SAFETY : constantes d'AppKit.
    let name = unsafe {
        if dark {
            NSAppearanceNameDarkAqua
        } else {
            NSAppearanceNameAqua
        }
    };
    window.setAppearance(NSAppearance::appearanceNamed(name).as_deref());
}
/// Éléments accessibles sous `node`, en profondeur.
pub fn collect(node: &AnyObject, out: &mut Vec<Retained<AnyObject>>) {
    // SAFETY : `accessibilityChildren` renvoie un NSArray ou nil.
    let children: Option<Retained<objc2_foundation::NSArray<AnyObject>>> =
        unsafe { msg_send![node, accessibilityChildren] };
    for child in children.map(|c| c.to_vec()).unwrap_or_default() {
        collect(&child, out);
        out.push(child);
    }
}
/// Texte d'un accesseur NSAccessibility (`NSString` ou rien).
pub fn ax_text(node: &AnyObject, selector: objc2::runtime::Sel) -> String {
    // SAFETY : accesseur sans argument renvoyant un objet ou nil.
    let value: Option<Retained<AnyObject>> = unsafe { msg_send![node, performSelector: selector] };
    value
        .and_then(|v| v.downcast::<NSString>().ok())
        .map(|s| s.to_string())
        .unwrap_or_default()
}
fn send(window: &NSWindow, event: Retained<NSEvent>) {
    let window = window.retain();
    deferred(move || window.sendEvent(&event));
}
fn point(window: &NSWindow, (x, y): (f32, f32)) -> NSPoint {
    let height = window.contentView().map_or(0., |v| v.bounds().size.height);
    NSPoint::new(f64::from(x), height - f64::from(y))
}
pub fn click(window: &NSWindow, at: (f32, f32)) {
    for kind in [NSEventType::LeftMouseDown, NSEventType::LeftMouseUp] {
        let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            kind,
            point(window, at),
            NSEventModifierFlags(0),
            0.,
            window.windowNumber(),
            None,
            0,
            1,
            1.,
        )
        .expect("événement souris");
        send(window, event);
    }
}
pub fn key(window: &NSWindow, characters: &str, code: u16) {
    for kind in [NSEventType::KeyDown, NSEventType::KeyUp] {
        let c = NSString::from_str(characters);
        let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            kind,
            NSPoint::new(0., 0.),
            NSEventModifierFlags(0),
            0.,
            window.windowNumber(),
            None,
            &c,
            &c,
            false,
            code,
        )
        .expect("événement clavier");
        send(window, event);
    }
}

/// Temps processeur consommé par le processus (utilisateur et système), en ms.
pub fn cpu_ms() -> u64 {
    // SAFETY : structure valide en écriture, processus courant.
    let usage = unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut usage);
        usage
    };
    let ms = |t: libc::timeval| t.tv_sec as u64 * 1000 + t.tv_usec as u64 / 1000;
    ms(usage.ru_utime) + ms(usage.ru_stime)
}

/// Boucle AppKit imbriquée pendant `seconds` (comme le suivi d'un menu) : elle consomme et
/// distribue tous les événements, réveils du moteur compris, puis poste un événement
/// anodin pour relancer la boucle principale.
pub fn nested_loop(seconds: f64) {
    let mtm = objc2::MainThreadMarker::new().expect("thread principal");
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    let until = objc2_foundation::NSDate::dateWithTimeIntervalSinceNow(seconds);
    // SAFETY : constante de Foundation.
    let mode = unsafe { objc2_foundation::NSDefaultRunLoopMode };
    while let Some(event) = app.nextEventMatchingMask_untilDate_inMode_dequeue(
        objc2_app_kit::NSEventMask::Any,
        Some(&until),
        mode,
        true,
    ) {
        app.sendEvent(&event);
    }
    let poke = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
        NSEventType::ApplicationDefined,
        NSPoint::new(0., 0.),
        NSEventModifierFlags(0),
        0.,
        0,
        None,
        1,
        0,
        0,
    )
    .expect("événement application");
    app.postEvent_atStart(&poke, false);
}
