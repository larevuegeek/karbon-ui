//! Pression mémoire du système : source libdispatch `DISPATCH_SOURCE_TYPE_MEMORYPRESSURE`
//! sur la file principale (liaisons écrites à la main, libdispatch fait partie de
//! libSystem). À l'avertissement, chaque fenêtre allège ses ressources comme après une
//! seconde sans image ; en pression critique, elle libère tout ce qu'une fenêtre réduite
//! dans le Dock libère. La prochaine image recrée ce qui lui manque.
use block2::RcBlock;
use std::ffi::c_void;

#[repr(C)]
struct DispatchSource {
    _private: [u8; 0],
}

/// `DISPATCH_MEMORYPRESSURE_WARN` et `DISPATCH_MEMORYPRESSURE_CRITICAL`.
const WARN: usize = 0x2;
const CRITICAL: usize = 0x4;

unsafe extern "C" {
    /// Type de source ; `DISPATCH_SOURCE_TYPE_MEMORYPRESSURE` en est l'adresse.
    static _dispatch_source_type_memorypressure: c_void;
    /// File principale ; `dispatch_get_main_queue()` en est l'adresse.
    static _dispatch_main_q: c_void;
    fn dispatch_source_create(
        kind: *const c_void,
        handle: usize,
        mask: usize,
        queue: *const c_void,
    ) -> *mut DispatchSource;
    fn dispatch_source_set_event_handler(
        source: *mut DispatchSource,
        handler: *mut block2::DynBlock<dyn Fn()>,
    );
    fn dispatch_source_get_data(source: *mut DispatchSource) -> usize;
    fn dispatch_source_cancel(source: *mut DispatchSource);
    fn dispatch_resume(object: *mut DispatchSource);
    fn dispatch_release(object: *mut DispatchSource);
}

/// Source active pendant la boucle d'événements ; annulée à sa destruction.
pub(super) struct MemoryPressure(*mut DispatchSource);

impl MemoryPressure {
    /// Écoute la pression mémoire ; `relieve(critical)` est appelé sur le thread
    /// principal, pendant l'attente des événements.
    pub fn watch(relieve: fn(bool)) -> Option<Self> {
        // SAFETY : symboles de libdispatch, valides pendant toute la vie du processus.
        let source = unsafe {
            dispatch_source_create(
                &raw const _dispatch_source_type_memorypressure,
                0,
                WARN | CRITICAL,
                &raw const _dispatch_main_q,
            )
        };
        if source.is_null() {
            return None;
        }
        // Pointeur gardé en entier : le bloc ne retient aucune donnée de Rust.
        let raw = source as usize;
        let handler = RcBlock::new(move || {
            // SAFETY : source vivante tant que son rappel peut être appelé (annulée et
            // libérée par `Drop`, après quoi libdispatch n'appelle plus le rappel).
            let level = unsafe { dispatch_source_get_data(raw as *mut DispatchSource) };
            relieve(level & CRITICAL != 0);
        });
        // SAFETY : source suspendue (créée ainsi) ; libdispatch copie le bloc.
        unsafe {
            dispatch_source_set_event_handler(source, RcBlock::as_ptr(&handler));
            dispatch_resume(source);
        }
        Some(Self(source))
    }
}

impl Drop for MemoryPressure {
    fn drop(&mut self) {
        // SAFETY : source créée par `watch`, annulée puis libérée une fois.
        unsafe {
            dispatch_source_cancel(self.0);
            dispatch_release(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_pressure_source_is_created_armed_and_cancelled() {
        fn relieve(_: bool) {}
        let watch = MemoryPressure::watch(relieve).expect("source libdispatch");
        assert!(!watch.0.is_null());
        drop(watch);
    }
}
