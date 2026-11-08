//! Pont partagé entre les callbacks COM (tout thread) et le thread UI : snapshot courant,
//! cache d'identité des fournisseurs et file de commandes envoyées à la fenêtre.
use super::*;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        MutexGuard,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use windows_sys::Win32::UI::WindowsAndMessaging::{SMTO_ABORTIFHUNG, SendMessageTimeoutW};

/// Verrou tolérant à l'empoisonnement : un callback COM paniqué ne bloque pas le pont.
pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}
pub(super) enum Command {
    Focus(NodeId),
    Invoke(NodeId),
    Toggle(NodeId),
    SelectChoice(NodeId),
    SetNumber(NodeId, f64),
    Expand(NodeId, bool),
    SetValue(NodeId, String),
    SelectText(NodeId, std::ops::Range<usize>, Arc<str>),
    RevealText(NodeId, usize),
}
pub(super) struct Request {
    pub(super) id: usize,
    pub(super) command: Command,
    pub(super) reply: Mutex<Option<HRESULT>>,
    pub(super) cancelled: AtomicBool,
}
/// Entrée du cache d'identité : référence faible COM et patterns exposés à la création.
pub(super) struct Cached {
    pub(super) provider: windows::core::Weak<IRawElementProviderSimple>,
    pub(super) capabilities: provider::Capabilities,
}
pub(super) struct Bridge {
    pub(super) window: usize,
    pub(super) snapshot: Mutex<Arc<Snapshot>>,
    pub(super) providers: Mutex<HashMap<NodeId, Cached>>,
    pub(super) requests: Mutex<VecDeque<Arc<Request>>>,
    next: AtomicUsize,
    closed: AtomicBool,
}
impl Bridge {
    pub(super) fn new(window: Hwnd) -> Arc<Self> {
        Arc::new(Self {
            window: window as usize,
            snapshot: Mutex::new(Arc::new(Snapshot::new(Vec::new()))),
            providers: Mutex::new(HashMap::new()),
            requests: Mutex::new(VecDeque::new()),
            next: AtomicUsize::new(1),
            closed: AtomicBool::new(false),
        })
    }
    pub(super) fn node(&self, key: NodeId) -> Result<Arc<Node>> {
        if self.closed.load(Ordering::Acquire) {
            return Err(fail(UNAVAILABLE));
        }
        lock(&self.snapshot).node(key)
    }
    /// Objet COM canonique du nœud (créé au besoin), converti vers l'interface `I`.
    pub(super) fn interface<I: Interface>(self: &Arc<Self>, key: NodeId) -> Result<I> {
        let node = self.node(key)?;
        let provider = {
            let mut providers = lock(&self.providers);
            match providers.get(&key).and_then(|c| c.provider.upgrade()) {
                Some(provider) => provider,
                None => {
                    let provider = provider::create(Arc::clone(self), &node);
                    providers.insert(
                        key,
                        Cached {
                            provider: provider.downgrade()?,
                            capabilities: provider::Capabilities::of(&node),
                        },
                    );
                    provider
                }
            }
        };
        provider.cast().map_err(|_| fail(NO_INTERFACE))
    }
    pub(super) fn dispatch(self: &Arc<Self>, command: Command) -> HRESULT {
        let key = match &command {
            Command::Focus(key)
            | Command::Invoke(key)
            | Command::Toggle(key)
            | Command::SelectChoice(key)
            | Command::SetNumber(key, _)
            | Command::Expand(key, _)
            | Command::SetValue(key, _)
            | Command::SelectText(key, _, _)
            | Command::RevealText(key, _) => *key,
        };
        let node = match self.node(key) {
            Ok(node) => node,
            Err(error) => return error.code(),
        };
        if !node.enabled {
            return DISABLED;
        }
        match &command {
            Command::Focus(_) if !node.focusable => return UNSUPPORTED,
            Command::Invoke(_) if !node.invokable => return UNSUPPORTED,
            Command::Expand(_, _) if node.expanded.is_none() => return UNSUPPORTED,
            Command::SetNumber(_, value) => {
                let Some(range) = node.range else {
                    return UNSUPPORTED;
                };
                if range.read_only {
                    return INVALID_OPERATION;
                }
                if !value.is_finite() || !(range.min..=range.max).contains(value) {
                    return INVALID_ARGUMENT;
                }
            }
            Command::Toggle(_) if !matches!(node.choice, Some(ChoiceState::Toggle(_))) => {
                return UNSUPPORTED;
            }
            Command::SelectChoice(_) if !matches!(node.choice, Some(ChoiceState::Radio(_))) => {
                return UNSUPPORTED;
            }
            Command::SetValue(_, _) if node.value.is_none() => return UNSUPPORTED,
            Command::SetValue(_, _) if node.read_only => return INVALID_OPERATION,
            Command::SelectText(_, _, _) | Command::RevealText(_, _) if node.text.is_none() => {
                return UNSUPPORTED;
            }
            _ => {}
        }
        let request = Arc::new(Request {
            id: self.next.fetch_add(1, Ordering::Relaxed),
            command,
            reply: Mutex::new(None),
            cancelled: AtomicBool::new(false),
        });
        {
            let mut requests = lock(&self.requests);
            if requests.len() >= 64 {
                return TIMEOUT;
            }
            requests.push_back(Arc::clone(&request));
        }
        let mut output = 0usize;
        // SAFETY: message privé sans pointeur ; la requête est retrouvée par identifiant.
        unsafe {
            SendMessageTimeoutW(
                self.window as Hwnd,
                COMMAND_MESSAGE,
                request.id,
                0,
                SMTO_ABORTIFHUNG,
                2000,
                &mut output,
            )
        };
        let result = lock(&request.reply).take();
        if let Some(result) = result {
            return result;
        }
        request.cancelled.store(true, Ordering::Release);
        lock(&self.requests).retain(|r| r.id != request.id);
        if self.closed.load(Ordering::Acquire) {
            UNAVAILABLE
        } else {
            TIMEOUT
        }
    }
    pub(super) fn close(&self) {
        self.closed.store(true, Ordering::Release);
        for request in lock(&self.requests).drain(..) {
            request.cancelled.store(true, Ordering::Release);
            *lock(&request.reply) = Some(UNAVAILABLE);
        }
        lock(&self.providers).clear();
        *lock(&self.snapshot) = Arc::new(Snapshot::new(Vec::new()));
    }
}
