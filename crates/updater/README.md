# karbon-updater

Self-update for desktop applications: ask a release endpoint whether a newer version
exists, download its package, verify its [minisign](https://jedisct1.github.io/minisign/)
signature, install it and restart the application. Windows, macOS and Linux.

It speaks the protocol of the Tauri updater: a server, a signing key and packages made
for it work unchanged, which lets an application move away from Tauri without losing its
users' updates.

```toml
[dependencies]
karbon-updater = "0.3"
```

## In five lines

```rust,no_run
use karbon_updater::Updater;

const PUBLIC_KEY: &str = include_str!("../keys/app.key.pub"); // see "Signing" below

let updater = Updater::new(PUBLIC_KEY, env!("CARGO_PKG_VERSION"))
    .endpoint("https://example.com/releases/{{target}}-{{arch}}/{{current_version}}");
if let Some(update) = updater.check()? {                     // 1. is there a newer version?
    let package = update.download(|done, total| { /* progress */ })?; // 2. download + verify
    package.install()?;                                      // 3. install, schedule the restart
    std::process::exit(0);                                   // 4. exit: the new version starts
}
# Ok::<(), karbon_updater::Error>(())
```

Every call blocks: run them off the UI thread (a karbon-ui `Task`, a thread, or
`tokio::task::spawn_blocking`).

## With karbon-ui

A typical application checks a few seconds after start, shows a card when a version is
available, and installs on demand. Sketch (see RustMusic's `src/ui/pages/updater.rs` for
a complete one, with progress and retries):

```rust,ignore
use karbon_ui::{task::{Task, TaskCommand, TaskId, TaskResult}, window::WindowCommand};
use karbon_updater::{Installation, Update, Updater};

const CHECK: TaskId = TaskId(1);
const INSTALL: TaskId = TaskId(2);

fn updater() -> Updater {
    Updater::new(PUBLIC_KEY, env!("CARGO_PKG_VERSION")).endpoint(ENDPOINT)
}

// At start (skip when the installation cannot update itself, e.g. Flatpak):
if Installation::current().can_update() {
    self.tasks.push(TaskCommand::Spawn(Task::new(CHECK, |_| updater().check())));
}

// Application::task_completed:
if result.id == CHECK {
    if let Ok(Ok(Some(update))) = result.into_value::<Result<Option<Update>, karbon_updater::Error>>() {
        self.available = Some(update); // show "Version x available — Install"
    }
}

// "Install" clicked:
let update = self.available.clone().unwrap();
self.tasks.push(TaskCommand::Spawn(Task::new(INSTALL, move |_| {
    update.download(|_, _| {})?.install()
})));

// INSTALL completed with Ok(()): exit, the new version starts afterwards.
self.window_commands.push(WindowCommand::Exit);
```

## The server

`check` requests each endpoint in order, after replacing these variables:

| Variable | Value |
| --- | --- |
| `{{current_version}}` | the version given to `Updater::new` |
| `{{target}}` | `windows`, `darwin` or `linux` (or `Updater::target`) |
| `{{arch}}` | `x86_64`, `aarch64`, `i686`, `armv7`, `riscv64` |
| `{{bundle_type}}` | `nsis`, `app`, `appimage`, `deb`, `rpm` |

It answers **204 No Content** when there is nothing newer, or a JSON manifest. Static
form, one file for every platform:

```json
{
  "version": "1.2.0",
  "notes": "What changed",
  "pub_date": "2026-11-13T00:00:00Z",
  "platforms": {
    "windows-x86_64": { "url": "https://example.com/App_1.2.0_x64-setup.exe", "signature": "<base64>" },
    "darwin-aarch64": { "url": "https://example.com/App.app.tar.gz", "signature": "<base64>" },
    "linux-x86_64":   { "url": "https://example.com/App_1.2.0_amd64.deb", "signature": "<base64>" }
  }
}
```

The package is looked up under `<os>-<arch>-<bundle_type>`, then `<os>-<arch>` (or only
under `Updater::target`). Dynamic form, when the server already picked the package:
`{"version", "notes", "pub_date", "url", "signature"}`. A version not greater than the
current one (SemVer order) is not an update.

## Signing

Packages are signed with minisign. With the Tauri CLI:

```sh
tauri signer generate -w keys/app.key       # once: secret key (keep it out of git) + app.key.pub
tauri signer sign -f keys/app.key App_1.2.0_x64-setup.exe   # prints the signature (base64)
```

or with minisign itself (`minisign -G`, then `minisign -S -m file`; the manifest's
`signature` is then the base64 of the `.minisig` file). The public key is accepted as the
base64 text `tauri signer` prints, the content of the `.pub` file, or its `RW…` line.

The download is refused unless its signature matches the public key. A signature made by
`tauri signer` also records the version it was made for (`version:` in the trusted
comment): it must be the version the server announced, so a compromised server cannot
pair a new version number with an old, correctly signed package.

## Packages each platform expects

| Platform | `Installation` | Package | What `install` does |
| --- | --- | --- | --- |
| Windows | `WindowsInstaller` | NSIS installer `.exe` | runs it with `/P /UPDATE /R /ARGS <app arguments>`: passive mode, update, restart the app |
| Windows | `WindowsInstaller` | `.msi` | `msiexec /i … /passive /promptrestart AUTOLAUNCHAPP=True` |
| macOS | `MacBundle` | `.app.tar.gz` (the `.app` at the archive's root) | extracts it next to the app, swaps the bundles (administrator password if the bundle is not writable), restarts it |
| Linux | `AppImage` (`$APPIMAGE`) | the AppImage, or an `.AppImage.tar.gz` | replaces the file, restarts it |
| Linux | `Deb` | `.deb` | `pkexec dpkg -i` (desktop password prompt), restarts the app |
| Linux | `Rpm` | `.rpm` | `pkexec rpm -U`, restarts the app |
| any | `Unsupported` | — | Flatpak and Snap (their store updates the app), development builds (run from a Cargo `target` folder), programs not installed by a package: `install` returns `Error::Unsupported` |

On macOS and Linux the restart waits until the current process has exited. On Windows the
installer does it: an NSIS installer must accept the arguments above, like the installer
template of the Tauri bundler does (`/P` passive, `/UPDATE`, `/R` restart, `/ARGS`
followed by the arguments to restart the application with).

## Errors

`Error::Network` (endpoint unreachable, error status), `Error::Manifest` (unreadable
answer), `Error::NoPackage` (no package for this platform), `Error::Signature`,
`Error::Unsupported`, `Error::Io`, `Error::Install`. A failed installation leaves the
installed application as it was.

## Dependencies

[`ureq`](https://crates.io/crates/ureq) (HTTPS through rustls, Mozilla's root
certificates) and [`minisign-verify`](https://crates.io/crates/minisign-verify); the
manifest is read by a small JSON reader of the crate. On Windows, `windows-sys` starts the
installer through the Shell (so an installer asking for elevation gets the UAC prompt).

## Trying it

```sh
cargo run -p karbon-updater --example check -- <public key file> <current version> <endpoint> [--download]
```

prints what the server announces and, with `--download`, downloads the package and
verifies its signature, without installing anything.
