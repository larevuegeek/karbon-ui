//! Secrets dans le Gestionnaire d'identification de Windows (identifiants génériques),
//! chiffrés par DPAPI pour l'utilisateur ; cible « service/clé ».
use super::ffi::{last_error, win32_error};
use windows_sys::Win32::{
    Foundation::{ERROR_NOT_FOUND, FILETIME, GetLastError},
    Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree,
        CredReadW, CredWriteW,
    },
};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
fn target(service: &str, key: &str) -> Vec<u16> {
    wide(&format!("{service}/{key}"))
}

pub(crate) fn set(service: &str, key: &str, secret: &str) -> crate::Result<()> {
    let mut target = target(service, key);
    let mut user = wide(key);
    let mut blob = secret.as_bytes().to_vec();
    let credential = CREDENTIALW {
        Flags: 0,
        Type: CRED_TYPE_GENERIC,
        TargetName: target.as_mut_ptr(),
        Comment: std::ptr::null_mut(),
        LastWritten: FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        },
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: std::ptr::null_mut(),
        UserName: user.as_mut_ptr(),
    };
    // SAFETY : la structure et ses tampons vivent jusqu'au retour ; Windows les copie.
    let ok = unsafe { CredWriteW(&credential, 0) };
    blob.fill(0);
    if ok == 0 {
        return Err(last_error("CredWriteW"));
    }
    Ok(())
}

pub(crate) fn get(service: &str, key: &str) -> crate::Result<Option<String>> {
    let target = target(service, key);
    let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY : cible terminée par un nul ; Windows alloue la structure rendue.
    if unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) } == 0 {
        // SAFETY : lecture de l'erreur du thread courant.
        let code = unsafe { GetLastError() };
        return if code == ERROR_NOT_FOUND {
            Ok(None)
        } else {
            Err(win32_error("CredReadW", code))
        };
    }
    // SAFETY : `credential` est valide jusqu'à `CredFree` ; le tampon fait
    // `CredentialBlobSize` octets.
    let bytes = unsafe {
        let c = &*credential;
        let bytes = if c.CredentialBlob.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec()
        };
        CredFree(credential.cast());
        bytes
    };
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| crate::Error::invalid_input("stored secret is not UTF-8"))
}

pub(crate) fn remove(service: &str, key: &str) -> crate::Result<()> {
    let target = target(service, key);
    // SAFETY : cible terminée par un nul.
    if unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0 {
        // SAFETY : lecture de l'erreur du thread courant.
        let code = unsafe { GetLastError() };
        if code != ERROR_NOT_FOUND {
            return Err(win32_error("CredDeleteW", code));
        }
    }
    Ok(())
}
