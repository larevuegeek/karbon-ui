//! macOS et Linux : relance de l'application une fois ce processus terminé. Un `sh`
//! détaché attend la fin du PID courant (l'application a le temps d'enregistrer son état
//! et de libérer ses fichiers), puis exécute la commande donnée.
use crate::Error;
use std::{
    ffi::OsString,
    process::{Command, Stdio},
};

/// Exécute `program arguments…` quand le processus courant sera terminé.
pub(crate) fn after_exit(program: OsString, arguments: Vec<OsString>) -> Result<(), Error> {
    // `$0` : PID à attendre ; `$@` : la commande, sans passer par l'interprétation du shell.
    Command::new("/bin/sh")
        .arg("-c")
        .arg(r#"while kill -0 "$0" 2>/dev/null; do sleep 0.2; done; exec "$@""#)
        .arg(std::process::id().to_string())
        .arg(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|e| Error::Install(format!("the restart could not be scheduled ({e})")))
}

/// Arguments de l'application en cours (sans le programme).
pub(crate) fn current_arguments() -> Vec<OsString> {
    std::env::args_os().skip(1).collect()
}
