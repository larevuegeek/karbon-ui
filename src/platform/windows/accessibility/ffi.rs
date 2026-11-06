//! Codes UIA et utilitaires VARIANT/BSTR/SAFEARRAY au-dessus des liaisons windows-rs.
use std::{mem::ManuallyDrop, ptr::null_mut};
use windows::{
    Win32::{
        Foundation::{
            E_FAIL, E_INVALIDARG, E_NOINTERFACE, E_NOTIMPL, E_POINTER, S_OK, SysAllocStringLen,
            VARIANT_BOOL,
        },
        System::{
            Com::SAFEARRAY,
            Ole::{
                SafeArrayAccessData, SafeArrayCreateVector, SafeArrayDestroy, SafeArrayPutElement,
                SafeArrayUnaccessData,
            },
            Variant::{
                VARENUM, VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_ARRAY, VT_BOOL,
                VT_BSTR, VT_I4, VT_R8, VT_UNKNOWN, VariantClear,
            },
        },
        UI::Accessibility::{
            UIA_E_ELEMENTNOTAVAILABLE, UIA_E_ELEMENTNOTENABLED, UIA_E_INVALIDOPERATION,
            UIA_E_NOTSUPPORTED, UIA_E_TIMEOUT, UiaRect,
        },
    },
    core::{BSTR, Error, HRESULT, IUnknown, Interface, Result},
};

pub(super) const OK: HRESULT = S_OK;
pub(super) const FAIL: HRESULT = E_FAIL;
pub(super) const POINTER: HRESULT = E_POINTER;
pub(super) const NO_INTERFACE: HRESULT = E_NOINTERFACE;
pub(super) const NOT_IMPLEMENTED: HRESULT = E_NOTIMPL;
pub(super) const INVALID_ARGUMENT: HRESULT = E_INVALIDARG;
pub(super) const UNAVAILABLE: HRESULT = HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32);
pub(super) const DISABLED: HRESULT = HRESULT(UIA_E_ELEMENTNOTENABLED as i32);
pub(super) const UNSUPPORTED: HRESULT = HRESULT(UIA_E_NOTSUPPORTED as i32);
pub(super) const INVALID_OPERATION: HRESULT = HRESULT(UIA_E_INVALIDOPERATION as i32);
pub(super) const TIMEOUT: HRESULT = HRESULT(UIA_E_TIMEOUT as i32);
pub(super) const COMMAND_MESSAGE: u32 = 0x8000 + 0x201;

/// Erreur COM sans IErrorInfo : seul le HRESULT traverse la frontière.
pub(super) fn fail(code: HRESULT) -> Error {
    Error::from_hresult(code)
}
pub(super) fn check(code: HRESULT) -> Result<()> {
    if code.is_ok() {
        Ok(())
    } else {
        Err(fail(code))
    }
}
/// S_OK sans objet : windows-rs traduit `Error::empty()` en succès avec sortie nulle.
pub(super) fn nothing<T>() -> Result<T> {
    Err(Error::empty())
}
pub(super) fn contains(rect: &UiaRect, x: f64, y: f64) -> bool {
    rect.width > 0.
        && rect.height > 0.
        && x >= rect.left
        && x < rect.left + rect.width
        && y >= rect.top
        && y < rect.top + rect.height
}
/// BSTR toujours non nul, même vide, comme SysAllocStringLen(ptr, 0).
pub(super) fn bstr(value: &str) -> Result<BSTR> {
    let chars: Vec<u16> = value.encode_utf16().collect();
    // SAFETY: la tranche UTF-16 reste valide pendant l'appel ; le BSTR rendu est possédé.
    let raw = unsafe { SysAllocStringLen(Some(&chars)) }.into_raw();
    if raw.is_null() {
        return Err(fail(FAIL));
    }
    // SAFETY: `raw` vient de SysAllocStringLen et sa propriété est reprise une seule fois.
    Ok(unsafe { BSTR::from_raw(raw) })
}

/// VARIANT possédé : VariantClear libère BSTR, interfaces et tableaux au drop.
/// La valeur par défaut est VT_EMPTY.
#[derive(Default)]
pub(super) struct Variant(VARIANT);
impl Drop for Variant {
    fn drop(&mut self) {
        // SAFETY: le VARIANT est possédé et initialisé par l'un des constructeurs ci-dessous.
        let _ = unsafe { VariantClear(&mut self.0) };
    }
}
impl Variant {
    fn new(kind: VARENUM, data: VARIANT_0_0_0) -> Self {
        Self(VARIANT {
            Anonymous: VARIANT_0 {
                Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                    vt: kind,
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: data,
                }),
            },
        })
    }
    pub fn integer(value: i32) -> Self {
        Self::new(VT_I4, VARIANT_0_0_0 { lVal: value })
    }
    pub fn number(value: f64) -> Self {
        Self::new(VT_R8, VARIANT_0_0_0 { dblVal: value })
    }
    pub fn boolean(value: bool) -> Self {
        Self::new(
            VT_BOOL,
            VARIANT_0_0_0 {
                boolVal: VARIANT_BOOL(if value { -1 } else { 0 }),
            },
        )
    }
    pub fn string(value: &str) -> Result<Self> {
        Ok(Self::new(
            VT_BSTR,
            VARIANT_0_0_0 {
                bstrVal: ManuallyDrop::new(bstr(value)?),
            },
        ))
    }
    pub fn unknown(value: IUnknown) -> Self {
        Self::new(
            VT_UNKNOWN,
            VARIANT_0_0_0 {
                punkVal: ManuallyDrop::new(Some(value)),
            },
        )
    }
    /// Adopte un SAFEARRAY possédé d'éléments `kind`.
    pub fn array(kind: VARENUM, array: *mut SAFEARRAY) -> Self {
        Self::new(
            VARENUM(VT_ARRAY.0 | kind.0),
            VARIANT_0_0_0 { parray: array },
        )
    }
    pub fn rectangle(rect: UiaRect) -> Result<Self> {
        Ok(Self::array(
            VT_R8,
            doubles(&[rect.left, rect.top, rect.width, rect.height])?,
        ))
    }
    #[cfg(test)]
    pub fn kind(&self) -> u16 {
        // SAFETY: `vt` occupe les mêmes octets dans toutes les variantes de l'union.
        unsafe { self.0.Anonymous.Anonymous.vt.0 }
    }
    pub fn get(&self) -> &VARIANT {
        &self.0
    }
    /// Transfère la propriété à l'appelant COM, sans VariantClear.
    pub fn into_raw(self) -> VARIANT {
        let this = ManuallyDrop::new(self);
        // SAFETY: `this` n'est plus jamais lu ni libéré ; la copie devient l'unique propriétaire.
        unsafe { std::ptr::read(&this.0) }
    }
    /// # Safety
    /// `value` doit être un VARIANT valide dont la propriété est transférée.
    #[cfg(test)]
    pub unsafe fn adopt(value: VARIANT) -> Self {
        Self(value)
    }
    #[cfg(test)]
    fn data(&self, kind: VARENUM) -> Option<&VARIANT_0_0_0> {
        // SAFETY: le champ n'est rendu que si `vt` annonce ce type de donnée.
        (self.kind() == kind.0).then(|| unsafe { &self.0.Anonymous.Anonymous.Anonymous })
    }
    #[cfg(test)]
    pub fn as_bool(&self) -> Option<bool> {
        // SAFETY: VT_BOOL garantit que `boolVal` est initialisé.
        self.data(VT_BOOL).map(|d| unsafe { d.boolVal }.0 != 0)
    }
    #[cfg(test)]
    pub fn as_i32(&self) -> Option<i32> {
        // SAFETY: VT_I4 garantit que `lVal` est initialisé.
        self.data(VT_I4).map(|d| unsafe { d.lVal })
    }
    #[cfg(test)]
    pub fn as_f64(&self) -> Option<f64> {
        // SAFETY: VT_R8 garantit que `dblVal` est initialisé.
        self.data(VT_R8).map(|d| unsafe { d.dblVal })
    }
    #[cfg(test)]
    pub fn as_array(&self, kind: VARENUM) -> Option<*mut SAFEARRAY> {
        // SAFETY: VT_ARRAY|kind garantit que `parray` est initialisé.
        self.data(VARENUM(VT_ARRAY.0 | kind.0))
            .map(|d| unsafe { d.parray })
    }
}

/// Vecteur SAFEARRAY ; `T` doit correspondre exactement au VARENUM.
fn vector<T: Copy>(kind: VARENUM, values: &[T]) -> Result<*mut SAFEARRAY> {
    // SAFETY: création d'un vecteur à une dimension, sans pointeur fourni.
    let array = unsafe { SafeArrayCreateVector(kind, 0, values.len() as u32) };
    if array.is_null() {
        return Err(fail(FAIL));
    }
    let mut data = null_mut();
    // SAFETY: `array` est valide ; `data` reçoit le stockage verrouillé.
    if let Err(error) = unsafe { SafeArrayAccessData(array, &mut data) } {
        // SAFETY: le tableau possédé n'a jamais été rendu.
        let _ = unsafe { SafeArrayDestroy(array) };
        return Err(error);
    }
    // SAFETY: le stockage verrouillé contient exactement `values.len()` éléments de type `T`.
    unsafe {
        if !values.is_empty() {
            std::ptr::copy_nonoverlapping(values.as_ptr(), data.cast::<T>(), values.len());
        }
        let _ = SafeArrayUnaccessData(array);
    }
    Ok(array)
}
pub(super) fn doubles(values: &[f64]) -> Result<*mut SAFEARRAY> {
    vector(VT_R8, values)
}
pub(super) fn integers(values: &[i32]) -> Result<*mut SAFEARRAY> {
    vector(VT_I4, values)
}
/// VT_UNKNOWN : SafeArrayPutElement prend sa propre référence COM.
pub(super) fn unknowns(items: &[IUnknown]) -> Result<*mut SAFEARRAY> {
    // SAFETY: création d'un vecteur d'interfaces initialisé à null.
    let array = unsafe { SafeArrayCreateVector(VT_UNKNOWN, 0, items.len() as u32) };
    if array.is_null() {
        return Err(fail(FAIL));
    }
    for (index, item) in items.iter().enumerate() {
        // SAFETY: indice borné par la taille ; l'interface reste vivante pendant l'appel.
        if let Err(error) = unsafe { SafeArrayPutElement(array, &(index as i32), item.as_raw()) } {
            // SAFETY: le tableau possédé libère les références déjà stockées.
            let _ = unsafe { SafeArrayDestroy(array) };
            return Err(error);
        }
    }
    Ok(array)
}

/// Copie les éléments d'un vecteur sans le libérer (tests uniquement).
///
/// # Safety
/// `array` doit être un vecteur valide dont les éléments sont de type `T`.
#[cfg(test)]
pub(super) unsafe fn elements<T: Clone>(array: *mut SAFEARRAY) -> Vec<T> {
    assert!(!array.is_null());
    // SAFETY: garanti par l'appelant ; les éléments sont clonés (AddRef) sous verrou d'accès.
    unsafe {
        let len = (*array).rgsabound[0].cElements as usize;
        let mut data = null_mut();
        SafeArrayAccessData(array, &mut data).unwrap();
        let values = if len == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(data.cast::<T>(), len).to_vec()
        };
        SafeArrayUnaccessData(array).unwrap();
        values
    }
}
/// Copie les éléments puis détruit le tableau possédé (tests uniquement).
///
/// # Safety
/// `array` doit être un vecteur possédé dont les éléments sont de type `T`.
#[cfg(test)]
pub(super) unsafe fn take<T: Clone>(array: *mut SAFEARRAY) -> Vec<T> {
    // SAFETY: garanti par l'appelant ; la destruction libère les références d'origine.
    unsafe {
        let values = elements(array);
        SafeArrayDestroy(array).unwrap();
        values
    }
}
