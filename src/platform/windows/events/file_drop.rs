//! Dépôt de fichiers depuis l'Explorateur (`Element::on_file_drop`) : la fenêtre
//! s'inscrit comme cible OLE (`RegisterDragDrop`) dès qu'un tel gestionnaire a été posé
//! sur son fil, une fois. Les survols, sorties et dépôts vont au runtime avec les chemins
//! lus du format `CF_HDROP` ; le curseur dit « copier » quand un élément accepte,
//! « interdit » sinon. Une application sans `on_file_drop` ne s'inscrit jamais.
use super::super::{
    ffi::{Hwnd, invalidate, scale, screen_to_client},
    present::runtime,
};
use crate::ui::{geometry::Point, runtime::FileDropPhase, scene::PointF};
use std::{cell::RefCell, path::PathBuf, sync::Arc};
use windows::{
    Win32::{
        Foundation::{HWND, POINTL},
        System::{
            Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL},
            Ole::{
                CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE, IDropTarget,
                IDropTarget_Impl, OleInitialize, RegisterDragDrop, ReleaseStgMedium,
            },
            SystemServices::MODIFIERKEYS_FLAGS,
        },
        UI::Shell::{DragQueryFileW, HDROP},
    },
    core::{Ref, implement},
};

thread_local! {
    /// Fenêtres déjà inscrites comme cibles de dépôt sur ce fil.
    static REGISTERED: RefCell<Vec<Hwnd>> = const { RefCell::new(Vec::new()) };
}

/// Inscrit `window` comme cible de dépôt si un gestionnaire `on_file_drop` a été posé sur
/// ce fil et qu'elle ne l'est pas encore. Un échec est sans suite : la fenêtre refuse
/// alors les fichiers, comme avant.
pub(crate) fn ensure(window: Hwnd) {
    let wanted = crate::ui::runtime::raw_input::FILE_DROP_BUILT.with(std::cell::Cell::get);
    if !wanted || REGISTERED.with(|r| r.borrow().contains(&window)) {
        return;
    }
    REGISTERED.with(|r| r.borrow_mut().push(window));
    let target: IDropTarget = DropTarget {
        window,
        paths: RefCell::new(Arc::from(Vec::new())),
    }
    .into();
    // SAFETY : OLE est initialisé sur le fil de la fenêtre (déjà en appartement unique :
    // l'appel ne fait qu'en augmenter le compte) ; la fenêtre est valide et la cible
    // vivra tant que le système la garde (il la relâche à la destruction de la fenêtre).
    unsafe {
        let _ = OleInitialize(None);
        let _ = RegisterDragDrop(HWND(window), &target);
    }
}

/// La cible de dépôt d'une fenêtre : les chemins du glisser en cours.
#[implement(IDropTarget, Agile = false)]
struct DropTarget {
    window: Hwnd,
    paths: RefCell<Arc<[PathBuf]>>,
}

impl DropTarget {
    /// Donne au runtime une étape du glisser à `pt` (écran) ; l'effet à montrer.
    fn deliver(&self, phase: FileDropPhase, pt: &POINTL) -> DROPEFFECT {
        let paths = Arc::clone(&self.paths.borrow());
        let position = screen_to_client(self.window, Point { x: pt.x, y: pt.y })
            .map(|p| {
                let factor = scale(self.window);
                PointF {
                    x: p.x as f32 / factor,
                    y: p.y as f32 / factor,
                }
            })
            .unwrap_or(PointF { x: -1., y: -1. });
        let mut accepted = false;
        let result = runtime::raw_input(self.window, false, |ui| {
            accepted = ui.file_drag(phase, &paths, position);
            crate::ui::runtime::EventResult {
                repaint: true,
                ..Default::default()
            }
        });
        if result.repaint {
            invalidate(self.window);
        }
        if accepted {
            DROPEFFECT_COPY
        } else {
            DROPEFFECT_NONE
        }
    }
}

impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(
        &self,
        data: Ref<IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        *self.paths.borrow_mut() = data.as_ref().map(paths_of).unwrap_or_default().into();
        let shown = self.deliver(FileDropPhase::Over, pt);
        // SAFETY : pointeur d'effet fourni par OLE, valide le temps de l'appel.
        unsafe { set(effect, shown) };
        Ok(())
    }

    fn DragOver(
        &self,
        _: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let shown = self.deliver(FileDropPhase::Over, pt);
        // SAFETY : voir DragEnter.
        unsafe { set(effect, shown) };
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        self.deliver(FileDropPhase::Leave, &POINTL { x: -1, y: -1 });
        *self.paths.borrow_mut() = Arc::from(Vec::new());
        Ok(())
    }

    fn Drop(
        &self,
        data: Ref<IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        if let Some(data) = data.as_ref() {
            *self.paths.borrow_mut() = paths_of(data).into();
        }
        let shown = self.deliver(FileDropPhase::Drop, pt);
        *self.paths.borrow_mut() = Arc::from(Vec::new());
        // SAFETY : voir DragEnter.
        unsafe { set(effect, shown) };
        Ok(())
    }
}

/// Écrit l'effet à montrer, si OLE a donné où.
///
/// # Safety
///
/// `effect` est nul ou pointe vers un `DROPEFFECT` modifiable.
unsafe fn set(effect: *mut DROPEFFECT, value: DROPEFFECT) {
    if !effect.is_null() {
        // SAFETY : non nul, et modifiable selon le contrat de l'appelant.
        unsafe { *effect = value };
    }
}

/// Les chemins complets portés par `data` au format `CF_HDROP` (des fichiers de
/// l'Explorateur) ; aucun pour d'autres données (du texte tiré d'un navigateur).
fn paths_of(data: &IDataObject) -> Vec<PathBuf> {
    let format = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    // SAFETY : format décrit ci-dessus ; le support rendu est relâché après lecture.
    let Ok(mut medium) = (unsafe { data.GetData(&format) }) else {
        return Vec::new();
    };
    // SAFETY : un support TYMED_HGLOBAL de CF_HDROP porte un HDROP valide tant que le
    // support n'est pas relâché.
    let paths = unsafe {
        let drop = HDROP(medium.u.hGlobal.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        (0..count)
            .map(|index| {
                let length = DragQueryFileW(drop, index, None) as usize;
                let mut buffer = vec![0u16; length + 1];
                let written = DragQueryFileW(drop, index, Some(&mut buffer)) as usize;
                PathBuf::from(String::from_utf16_lossy(&buffer[..written.min(length)]))
            })
            .collect()
    };
    // SAFETY : support obtenu par GetData, relâché une fois.
    unsafe { ReleaseStgMedium(&mut medium) };
    paths
}
