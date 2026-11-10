//! Classes Objective-C du backend : la vue de contenu (calque affichant l'image, entrées
//! souris et clavier) et le délégué de fenêtre (fermeture, taille, activation).
//!
//! Les méthodes ne font que transmettre à `window/event_loop.rs`, qui retrouve la session par
//! son identité.
use super::Pointer;
use crate::{
    ui::text_input::EditCommand,
    window::{WindowAction, WindowCommand, WindowId},
};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Sel},
};
use objc2_app_kit::{
    NSEvent, NSResponder, NSTextInputClient, NSTrackingArea, NSTrackingAreaOptions, NSView,
    NSWindowDelegate,
};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSNotFound, NSNotification, NSObject,
    NSObjectProtocol, NSPoint, NSRange, NSRangePointer, NSRect, NSUInteger,
};

/// Données de la vue : sa fenêtre et la transparence de son contenu.
pub(super) struct ViewState {
    id: WindowId,
    transparent: bool,
}

define_class!(
    // SAFETY : NSView n'impose rien aux sous-classes ; `View` n'implémente pas `Drop`.
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KarbonUiView"]
    #[ivars = ViewState]
    pub(super) struct View;

    impl View {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }
        // Un appui ne déplace jamais la fenêtre de lui-même (AppKit le ferait dans la
        // zone de la barre de titre pour une vue non opaque) : seules les zones
        // `data-drag-region` de l'application la déplacent, comme sous Windows.
        #[unsafe(method(mouseDownCanMoveWindow))]
        fn mouse_down_can_move_window(&self) -> bool {
            false
        }
        // Le contenu couvre toute la vue : AppKit n'a rien à composer dessous.
        // Contenu opaque : AppKit n'a rien à composer dessous (sauf fenêtre transparente).
        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool {
            !self.ivars().transparent
        }
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }
        #[unsafe(method(wantsUpdateLayer))]
        fn wants_update_layer(&self) -> bool {
            true
        }
        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {
            super::paint_now(self.ivars().id);
        }
        // Thème du système changé : l'image suit tout de suite, même sans autre événement.
        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn appearance_changed(&self) {
            super::resized(self.ivars().id);
        }
        // Conteneur accessible : ses éléments sont ceux du moteur (`accessible.rs`).
        #[unsafe(method_id(accessibilityChildren))]
        fn accessibility_children(&self) -> Option<Retained<NSArray<AnyObject>>> {
            super::accessible::children(self.ivars().id, None)
        }
        #[unsafe(method_id(accessibilityHitTest:))]
        fn accessibility_hit_test(&self, point: NSPoint) -> Option<Retained<AnyObject>> {
            super::accessible::hit(self.ivars().id, point).or_else(|| {
                // SAFETY : la vue est un objet Objective-C.
                Some(unsafe { Retained::cast_unchecked(self.retain()) })
            })
        }
        #[unsafe(method_id(accessibilityFocusedUIElement))]
        fn accessibility_focused(&self) -> Option<Retained<AnyObject>> {
            super::accessible::focused(self.ivars().id).or_else(|| {
                // SAFETY : la vue est un objet Objective-C.
                Some(unsafe { Retained::cast_unchecked(self.retain()) })
            })
        }
        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            self.pointer(Pointer::Move, event);
        }
        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.pointer(Pointer::Move, event);
        }
        #[unsafe(method(rightMouseDragged:))]
        fn right_mouse_dragged(&self, event: &NSEvent) {
            self.pointer(Pointer::Move, event);
        }
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            // Un clic valide la composition en cours, comme dans les champs AppKit.
            if super::text::marked_range(self.ivars().id).location != NSNotFound as usize {
                super::text::unmark(self.ivars().id);
                if let Some(context) = self.inputContext() {
                    context.discardMarkedText();
                }
            }
            self.pointer(Pointer::Down, event);
        }
        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            self.pointer(Pointer::Up, event);
        }
        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            self.pointer(Pointer::Context, event);
        }
        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, event: &NSEvent) {
            self.pointer(Pointer::Leave, event);
        }
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, event: &NSEvent) {
            self.pointer(Pointer::Move, event);
        }
        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            self.pointer(Pointer::Scroll, event);
        }
        // Actions de la barre de menus (`menu.rs`), remises au premier répondeur.
        #[unsafe(method(undo:))]
        fn undo(&self, _sender: Option<&AnyObject>) {
            super::edit(self.ivars().id, EditCommand::Undo);
        }
        #[unsafe(method(redo:))]
        fn redo(&self, _sender: Option<&AnyObject>) {
            super::edit(self.ivars().id, EditCommand::Redo);
        }
        #[unsafe(method(cut:))]
        fn cut(&self, _sender: Option<&AnyObject>) {
            super::edit(self.ivars().id, EditCommand::Cut);
        }
        #[unsafe(method(copy:))]
        fn copy(&self, _sender: Option<&AnyObject>) {
            super::edit(self.ivars().id, EditCommand::Copy);
        }
        #[unsafe(method(paste:))]
        fn paste(&self, _sender: Option<&AnyObject>) {
            super::edit(self.ivars().id, EditCommand::Paste);
        }
        #[unsafe(method(selectAll:))]
        fn select_all(&self, _sender: Option<&AnyObject>) {
            super::edit(self.ivars().id, EditCommand::SelectAll);
        }
        #[unsafe(method(karbonQuit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            super::command(WindowCommand::Exit);
        }
        #[unsafe(method(karbonMinimize:))]
        fn minimize(&self, _sender: Option<&AnyObject>) {
            super::command(WindowCommand::Action(self.ivars().id, WindowAction::Minimize));
        }
        #[unsafe(method(karbonZoom:))]
        fn zoom(&self, _sender: Option<&AnyObject>) {
            super::command(WindowCommand::Action(self.ivars().id, WindowAction::ToggleMaximize));
        }
        #[unsafe(method(karbonClose:))]
        fn close(&self, _sender: Option<&AnyObject>) {
            super::command(WindowCommand::Action(self.ivars().id, WindowAction::Close));
        }
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            if super::key(self.ivars().id, event, true) {
                // Texte, touche morte ou IME : AppKit rappelle `NSTextInputClient`.
                self.interpretKeyEvents(&NSArray::from_slice(&[event]));
            }
        }
        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) {
            super::key(self.ivars().id, event, false);
        }
    }

    unsafe impl NSObjectProtocol for View {}

    unsafe impl NSTextInputClient for View {
        #[unsafe(method(insertText:replacementRange:))]
        unsafe fn insert_text(&self, string: &AnyObject, replacement: NSRange) {
            super::text::insert(self.ivars().id, super::text::string(string), replacement);
        }
        #[unsafe(method(doCommandBySelector:))]
        unsafe fn do_command(&self, _selector: Sel) {
            // Les commandes (Entrée, flèches…) sont déjà traitées par `keyDown:` : rien
            // à faire, et surtout pas de bip.
        }
        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        unsafe fn set_marked_text(
            &self,
            string: &AnyObject,
            selected: NSRange,
            _replacement: NSRange,
        ) {
            super::text::set_marked(self.ivars().id, super::text::string(string), selected);
        }
        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            super::text::unmark(self.ivars().id);
        }
        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange {
            super::text::selected_range(self.ivars().id)
        }
        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange {
            super::text::marked_range(self.ivars().id)
        }
        #[unsafe(method(hasMarkedText))]
        fn has_marked_text(&self) -> bool {
            super::text::marked_range(self.ivars().id).location != NSNotFound as usize
        }
        #[unsafe(method_id(attributedSubstringForProposedRange:actualRange:))]
        unsafe fn attributed_substring(
            &self,
            _range: NSRange,
            _actual: NSRangePointer,
        ) -> Option<Retained<NSAttributedString>> {
            None
        }
        #[unsafe(method_id(validAttributesForMarkedText))]
        fn valid_attributes(&self) -> Retained<NSArray<NSAttributedStringKey>> {
            NSArray::new()
        }
        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        unsafe fn first_rect(&self, _range: NSRange, _actual: NSRangePointer) -> NSRect {
            // Fenêtre des candidats sous le caret, en coordonnées de l'écran.
            let Some(caret) = super::text::caret(self.ivars().id) else {
                return NSRect::ZERO;
            };
            let in_window = self.convertRect_toView(caret, None);
            match self.window() {
                Some(window) => window.convertRectToScreen(in_window),
                None => in_window,
            }
        }
        #[unsafe(method(characterIndexForPoint:))]
        fn character_index(&self, _point: NSPoint) -> NSUInteger {
            NSNotFound as NSUInteger
        }
    }
);
impl View {
    pub(super) fn new(mtm: MainThreadMarker, id: WindowId, transparent: bool) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ViewState { id, transparent });
        // SAFETY : initialiseur désigné de NSView.
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] };
        view.setWantsLayer(true);
        // Survol et sortie du pointeur dans toute la vue, fenêtre active ou non.
        let options = NSTrackingAreaOptions::MouseEnteredAndExited
            | NSTrackingAreaOptions::MouseMoved
            | NSTrackingAreaOptions::ActiveAlways
            | NSTrackingAreaOptions::InVisibleRect;
        // SAFETY : la vue possède la zone et en est la destinataire ; aucune donnée.
        let area = unsafe {
            NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                NSRect::ZERO,
                options,
                Some(&view),
                None,
            )
        };
        view.addTrackingArea(&area);
        view
    }
    fn pointer(&self, kind: Pointer, event: &NSEvent) {
        let p = self.convertPoint_fromView(event.locationInWindow(), None);
        let point = crate::ui::scene::PointF {
            x: p.x as f32,
            y: p.y as f32,
        };
        super::pointer(self.ivars().id, kind, point, event);
    }
}

define_class!(
    // SAFETY : NSObject n'impose rien ; `Delegate` n'implémente pas `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KarbonUiWindowDelegate"]
    #[ivars = WindowId]
    pub(super) struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl NSWindowDelegate for Delegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            super::closed(*self.ivars());
        }
        #[unsafe(method(windowDidResize:))]
        fn window_did_resize(&self, _notification: &NSNotification) {
            super::resized(*self.ivars());
        }
        #[unsafe(method(windowWillStartLiveResize:))]
        fn window_will_start_live_resize(&self, _notification: &NSNotification) {
            super::live_resize(*self.ivars(), true);
        }
        #[unsafe(method(windowDidEndLiveResize:))]
        fn window_did_end_live_resize(&self, _notification: &NSNotification) {
            super::live_resize(*self.ivars(), false);
        }
        #[unsafe(method(windowDidChangeBackingProperties:))]
        fn window_did_change_backing(&self, _notification: &NSNotification) {
            super::resized(*self.ivars());
        }
        #[unsafe(method(windowDidMiniaturize:))]
        fn window_did_miniaturize(&self, _notification: &NSNotification) {
            super::hidden(*self.ivars());
        }
        #[unsafe(method(windowDidDeminiaturize:))]
        fn window_did_deminiaturize(&self, _notification: &NSNotification) {
            super::resized(*self.ivars());
        }
        #[unsafe(method(windowDidChangeOcclusionState:))]
        fn window_did_change_occlusion(&self, _notification: &NSNotification) {
            super::occlusion(*self.ivars());
        }
        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _notification: &NSNotification) {
            super::focus(*self.ivars(), true);
        }
        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _notification: &NSNotification) {
            super::focus(*self.ivars(), false);
        }
    }
);
impl Delegate {
    pub(super) fn new(mtm: MainThreadMarker, id: WindowId) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(id);
        // SAFETY : initialiseur de NSObject.
        unsafe { msg_send![super(this), init] }
    }
}
