//! Icône d'une fenêtre dans la barre des menus (`NSStatusItem`), posée, remplacée ou
//! retirée par `WindowCommand::Tray`, retirée avec la session de sa fenêtre.
//!
//! Même comportement que sous Windows et Linux, pour qu'une application n'en écrive
//! qu'un : le clic principal remet `TrayEvent::Activated`, le clic secondaire (ou
//! Contrôle + clic) ouvre le menu, dont le choix remet `TrayEvent::MenuItem`. Le bouton
//! envoie son action aux deux relâchements (`sendActionOn:`) ; pour le clic secondaire,
//! le menu est attaché le temps d'un `performClick:` (AppKit l'ouvre sous l'icône), puis
//! détaché pour que le clic principal reste une action.
use crate::window::{TrayEvent, TrayIcon, TrayMenuItem, WindowId};
use objc2::{
    AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained, runtime::AnyObject, sel,
};
use objc2_app_kit::{
    NSApplication, NSEventMask, NSEventModifierFlags, NSEventType, NSImage, NSMenu, NSMenuItem,
    NSSquareStatusItemLength, NSStatusBar, NSStatusItem,
};
use objc2_foundation::{NSData, NSObject, NSObjectProtocol, NSSize, NSString};
use std::cell::RefCell;

/// Hauteur d'une icône de la barre des menus, en points (recommandation d'Apple).
const ICON_POINTS: f64 = 18.;

/// État de la cible : la fenêtre destinataire, l'icône et son menu.
pub(super) struct TargetState {
    id: WindowId,
    item: RefCell<Option<Retained<NSStatusItem>>>,
    menu: RefCell<Option<Retained<NSMenu>>>,
}

define_class!(
    // SAFETY : NSObject n'impose rien aux sous-classes ; pas de `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KarbonUiTrayTarget"]
    #[ivars = TargetState]
    pub(super) struct Target;

    unsafe impl NSObjectProtocol for Target {}

    impl Target {
        /// Relâchement d'un clic sur l'icône.
        #[unsafe(method(karbonTrayClick:))]
        fn click(&self, _sender: Option<&AnyObject>) {
            let secondary = NSApplication::sharedApplication(self.mtm())
                .currentEvent()
                .is_some_and(|event| {
                    matches!(event.r#type(), NSEventType::RightMouseUp | NSEventType::RightMouseDown)
                        || event.modifierFlags().contains(NSEventModifierFlags::Control)
                });
            if secondary {
                self.show_menu();
            } else {
                super::tray_event(self.ivars().id, TrayEvent::Activated);
            }
        }

        /// Choix d'une entrée du menu ; son étiquette (`tag`) est l'identifiant.
        #[unsafe(method(karbonTrayItem:))]
        fn item(&self, sender: Option<&NSMenuItem>) {
            if let Some(item) = sender
                && let Ok(id) = u32::try_from(item.tag())
            {
                super::tray_event(self.ivars().id, TrayEvent::MenuItem(id));
            }
        }
    }
);

impl Target {
    fn new(mtm: MainThreadMarker, id: WindowId) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TargetState {
            id,
            item: RefCell::new(None),
            menu: RefCell::new(None),
        });
        // SAFETY : initialiseur de NSObject.
        unsafe { msg_send![super(this), init] }
    }

    /// Ouvre le menu sous l'icône : attaché le temps du suivi, détaché ensuite.
    fn show_menu(&self) {
        let item = self.ivars().item.borrow().clone();
        let menu = self.ivars().menu.borrow().clone();
        let (Some(item), Some(menu)) = (item, menu) else {
            return;
        };
        item.setMenu(Some(&menu));
        if let Some(button) = item.button(self.mtm()) {
            // SAFETY : bouton vivant ; le suivi du menu est synchrone.
            unsafe { button.performClick(None) };
        }
        item.setMenu(None);
    }
}

/// Icône posée dans la barre des menus ; retirée quand la valeur est détruite.
pub(crate) struct StatusTray {
    item: Retained<NSStatusItem>,
    // La cible n'est pas retenue par AppKit (cibles faibles) : gardée ici.
    _target: Retained<Target>,
}
impl Drop for StatusTray {
    fn drop(&mut self) {
        NSStatusBar::systemStatusBar().removeStatusItem(&self.item);
    }
}

/// Image d'icône (barre des menus ou Dock) tirée d'octets encodés.
pub(super) fn image(bytes: &[u8]) -> crate::Result<Retained<NSImage>> {
    NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(bytes))
        .ok_or_else(|| crate::ImageError::Decode.into())
}

/// Pose l'icône de la fenêtre `id` décrite par `tray`. Une image illisible est une erreur.
pub(crate) fn create(
    id: WindowId,
    tray: &TrayIcon,
    mtm: MainThreadMarker,
) -> crate::Result<StatusTray> {
    let picture = image(&tray.image)?;
    picture.setSize(NSSize::new(ICON_POINTS, ICON_POINTS));
    picture.setTemplate(tray.template);
    let target = Target::new(mtm, id);
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSSquareStatusItemLength);
    if let Some(button) = item.button(mtm) {
        button.setImage(Some(&picture));
        if !tray.tooltip.is_empty() {
            button.setToolTip(Some(&NSString::from_str(&tray.tooltip)));
        }
        button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp);
        // SAFETY : la cible vit autant que l'icône (gardée par `StatusTray`) ; le sélecteur
        // est une méthode de la cible.
        unsafe {
            button.setTarget(Some(&target));
            button.setAction(Some(sel!(karbonTrayClick:)));
        }
    }
    if !tray.menu.is_empty() {
        *target.ivars().menu.borrow_mut() = Some(menu(&tray.menu, &target, mtm));
    }
    *target.ivars().item.borrow_mut() = Some(item.clone());
    Ok(StatusTray {
        item,
        _target: target,
    })
}

/// Menu de l'icône ; chaque entrée envoie `karbonTrayItem:` à la cible, avec son
/// identifiant pour étiquette.
fn menu(items: &[TrayMenuItem], target: &Target, mtm: MainThreadMarker) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);
    // Entrées grisées par `enabled`, pas par la chaîne des répondeurs.
    menu.setAutoenablesItems(false);
    for entry in items {
        match entry {
            TrayMenuItem::Item { id, label, enabled } => {
                // SAFETY : sélecteur d'une méthode de la cible ; équivalent clavier vide.
                let item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str(label),
                        Some(sel!(karbonTrayItem:)),
                        &NSString::from_str(""),
                    )
                };
                // SAFETY : la cible vit autant que le menu (gardée par `StatusTray`).
                unsafe { item.setTarget(Some(target)) };
                item.setTag(*id as isize);
                item.setEnabled(*enabled);
                menu.addItem(&item);
            }
            TrayMenuItem::Separator => menu.addItem(&NSMenuItem::separatorItem(mtm)),
        }
    }
    menu
}

/// `WindowOptions::icon` : icône du Dock quand le programme ne tourne pas depuis un paquet
/// `.app` (sans identifiant de paquet) ; dans un paquet, celle du paquet est gardée.
pub(super) fn dock_icon(bytes: &[u8], mtm: MainThreadMarker) {
    if objc2_foundation::NSBundle::mainBundle()
        .bundleIdentifier()
        .is_some()
    {
        return;
    }
    match image(bytes) {
        // SAFETY : image valide, retenue par NSApplication.
        Ok(picture) => unsafe {
            NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&picture));
        },
        Err(error) => eprintln!("karbon-ui: window icon unavailable ({error})"),
    }
}
