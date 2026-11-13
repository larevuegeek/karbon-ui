//! Connexion au compositeur : envoi des requêtes, répartiteur unique des événements (mis en
//! file, traités par la boucle hors de tout appel à libwayland) et objets globaux liés.
use super::protocol::{self, wl_registry};
use crate::platform::linux::ffi::{
    c_string,
    wayland::{
        Interface, wl_display_connect, wl_display_disconnect, wl_display_get_error,
        wl_display_roundtrip, wl_registry_interface,
    },
};
use std::ffi::c_void;

/// Objet du protocole (`wl_proxy`), possédé par la boucle.
pub(super) type Ptr = *mut c_void;
/// Emplacement d'un `new_id` dans les arguments d'une requête.
pub(super) const NEW: Ptr = std::ptr::null_mut();

/// Envoie une requête sur `proxy` (version de l'objet), avec ses arguments C dans
/// l'ordre du protocole.
macro_rules! request {
    ($proxy:expr, $opcode:expr $(, $arg:expr)* $(,)?) => {{
        let proxy: $crate::platform::linux::wayland::display::Ptr = $proxy;
        // SAFETY : objet vivant possédé par la boucle ; arguments conformes à la
        // signature du message (vérifiée par libwayland à l'envoi).
        let _ = unsafe {
            $crate::platform::linux::ffi::wayland::wl_proxy_marshal_flags(
                proxy,
                $opcode,
                std::ptr::null(),
                $crate::platform::linux::ffi::wayland::wl_proxy_get_version(proxy),
                0
                $(, $arg)*
            )
        };
    }};
}
/// Envoie une requête qui crée un objet d'interface `$interface` (même version que
/// `proxy`) et lui attache le répartiteur avec la nature `$kind`.
macro_rules! create {
    ($proxy:expr, $opcode:expr, $interface:expr, $kind:expr $(, $arg:expr)* $(,)?) => {{
        let proxy: $crate::platform::linux::wayland::display::Ptr = $proxy;
        // SAFETY : comme `request!` ; l'objet créé appartient à l'appelant.
        let object = unsafe {
            $crate::platform::linux::ffi::wayland::wl_proxy_marshal_flags(
                proxy,
                $opcode,
                $interface,
                $crate::platform::linux::ffi::wayland::wl_proxy_get_version(proxy),
                0
                $(, $arg)*
            )
        };
        $crate::platform::linux::wayland::display::listen(object, $kind);
        object
    }};
}
/// Envoie la requête destructrice `$opcode` de `proxy`, qui n'est plus utilisable ensuite.
macro_rules! destroy {
    ($proxy:expr, $opcode:expr $(, $arg:expr)* $(,)?) => {{
        let proxy: $crate::platform::linux::wayland::display::Ptr = $proxy;
        if !proxy.is_null() {
            // SAFETY : objet vivant, détruit par cette requête et oublié par l'appelant.
            unsafe {
                $crate::platform::linux::ffi::wayland::wl_proxy_marshal_flags(
                    proxy,
                    $opcode,
                    std::ptr::null(),
                    $crate::platform::linux::ffi::wayland::wl_proxy_get_version(proxy),
                    $crate::platform::linux::ffi::wayland::MARSHAL_DESTROY
                    $(, $arg)*
                )
            };
        }
    }};
}
pub(super) use {create, destroy, request};

pub(super) use super::dispatch::{Event, Kind, listen, take_events};
/// Objets globaux liés au démarrage.
pub(super) struct Globals {
    pub compositor: Ptr,
    pub shm: Ptr,
    pub wm_base: Ptr,
    /// Siège (clavier, souris) : nom global et objet.
    pub seat: Option<(u32, Ptr)>,
    /// Sorties : nom global, objet et échelle entière annoncée.
    pub outputs: Vec<(u32, Ptr, i32)>,
    pub data_manager: Ptr,
    pub viewporter: Ptr,
    /// Sous-surfaces (bordure de redimensionnement hors du contenu).
    pub subcompositor: Ptr,
    pub fractional: Ptr,
    pub decoration: Ptr,
    pub text_input: Ptr,
    /// Export des fenêtres (`zxdg_exporter_v2`, sinon `v1`) et objet exporté de cette
    /// version.
    pub exporter: Ptr,
    pub exported: &'static crate::platform::linux::ffi::wayland::Interface,
    /// Instants d'affichage (`wp_presentation`), liés seulement pendant la sonde.
    pub presentation: Ptr,
}
pub(super) struct Display {
    pub raw: Ptr,
    registry: Ptr,
    pub globals: Globals,
}
impl Display {
    /// Connexion à `WAYLAND_DISPLAY` et liaison des objets globaux.
    pub fn connect() -> crate::Result<Self> {
        // SAFETY : nom nul : variable d'environnement `WAYLAND_DISPLAY`.
        let raw = unsafe { wl_display_connect(std::ptr::null()) };
        if raw.is_null() {
            return Err(crate::Error::Unsupported {
                reason: "no Wayland compositor (WAYLAND_DISPLAY); without an X server (DISPLAY) either, no window can be opened",
            });
        }
        let registry = create!(
            raw,
            protocol::wl_display::GET_REGISTRY,
            &raw const wl_registry_interface,
            Kind::Registry,
            NEW
        );
        let mut display = Self {
            raw,
            registry,
            globals: Globals {
                compositor: std::ptr::null_mut(),
                shm: std::ptr::null_mut(),
                wm_base: std::ptr::null_mut(),
                seat: None,
                outputs: Vec::new(),
                data_manager: std::ptr::null_mut(),
                viewporter: std::ptr::null_mut(),
                subcompositor: std::ptr::null_mut(),
                fractional: std::ptr::null_mut(),
                decoration: std::ptr::null_mut(),
                text_input: std::ptr::null_mut(),
                exporter: std::ptr::null_mut(),
                exported: &protocol::ZXDG_EXPORTED_V2,
                presentation: std::ptr::null_mut(),
            },
        };
        display.roundtrip()?;
        let pending = take_events();
        for event in pending {
            display.global(&event);
        }
        let g = &display.globals;
        if g.compositor.is_null() || g.shm.is_null() || g.wm_base.is_null() {
            return Err(crate::Error::Unsupported {
                reason: "the Wayland compositor lacks wl_compositor 4, wl_shm or xdg_wm_base",
            });
        }
        Ok(display)
    }
    /// Aller-retour : toutes les requêtes envoyées ont été traitées par le compositeur.
    pub fn roundtrip(&self) -> crate::Result<()> {
        // SAFETY : connexion valide.
        if unsafe { wl_display_roundtrip(self.raw) } < 0 {
            return Err(self.error("wl_display_roundtrip"));
        }
        Ok(())
    }
    /// Erreur de la connexion (protocole ou compositeur perdu).
    pub fn error(&self, operation: &'static str) -> crate::Error {
        // SAFETY : connexion valide.
        let code = unsafe { wl_display_get_error(self.raw) };
        crate::Error::Platform { operation, code }
    }
    /// Lie un objet global annoncé par le registre (versions plafonnées à celles écrites).
    fn bind(
        registry: Ptr,
        name: u32,
        interface: &Interface,
        wanted: u32,
        offered: u32,
        kind: Kind,
    ) -> Ptr {
        let version = wanted.min(offered);
        // SAFETY : registre valide ; signature `usun` (nom, interface, version, objet).
        let object = unsafe {
            crate::platform::linux::ffi::wayland::wl_proxy_marshal_flags(
                registry,
                wl_registry::BIND,
                interface,
                version,
                0,
                name,
                interface.name(),
                version,
                NEW,
            )
        };
        listen(object, kind);
        object
    }
    /// Événement du registre : nouvel objet global, ou objet retiré.
    pub fn global(&mut self, event: &Event) {
        use crate::platform::linux::ffi::wayland as w;
        if event.kind != Kind::Registry {
            return;
        }
        if event.opcode == wl_registry::EV_GLOBAL_REMOVE {
            let name = event.uint(0);
            // Sortie débranchée : son objet est oublié (lié en version 2, sans `release`).
            // Les fenêtres l'ont déjà quittée (`wl_surface.leave`) ; un identifiant
            // inconnu qu'elles garderaient ne correspond plus à aucune sortie.
            if let Some(index) = self.globals.outputs.iter().position(|(n, ..)| *n == name) {
                let (_, output, _) = self.globals.outputs.remove(index);
                // SAFETY : objet vivant, oublié ici.
                unsafe { w::wl_proxy_destroy(output) };
            }
            return;
        }
        let (name, interface, version) = (event.uint(0), event.string(1), event.uint(2));
        let registry = self.registry;
        let bind = |interface: &Interface, wanted: u32, kind: Kind| {
            Self::bind(registry, name, interface, wanted, version, kind)
        };
        let g = &mut self.globals;
        // SAFETY : interfaces exportées par libwayland-client (adresses seulement).
        unsafe {
            match interface {
                "wl_compositor" if version >= 4 => {
                    g.compositor = bind(&w::wl_compositor_interface, 6, Kind::Silent);
                }
                "wl_shm" => g.shm = bind(&w::wl_shm_interface, 1, Kind::Silent),
                "xdg_wm_base" => {
                    g.wm_base = bind(&protocol::XDG_WM_BASE, 6, Kind::WmBase);
                }
                "wl_seat" if g.seat.is_none() => {
                    g.seat = Some((name, bind(&w::wl_seat_interface, 8, Kind::Seat)));
                }
                "wl_output" => {
                    let output = bind(&w::wl_output_interface, 2, Kind::Output);
                    g.outputs.push((name, output, 1));
                }
                "wl_data_device_manager" => {
                    g.data_manager = bind(&w::wl_data_device_manager_interface, 3, Kind::Silent);
                }
                "wl_subcompositor" => {
                    g.subcompositor = bind(&w::wl_subcompositor_interface, 1, Kind::Silent);
                }
                "wp_viewporter" => {
                    g.viewporter = bind(&protocol::WP_VIEWPORTER, 1, Kind::Silent);
                }
                "wp_fractional_scale_manager_v1" => {
                    g.fractional = bind(&protocol::WP_FRACTIONAL_SCALE_MANAGER_V1, 1, Kind::Silent);
                }
                "zxdg_decoration_manager_v1" => {
                    g.decoration = bind(&protocol::ZXDG_DECORATION_MANAGER_V1, 1, Kind::Silent);
                }
                // v2 préférée ; v1 seulement si la v2 n'est pas (encore) liée (une v1 liée
                // avant reste inutilisée, libérée avec la connexion).
                "zxdg_exporter_v2" => {
                    g.exporter = bind(&protocol::ZXDG_EXPORTER_V2, 1, Kind::Silent);
                    g.exported = &protocol::ZXDG_EXPORTED_V2;
                }
                "zxdg_exporter_v1" if g.exporter.is_null() => {
                    g.exporter = bind(&protocol::ZXDG_EXPORTER_V1, 1, Kind::Silent);
                    g.exported = &protocol::ZXDG_EXPORTED_V1;
                }
                "wp_presentation" if std::env::var_os("KARBON_SCROLL_PROBE").is_some() => {
                    g.presentation = bind(&protocol::WP_PRESENTATION, 1, Kind::Silent);
                }
                "zwp_text_input_manager_v3" => {
                    g.text_input = bind(&protocol::ZWP_TEXT_INPUT_MANAGER_V3, 1, Kind::Silent);
                }
                _ => {}
            }
        }
    }
}
impl Drop for Display {
    fn drop(&mut self) {
        // SAFETY : connexion possédée ; les objets restants sont libérés avec elle.
        unsafe { wl_display_disconnect(self.raw) };
    }
}
/// Chaîne C d'un titre ou d'un identifiant (octets nuls retirés).
pub(super) fn text(value: &str) -> std::ffi::CString {
    c_string(&value.replace('\0', "")).unwrap_or_default()
}
