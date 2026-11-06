//! Élément accessible : objet `NSAccessibilityElement` d'un élément du moteur.
use super::super::window::accessible as tree;
use crate::{ui::element::ElementId, window::WindowId};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::AnyObject,
};
use objc2_app_kit::NSAccessibilityElement;
use objc2_foundation::{NSArray, NSCopying, NSNumber, NSObject, NSRect, NSString};

/// Fenêtre et élément représentés.
pub(in crate::platform::macos) struct Ids {
    pub window: WindowId,
    pub element: ElementId,
}
define_class!(
    // SAFETY : NSAccessibilityElement n'impose rien aux sous-classes ; pas de `Drop`.
    #[unsafe(super(NSAccessibilityElement, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KarbonUiAccessibilityNode"]
    #[ivars = Ids]
    pub(in crate::platform::macos) struct Node;

    impl Node {
        #[unsafe(method(isAccessibilityElement))]
        fn is_element(&self) -> bool {
            true
        }
        #[unsafe(method_id(accessibilityRole))]
        fn role(&self) -> Option<Retained<NSString>> {
            self.element().map(|e| super::element_role(&e).copy())
        }
        #[unsafe(method_id(accessibilitySubrole))]
        fn subrole(&self) -> Option<Retained<NSString>> {
            self.element().and_then(|e| super::subrole(e.role))
        }
        #[unsafe(method_id(accessibilityLabel))]
        fn label(&self) -> Option<Retained<NSString>> {
            self.element()
                .filter(|e| !e.name.is_empty())
                .map(|e| NSString::from_str(&e.name))
        }
        #[unsafe(method_id(accessibilityHelp))]
        fn help(&self) -> Option<Retained<NSString>> {
            self.element()
                .filter(|e| !e.help.is_empty())
                .map(|e| NSString::from_str(&e.help))
        }
        #[unsafe(method_id(accessibilityValue))]
        fn value(&self) -> Option<Retained<AnyObject>> {
            self.element().and_then(|e| super::value(&e))
        }
        #[unsafe(method_id(accessibilityMinValue))]
        fn min_value(&self) -> Option<Retained<AnyObject>> {
            self.element()
                .and_then(|e| e.range)
                .map(|r| super::object(NSNumber::new_f64(r.min)))
        }
        #[unsafe(method_id(accessibilityMaxValue))]
        fn max_value(&self) -> Option<Retained<AnyObject>> {
            self.element()
                .and_then(|e| e.range)
                .map(|r| super::object(NSNumber::new_f64(r.max)))
        }
        #[unsafe(method(isAccessibilityEnabled))]
        fn enabled(&self) -> bool {
            tree::element(self.ivars().window, self.ivars().element).is_some_and(|e| e.enabled)
        }
        #[unsafe(method(isAccessibilityFocused))]
        fn focused(&self) -> bool {
            tree::element(self.ivars().window, self.ivars().element).is_some_and(|e| e.focused)
        }
        #[unsafe(method(isAccessibilityExpanded))]
        fn expanded(&self) -> bool {
            tree::element(self.ivars().window, self.ivars().element)
                .is_some_and(|e| e.expanded == Some(true))
        }
        #[unsafe(method(setAccessibilityFocused:))]
        fn set_focused(&self, focused: bool) {
            if focused {
                let id = self.ivars().element;
                tree::act(
                    self.ivars().window,
                    crate::ui::runtime::AccessibilityAction::Focus(id),
                );
            }
        }
        #[unsafe(method(setAccessibilityValue:))]
        fn set_value(&self, value: Option<&AnyObject>) {
            let Some(text) = value.and_then(|v| v.downcast_ref::<NSString>()) else {
                return;
            };
            let id = self.ivars().element;
            tree::act(
                self.ivars().window,
                crate::ui::runtime::AccessibilityAction::SetValue {
                    id,
                    value: text.to_string(),
                },
            );
        }
        #[unsafe(method(accessibilityFrame))]
        fn frame(&self) -> NSRect {
            tree::element(self.ivars().window, self.ivars().element)
                .map_or(NSRect::ZERO, |e| tree::screen(self.ivars().window, e.clipped_bounds))
        }
        #[unsafe(method_id(accessibilityParent))]
        fn parent(&self) -> Option<Retained<AnyObject>> {
            tree::parent(self.ivars().window, self.ivars().element)
        }
        #[unsafe(method_id(accessibilityChildren))]
        fn children(&self) -> Option<Retained<NSArray<AnyObject>>> {
            tree::children(self.ivars().window, Some(self.ivars().element))
        }
        #[unsafe(method(accessibilityPerformPress))]
        fn perform_press(&self) -> bool {
            self.element()
                .and_then(|e| super::press(&e))
                .is_some_and(|action| tree::act(self.ivars().window, action))
        }
        #[unsafe(method(accessibilityPerformIncrement))]
        fn perform_increment(&self) -> bool {
            self.step(true)
        }
        #[unsafe(method(accessibilityPerformDecrement))]
        fn perform_decrement(&self) -> bool {
            self.step(false)
        }
    }
);
impl Node {
    pub(in crate::platform::macos) fn new(
        mtm: MainThreadMarker,
        window: WindowId,
        element: ElementId,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ids { window, element });
        // SAFETY : initialiseur de NSObject.
        unsafe { msg_send![super(this), init] }
    }
    fn element(&self) -> Option<crate::ui::runtime::AccessibleElement> {
        tree::element(self.ivars().window, self.ivars().element)
    }
    fn step(&self, up: bool) -> bool {
        self.element()
            .and_then(|e| super::step(&e, up))
            .is_some_and(|action| tree::act(self.ivars().window, action))
    }
}
