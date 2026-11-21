//! Couche DirectComposition des surfaces partagées (`ui::shared_surface`) : l'image d'un
//! autre renderer montrée sous l'interface, à la place d'un élément.
//!
//! Une fenêtre a deux cibles de composition : celle de l'interface (`composition.rs`,
//! au-dessus de tout) et, quand une surface partagée y est peinte, celle-ci, créée
//! derrière (`CreateTargetForHwnd(…, topmost = false)`). L'interface troue son image à la
//! place de l'élément (`graphics/surface_hole.rs`) : la surface se voit par le trou, et
//! tout ce que l'interface peint ensuite (menus, bulles) reste par-dessus. Rien de la
//! couche de l'interface n'est changé.
//!
//! Chaque surface a un visuel, dont le contenu est sa chaîne d'échange : une image neuve
//! du producteur ne passe pas par ici, DWM la compose seul. Ce fichier ne travaille que
//! lorsque l'interface se repeint (la place de l'élément a pu changer) ou qu'une surface
//! a changé de chaîne d'échange (nouvelle taille).
use super::super::{ffi::Hwnd, graphics::Checked};
use crate::ui::shared_surface::{Shared, SurfacePlacement};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, Weak},
};
use windows::Win32::{
    Foundation::HWND,
    Graphics::{
        DirectComposition::{
            DCOMPOSITION_BITMAP_INTERPOLATION_MODE_LINEAR, DCompositionCreateDevice,
            IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
        },
        Dxgi::{IDXGIDevice, IDXGISwapChain1},
    },
};
use windows_numerics::Matrix3x2;
use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;

thread_local! {
    /// La couche de chaque fenêtre de ce fil qui montre une surface partagée.
    static LAYERS: RefCell<HashMap<isize, Layer>> = RefCell::new(HashMap::new());
    /// La fenêtre dont l'interface est en train d'être dessinée dans sa chaîne
    /// d'échange (0 : aucune ; un dessin hors écran ne troue rien).
    static PAINTING: Cell<isize> = const { Cell::new(0) };
}

/// La couche d'une fenêtre : sa cible de composition, derrière celle de l'interface.
struct Layer {
    device: IDCompositionDevice,
    // Conservée : sa libération détacherait l'arbre de la fenêtre.
    _target: IDCompositionTarget,
    root: IDCompositionVisual,
    surfaces: Vec<Shown>,
}

/// Une surface montrée dans une fenêtre.
struct Shown {
    surface: Weak<Shared>,
    visual: IDCompositionVisual,
    /// Place de l'élément à la dernière peinture de l'interface.
    placement: SurfacePlacement,
    /// Peinte dans l'image en cours.
    painted: bool,
    /// Ce que le visuel montre : la chaîne, et la place à laquelle elle est étirée.
    applied: Option<(IDXGISwapChain1, SurfacePlacement)>,
}

impl Layer {
    fn new(window: Hwnd) -> crate::Result<Self> {
        // SAFETY : appel sans pointeur ; l'IID provient du type demandé.
        let device: IDCompositionDevice = unsafe { DCompositionCreateDevice(None::<&IDXGIDevice>) }
            .check("DCompositionCreateDevice")?;
        // SAFETY : fenêtre valide de ce fil ; `false` : derrière la cible de l'interface.
        let target = unsafe { device.CreateTargetForHwnd(HWND(window), false) }
            .check("CreateTargetForHwnd")?;
        // SAFETY : périphérique valide ; le visuel est possédé par `Layer`.
        let root = unsafe { device.CreateVisual() }.check("CreateVisual")?;
        // SAFETY : cible et visuel issus du même périphérique.
        unsafe { target.SetRoot(&root) }.check("IDCompositionTarget::SetRoot")?;
        Ok(Self {
            device,
            _target: target,
            root,
            surfaces: Vec::new(),
        })
    }

    /// Le visuel de `surface` dans cette couche, créé au besoin.
    fn shown(&mut self, surface: &Arc<Shared>) -> crate::Result<&mut Shown> {
        let weak = Arc::downgrade(surface);
        let found = self.surfaces.iter().position(|s| s.surface.ptr_eq(&weak));
        let index = match found {
            Some(index) => index,
            None => {
                // SAFETY : périphérique valide ; le visuel est possédé par `Shown`.
                let visual = unsafe { self.device.CreateVisual() }.check("CreateVisual")?;
                // SAFETY : visuels du même périphérique ; sans voisin de référence, le
                // nouveau visuel va au-dessus des autres surfaces.
                unsafe {
                    visual
                        .SetBitmapInterpolationMode(DCOMPOSITION_BITMAP_INTERPOLATION_MODE_LINEAR)
                        .check("SetBitmapInterpolationMode")?;
                    self.root
                        .AddVisual(&visual, true, None::<&IDCompositionVisual>)
                        .check("IDCompositionVisual::AddVisual")?;
                }
                self.surfaces.push(Shown {
                    surface: weak,
                    visual,
                    placement: SurfacePlacement {
                        x: 0,
                        y: 0,
                        width: 1,
                        height: 1,
                    },
                    painted: false,
                    applied: None,
                });
                self.surfaces.len() - 1
            }
        };
        Ok(&mut self.surfaces[index])
    }

    /// Met chaque visuel d'accord avec sa surface (chaîne d'échange courante, place) et
    /// retire ceux que `keep` refuse ou dont la surface n'existe plus ; valide le tout.
    fn apply(&mut self, keep: impl Fn(&Shown) -> bool) -> crate::Result<()> {
        let mut changed = false;
        let mut result = Ok(());
        self.surfaces.retain_mut(|shown| {
            let surface = shown.surface.upgrade().filter(|_| keep(shown));
            let Some(surface) = surface else {
                // SAFETY : visuel ajouté à cette racine par `shown`.
                let _ = unsafe { self.root.RemoveVisual(&shown.visual) };
                if let Some(surface) = shown.surface.upgrade() {
                    surface.set_placement(None);
                    surface.native().shown_in(0);
                }
                changed = true;
                return false;
            };
            let wanted = (surface.native().chain(), shown.placement);
            if shown.applied.as_ref() != Some(&wanted) {
                changed = true;
                match show(&shown.visual, &wanted.0, wanted.1) {
                    Ok(()) => shown.applied = Some(wanted),
                    Err(error) => result = Err(error),
                }
            }
            true
        });
        if changed {
            // SAFETY : périphérique valide ; valide la transaction sur ce fil.
            unsafe { self.device.Commit() }.check("IDCompositionDevice::Commit")?;
        }
        result
    }
}

/// Donne à `visual` l'image de `chain`, étirée à la place `placement`.
fn show(
    visual: &IDCompositionVisual,
    chain: &IDXGISwapChain1,
    placement: SurfacePlacement,
) -> crate::Result<()> {
    // SAFETY : chaîne valide ; description copiée par valeur.
    let description = unsafe { chain.GetDesc1() }.check("IDXGISwapChain1::GetDesc1")?;
    // La chaîne a la taille que le producteur dessine ; l'élément peut en avoir déjà
    // changé : l'image est étirée à sa place en attendant la suivante.
    let matrix = Matrix3x2 {
        M11: placement.width as f32 / description.Width.max(1) as f32,
        M12: 0.,
        M21: 0.,
        M22: placement.height as f32 / description.Height.max(1) as f32,
        M31: placement.x as f32,
        M32: placement.y as f32,
    };
    // SAFETY : visuel valide ; DirectComposition prend sa propre référence sur la chaîne
    // et copie la matrice pendant l'appel.
    unsafe {
        visual
            .SetContent(chain)
            .check("IDCompositionVisual::SetContent")?;
        visual
            .SetTransform2(&matrix)
            .check("IDCompositionVisual::SetTransform")
    }
}

/// Le dessin de l'interface d'une fenêtre dans sa chaîne d'échange, pendant lequel les
/// gabarits de surface rencontrés deviennent des trous (`place`).
pub(super) struct Painting {
    window: Hwnd,
    /// La fenêtre montrait déjà une surface avant cette image.
    had_surfaces: bool,
}

impl Painting {
    pub(super) fn begin(window: Hwnd) -> Self {
        PAINTING.with(|painting| painting.set(window as isize));
        let had_surfaces = LAYERS.with_borrow_mut(|layers| {
            // SAFETY : simple question sur une poignée, valide ou non.
            layers.retain(|window, _| unsafe { IsWindow(*window as Hwnd) } != 0);
            let layer = layers.get_mut(&(window as isize));
            let had_surfaces = layer.is_some();
            for shown in layer.into_iter().flat_map(|layer| &mut layer.surfaces) {
                shown.painted = false;
            }
            had_surfaces
        });
        Self {
            window,
            had_surfaces,
        }
    }

    /// L'image de l'interface est présentée : les visuels suivent les éléments peints,
    /// ceux des éléments disparus sont retirés. Rend vrai quand la fenêtre a maintenant
    /// des surfaces et n'en avait pas : elle doit alors se repeindre avec l'alpha.
    pub(super) fn finish(self) -> crate::Result<bool> {
        let (window, had_surfaces) = (self.window as isize, self.had_surfaces);
        drop(self);
        LAYERS.with_borrow_mut(|layers| {
            let Some(layer) = layers.get_mut(&window) else {
                return Ok(false);
            };
            layer.apply(|shown| shown.painted)?;
            let has_surfaces = !layer.surfaces.is_empty();
            if !has_surfaces {
                layers.remove(&window);
            }
            Ok(has_surfaces && !had_surfaces)
        })
    }
}

impl Drop for Painting {
    fn drop(&mut self) {
        PAINTING.with(|painting| painting.set(0));
    }
}

/// Appelé par le dessin de l'interface quand il rencontre le gabarit de `surface` à la
/// place `placement` (pixels de la fenêtre). Vrai : la fenêtre montre la surface sous
/// l'interface, qui doit trouer son image à cet endroit. Faux : rien ne peut la montrer
/// ici (dessin hors écran, composition indisponible), le gabarit se peint tel quel.
pub(crate) fn place(surface: &Arc<Shared>, placement: SurfacePlacement) -> bool {
    let window = PAINTING.with(Cell::get);
    if window == 0 || placement.width == 0 || placement.height == 0 {
        return false;
    }
    LAYERS.with_borrow_mut(|layers| {
        let layer = match layers.entry(window) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => match Layer::new(window as Hwnd) {
                Ok(layer) => entry.insert(layer),
                Err(_) => return false,
            },
        };
        let Ok(shown) = layer.shown(surface) else {
            return false;
        };
        shown.placement = placement;
        shown.painted = true;
        surface.set_placement(Some(placement));
        surface.native().shown_in(window);
        true
    })
}

/// La fenêtre montre-t-elle au moins une surface partagée sous son interface ?
pub(in crate::platform::windows) fn shown_in(window: Hwnd) -> bool {
    LAYERS.with_borrow(|layers| layers.contains_key(&(window as isize)))
}

/// La fenêtre se ferme : sa couche est défaite avant elle, et ses surfaces ne sont plus
/// montrées nulle part.
pub(in crate::platform::windows) fn close(window: Hwnd) {
    let layer = LAYERS.with_borrow_mut(|layers| layers.remove(&(window as isize)));
    if let Some(mut layer) = layer {
        let _ = layer.apply(|_| false);
    }
}

/// Avant une peinture, même sautée : rattache la chaîne d'échange d'une surface qui en a
/// changé (nouvelle taille), sans repeindre l'interface.
pub(super) fn refresh(window: Hwnd) {
    LAYERS.with_borrow_mut(|layers| {
        let window = window as isize;
        if let Some(layer) = layers.get_mut(&window) {
            // Un échec laisse l'ancienne image ; la peinture suivante retentera.
            let _ = layer.apply(|_| true);
            if layer.surfaces.is_empty() {
                layers.remove(&window);
            }
        }
    });
}
