//! Windows : lance le nouvel installateur, comme le module de mise à jour de Tauri (mode
//! passif). NSIS : `/P /UPDATE /R /ARGS <arguments de l'application>` ; MSI :
//! `msiexec /i <paquet> /passive /promptrestart AUTOLAUNCHAPP=True LAUNCHAPPARGS="…"`.
//! L'installateur attend la fin de l'application, la remplace puis la relance.
use crate::{Error, Installation};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW};

pub(crate) fn install(package: &Path, installation: &Installation) -> Result<(), Error> {
    if let Installation::Unsupported(reason) = installation {
        return Err(Error::Unsupported(reason.clone()));
    }
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(package)?;
    let (file, parameters) = if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        // Paquet MSI (fichier composé OLE).
        let system = std::env::var("SYSTEMROOT").unwrap_or_else(|_| r"C:\Windows".into());
        let launch = arguments
            .iter()
            .map(|a| a.replace('"', "\"\""))
            .collect::<Vec<_>>()
            .join(" ");
        (
            format!(r"{system}\System32\msiexec.exe"),
            format!(
                "/i \"{}\" /passive /promptrestart AUTOLAUNCHAPP=True LAUNCHAPPARGS=\"{launch}\"",
                package.display()
            ),
        )
    } else if bytes.starts_with(b"MZ") {
        let mut parameters = String::from("/P /UPDATE /R /ARGS");
        for argument in &arguments {
            parameters.push(' ');
            parameters.push_str(&nsis_argument(argument));
        }
        (package.display().to_string(), parameters)
    } else {
        return Err(Error::Install(
            "the Windows package is neither an installer (.exe) nor an MSI".into(),
        ));
    };
    let wide = |text: &str| -> Vec<u16> { OsStr::new(text).encode_wide().chain(Some(0)).collect() };
    let (verb, file, parameters) = (wide("open"), wide(&file), wide(&parameters));
    // SAFETY : chaînes terminées par zéro, vivantes pendant l'appel ; aucune fenêtre parente.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            parameters.as_ptr(),
            std::ptr::null(),
            SW_SHOW,
        )
    };
    // Une valeur supérieure à 32 signale le succès.
    if result as isize <= 32 {
        return Err(Error::Install(format!(
            "the installer could not be started ({})",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

/// Argument transmis à l'application relancée, échappé comme le fait Tauri : guillemets
/// autour des arguments avec blanc ou `/` (NSIS les lirait comme ses propres options),
/// barres obliques inverses doublées devant un guillemet.
pub(crate) fn nsis_argument(argument: &str) -> String {
    let quote = argument.is_empty() || argument.contains([' ', '\t', '/']);
    let mut out = String::new();
    if quote {
        out.push('"');
    }
    let mut backslashes = 0;
    for c in argument.chars() {
        if c == '\\' {
            backslashes += 1;
        } else {
            if c == '"' {
                out.extend(std::iter::repeat_n('\\', backslashes + 1));
            }
            backslashes = 0;
        }
        out.push(c);
    }
    if quote {
        out.extend(std::iter::repeat_n('\\', backslashes));
        out.push('"');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_escaped_for_nsis() {
        assert_eq!(nsis_argument("--minimized"), "--minimized");
        assert_eq!(nsis_argument("/play"), "\"/play\"");
        assert_eq!(nsis_argument("a b"), "\"a b\"");
        assert_eq!(nsis_argument(""), "\"\"");
        assert_eq!(nsis_argument(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(nsis_argument(r"C:\dir\ x\"), r#""C:\dir\ x\\""#);
    }
}
