//! Secrets dans le Trousseau de macOS (mots de passe génériques : service, compte = clé),
//! par les fonctions C de Security.framework.
use std::ffi::{c_char, c_void};

type OsStatus = i32;
const ERR_SEC_ITEM_NOT_FOUND: OsStatus = -25300;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecKeychainAddGenericPassword(
        keychain: *const c_void,
        service_length: u32,
        service: *const c_char,
        account_length: u32,
        account: *const c_char,
        password_length: u32,
        password: *const c_void,
        item: *mut *mut c_void,
    ) -> OsStatus;
    fn SecKeychainFindGenericPassword(
        keychains: *const c_void,
        service_length: u32,
        service: *const c_char,
        account_length: u32,
        account: *const c_char,
        password_length: *mut u32,
        password: *mut *mut c_void,
        item: *mut *mut c_void,
    ) -> OsStatus;
    fn SecKeychainItemModifyAttributesAndData(
        item: *mut c_void,
        attributes: *const c_void,
        length: u32,
        data: *const c_void,
    ) -> OsStatus;
    fn SecKeychainItemDelete(item: *mut c_void) -> OsStatus;
    fn SecKeychainItemFreeContent(attributes: *mut c_void, data: *mut c_void) -> OsStatus;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(object: *const c_void);
}

fn failure(operation: &'static str, status: OsStatus) -> crate::Error {
    crate::Error::Platform {
        operation,
        code: status,
    }
}

/// Élément du trousseau et, à la demande, son mot de passe.
fn find(
    service: &str,
    key: &str,
    with_data: bool,
) -> crate::Result<Option<(*mut c_void, Vec<u8>)>> {
    let (mut length, mut data, mut item) = (0u32, std::ptr::null_mut(), std::ptr::null_mut());
    // SAFETY : chaînes passées avec leur longueur ; le trousseau alloue `data` et `item`.
    let status = unsafe {
        SecKeychainFindGenericPassword(
            std::ptr::null(),
            service.len() as u32,
            service.as_ptr().cast(),
            key.len() as u32,
            key.as_ptr().cast(),
            if with_data {
                &mut length
            } else {
                std::ptr::null_mut()
            },
            if with_data {
                &mut data
            } else {
                std::ptr::null_mut()
            },
            &mut item,
        )
    };
    match status {
        0 => {
            let mut bytes = Vec::new();
            if with_data && !data.is_null() {
                // SAFETY : `data` fait `length` octets, libéré juste après la copie.
                unsafe {
                    bytes = std::slice::from_raw_parts(data.cast::<u8>(), length as usize).to_vec();
                    SecKeychainItemFreeContent(std::ptr::null_mut(), data);
                }
            }
            Ok(Some((item, bytes)))
        }
        ERR_SEC_ITEM_NOT_FOUND => Ok(None),
        status => Err(failure("SecKeychainFindGenericPassword", status)),
    }
}

fn release(item: *mut c_void) {
    if !item.is_null() {
        // SAFETY : référence rendue par le trousseau, libérée une fois.
        unsafe { CFRelease(item) };
    }
}

pub(crate) fn set(service: &str, key: &str, secret: &str) -> crate::Result<()> {
    if let Some((item, _)) = find(service, key, false)? {
        // SAFETY : élément valide jusqu'à `release`.
        let status = unsafe {
            SecKeychainItemModifyAttributesAndData(
                item,
                std::ptr::null(),
                secret.len() as u32,
                secret.as_ptr().cast(),
            )
        };
        release(item);
        return match status {
            0 => Ok(()),
            status => Err(failure("SecKeychainItemModifyAttributesAndData", status)),
        };
    }
    // SAFETY : chaînes passées avec leur longueur ; pas d'élément demandé en retour.
    let status = unsafe {
        SecKeychainAddGenericPassword(
            std::ptr::null(),
            service.len() as u32,
            service.as_ptr().cast(),
            key.len() as u32,
            key.as_ptr().cast(),
            secret.len() as u32,
            secret.as_ptr().cast(),
            std::ptr::null_mut(),
        )
    };
    match status {
        0 => Ok(()),
        status => Err(failure("SecKeychainAddGenericPassword", status)),
    }
}

pub(crate) fn get(service: &str, key: &str) -> crate::Result<Option<String>> {
    let Some((item, bytes)) = find(service, key, true)? else {
        return Ok(None);
    };
    release(item);
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| crate::Error::invalid_input("stored secret is not UTF-8"))
}

pub(crate) fn remove(service: &str, key: &str) -> crate::Result<()> {
    let Some((item, _)) = find(service, key, false)? else {
        return Ok(());
    };
    // SAFETY : élément valide jusqu'à `release`.
    let status = unsafe { SecKeychainItemDelete(item) };
    release(item);
    match status {
        0 | ERR_SEC_ITEM_NOT_FOUND => Ok(()),
        status => Err(failure("SecKeychainItemDelete", status)),
    }
}
