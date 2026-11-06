//! Publication de l'arbre : snapshot reconstruit depuis le runtime (au plus dix fois
//! par seconde, focus immédiat) puis événements aux clients à l'écoute.
use super::super::ffi::scale;
use super::notify::raise_events;
use super::*;
use windows::Win32::UI::Accessibility::UiaClientsAreListening;
use windows_sys::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::{
        Input::KeyboardAndMouse::GetFocus,
        WindowsAndMessaging::{
            GetClientRect, GetWindowRect, IsIconic, IsWindowVisible, IsZoomed, KillTimer, SetTimer,
        },
    },
};

/// État de la fenêtre lors de la dernière publication : rien n'est republié sans changement.
#[derive(PartialEq)]
pub(super) struct Stamp {
    core: (u64, u64, u64, u64, Option<ElementId>),
    origin: Point,
    window: Rect,
    scale: u32,
    visible: bool,
    focused: bool,
    maximized: bool,
}
const SYNC_TIMER: usize = 0x5756;
/// Intervalle minimal entre deux reconstructions de l'arbre UI Automation.
const SYNC_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
/// Instant de la dernière publication et élément focalisé alors annoncé.
pub(crate) type Synced = Option<(std::time::Instant, Option<crate::ui::element::ElementId>)>;
static SYNCED: super::state::WindowLocal<std::cell::Cell<Synced>> =
    super::state::WindowLocal::new(|state| &state.accessibility_synced);
static PENDING: super::state::WindowLocal<std::cell::Cell<bool>> =
    super::state::WindowLocal::new(|state| &state.accessibility_pending);
/// Publie l'arbre aux clients UI Automation, au plus dix fois par seconde : le reconstruire
/// à chaque image (glisser un curseur, redimensionner) coûtait ~10 ms par image. Un
/// changement de focus part tout de suite (les lecteurs d'écran l'annoncent).
pub(crate) fn sync(window: Hwnd) {
    let Some(version) = runtime::accessibility_version() else {
        return;
    };
    let focus = version.4;
    let now = std::time::Instant::now();
    let last = SYNCED.with(std::cell::Cell::get);
    let throttled =
        state::sizing() || last.is_some_and(|(at, _)| now.duration_since(at) < SYNC_INTERVAL);
    if throttled && last.is_some_and(|(_, focused)| focused == focus) {
        if !PENDING.with(std::cell::Cell::get) {
            // SAFETY : sans callback, Windows publie WM_TIMER dans la file de la fenêtre.
            if unsafe { SetTimer(window, SYNC_TIMER, SYNC_INTERVAL.as_millis() as u32, None) } != 0
            {
                PENDING.with(|p| p.set(true));
            }
        }
        return;
    }
    SYNCED.with(|s| s.set(Some((now, focus))));
    sync_now(window, version);
}
/// Minuteur de [`sync`] : publie l'état différé.
pub(crate) fn timer_fired(window: Hwnd, id: usize) -> bool {
    if id != SYNC_TIMER {
        return false;
    }
    // SAFETY : arrêt d'un timer identifié de cette fenêtre.
    unsafe { KillTimer(window, SYNC_TIMER) };
    PENDING.with(|p| p.set(false));
    // Échéance atteinte : la prochaine synchronisation n'est plus différée.
    SYNCED.with(|s| s.set(None));
    sync(window);
    true
}
fn sync_now(window: Hwnd, version: (u64, u64, u64, u64, Option<crate::ui::element::ElementId>)) {
    let mut point = POINT { x: 0, y: 0 };
    let mut native = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: sorties locales valides pour la fenêtre du thread courant.
    let located = unsafe {
        ClientToScreen(window, &mut point) != 0 && GetWindowRect(window, &mut native) != 0
    };
    if !located {
        return;
    }
    let origin = Point {
        x: point.x,
        y: point.y,
    };
    let rect = Rect {
        left: native.left,
        top: native.top,
        right: native.right,
        bottom: native.bottom,
    };
    let scale = scale(window);
    // SAFETY: requêtes d'état en lecture seule sur la fenêtre du thread courant.
    let (visible, focused, maximized) = unsafe {
        (
            IsWindowVisible(window) != 0 && IsIconic(window) == 0,
            GetFocus() == window,
            IsZoomed(window) != 0,
        )
    };
    let stamp = Stamp {
        core: version,
        origin,
        window: rect,
        scale: scale.to_bits(),
        visible,
        focused,
        maximized,
    };
    let bridge = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        if state.last.as_ref() == Some(&stamp) {
            return None;
        }
        state.last = Some(stamp);
        Some(Arc::clone(&state.bridge))
    });
    let Some(bridge) = bridge else {
        return;
    };
    let Some(elements) = runtime::accessibility_snapshot() else {
        return;
    };
    let bounds = UiaRect {
        left: rect.left as f64,
        top: rect.top as f64,
        width: (rect.right - rect.left) as f64,
        height: (rect.bottom - rect.top) as f64,
    };
    let root = Node {
        key: NodeId::Root,
        parent: None,
        name: state::options().title.into(),
        help: String::new(),
        control_type: 50032,
        is_dialog: false,
        enabled: true,
        focusable: state::options().activate,
        focused: focused
            && !elements.iter().any(|n| {
                n.focused
                    || n.select
                        .as_ref()
                        .is_some_and(|s| s.options.iter().any(|o| o.focused))
            }),
        offscreen: !visible,
        invokable: false,
        value: None,
        text: None,
        choice: None,
        selection_container: None,
        range: None,
        expanded: None,
        read_only: true,
        bounds,
        clipped: bounds,
    };
    let mut nodes = vec![Arc::new(root)];
    let mut native = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: sortie locale valide pour la fenêtre du thread courant.
    unsafe { GetClientRect(window, &mut native) };
    let mut client = Rect {
        left: native.left,
        top: native.top,
        right: native.right,
        bottom: native.bottom,
    };
    client.right = (client.right as f32 / scale) as i32;
    client.bottom = (client.bottom as f32 / scale) as i32;
    use crate::ui::title_bar::{CaptionButton, button_rect};
    for (id, button, name) in [
        (0, CaptionButton::Minimize, "Réduire"),
        (
            1,
            CaptionButton::Maximize,
            if maximized { "Restaurer" } else { "Agrandir" },
        ),
        (2, CaptionButton::Close, "Fermer"),
    ]
    .into_iter()
    .filter(|_| state::options().title_bar)
    {
        let r = button_rect(&client, button);
        let r = physical(
            crate::ui::scene::RectF::new(
                r.left as f32,
                r.top as f32,
                (r.right - r.left) as f32,
                (r.bottom - r.top) as f32,
            ),
            origin,
            scale,
        );
        nodes.push(Arc::new(Node {
            key: NodeId::Caption(id),
            parent: Some(NodeId::Root),
            name: name.into(),
            help: String::new(),
            control_type: 50000,
            is_dialog: false,
            enabled: true,
            focusable: false,
            focused: false,
            offscreen: !visible,
            invokable: true,
            value: None,
            text: None,
            choice: None,
            selection_container: None,
            range: None,
            expanded: None,
            read_only: true,
            bounds: r,
            clipped: r,
        }));
    }
    let mut popup_nodes = Vec::new();
    for node in elements {
        let control_type = match node.role {
            Role::Text => 50020,
            Role::Button => 50000,
            Role::TextInput => 50004,
            Role::Checkbox => 50002,
            Role::Switch => 50000,
            Role::RadioButton => 50013,
            Role::RadioGroup => 50026,
            Role::Slider => 50015,
            Role::Select => 50003,
            Role::Dialog => 50033,
            Role::Popover => 50033,
            Role::Menu => 50009,
            Role::MenuItem => 50011,
            Role::Separator => 50038,
            Role::List => 50008,
            Role::Link => 50005,
            Role::Image => 50006,
            Role::ListItem => 50007,
            Role::Heading => 50020,
            Role::Group
            | Role::Main
            | Role::Navigation
            | Role::Banner
            | Role::ContentInfo
            | Role::Complementary => 50026,
            Role::ScrollArea | Role::Custom => 50033,
        };
        nodes.push(Arc::new(Node {
            key: NodeId::Element(node.id.0),
            parent: Some(node.parent.map_or(NodeId::Root, |id| NodeId::Element(id.0))),
            name: node.name,
            help: node.help,
            control_type,
            is_dialog: node.role == Role::Dialog,
            enabled: node.enabled,
            focusable: node.focusable && state::options().activate,
            focused: focused && node.focused,
            offscreen: !visible || node.offscreen,
            invokable: node.invokable,
            value: node.value,
            text: node
                .text
                .map(|t| Arc::new(text::TextData::physical(t, origin, scale))),
            choice: node.choice,
            selection_container: node.selection_container.map(|id| NodeId::Element(id.0)),
            range: node.range,
            expanded: node.select.as_ref().map(|s| s.expanded).or(node.expanded),
            read_only: node.read_only,
            bounds: physical(node.bounds, origin, scale),
            clipped: physical(node.clipped_bounds, origin, scale),
        }));
        if let Some(select) = node.select {
            expand::append_nodes(
                &mut popup_nodes,
                node.id,
                select,
                origin,
                scale,
                visible,
                focused,
            );
        }
    }
    nodes.extend(popup_nodes);
    if let Some(info) = runtime::tooltip() {
        tooltip::append(&mut nodes, info, origin, scale, visible);
    }
    let snapshot = Arc::new(Snapshot { nodes });
    let old = {
        let mut current = lock(&bridge.snapshot);
        std::mem::replace(&mut *current, Arc::clone(&snapshot))
    };
    // SAFETY: simple lecture de l'état global UIA, sans argument.
    if unsafe { UiaClientsAreListening() }.as_bool() {
        raise_events(&bridge, &old, &snapshot);
    }
    lock(&bridge.providers).retain(|key, cached| {
        cached.provider.upgrade().is_some()
            && snapshot
                .nodes
                .iter()
                .any(|n| n.key == *key && cached.capabilities == provider::Capabilities::of(n))
    });
}
