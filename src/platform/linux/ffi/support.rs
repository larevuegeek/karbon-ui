/// Échec d'un appel natif qui ne fournit pas de code d'erreur : une sortie nulle devient
/// `Error::Platform` avec le code générique `E_FAIL`, comme sous macOS.
pub(crate) fn failure(operation: &'static str) -> crate::Error {
    crate::Error::Platform {
        operation,
        code: 0x8000_4005_u32 as i32,
    }
}
/// Pointeur non nul obligatoire renvoyé par un appel natif.
pub(crate) fn required<T>(
    value: *mut T,
    operation: &'static str,
) -> crate::Result<std::ptr::NonNull<T>> {
    std::ptr::NonNull::new(value).ok_or_else(|| failure(operation))
}
/// Chaîne C d'un nom (famille, attribut) ; un octet nul intérieur est refusé.
pub(crate) fn c_string(value: &str) -> crate::Result<std::ffi::CString> {
    std::ffi::CString::new(value).map_err(|_| crate::Error::invalid_input("string contains NUL"))
}
