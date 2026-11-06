//! Bootstrap 5.3 rendered natively: the unmodified `bootstrap.css` compiled in framework
//! mode, Bootstrap markup in `.ui` templates, `data-bs-*` behaviours mapped onto native
//! components. See docs/css-frameworks.md.
//!
//! ```powershell
//! cargo run --release --example bootstrap_demo
//! cargo run --release --example bootstrap_demo -- --theme dark
//! cargo run --release --example bootstrap_demo -- --export demo.png --size 1280x2000 --scale 1.5 --open dropdown
//! cargo run --release --example bootstrap_demo -- --report
//! ```
use karbon_ui::{
    Error,
    diagnostics::{self, BenchmarkOptions},
    platform,
    ui::{bootstrap, prelude::*},
    window::{WindowAction, WindowCommand, WindowId, WindowOptions},
};
css_file!(mod bs, "../tests/fixtures/bootstrap/bootstrap.css", framework);

// Seuls la case, l'interrupteur et le curseur gardent leur valeur (les champs texte
// et la liste conservent la leur dans leur composant).
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Msg {
    Noop,
    Text(String),
    Pick(u64),
    Check(bool),
    Switch(bool),
    Range(f64),
    Window(WindowAction),
}
#[derive(Clone)]
struct Demo {
    theme: &'static str,
    /// Barre de titre Bootstrap dans la page (fenêtre interactive, pas les exports).
    chrome: bool,
    checked: bool,
    switched: bool,
    range: f64,
    actions: Vec<WindowAction>,
}
impl Application for Demo {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        let (theme, chrome) = (self.theme, self.chrome);
        let (checked, switched, range) = (self.checked, self.switched, self.range);
        ui_file!("bootstrap_demo/page.ui", context = cx, styles = bs)
    }
    fn update(&mut self, message: Msg) -> bool {
        // Les comportements Bootstrap (menus, onglets, accordéon…) sont appliqués par le
        // runtime ; seules les valeurs des contrôles et les actions de fenêtre arrivent ici.
        match message {
            Msg::Check(on) => self.checked = on,
            Msg::Switch(on) => self.switched = on,
            Msg::Range(value) => self.range = value,
            Msg::Window(action) => {
                self.actions.push(action);
                return false;
            }
            Msg::Noop | Msg::Text(_) | Msg::Pick(_) => return false,
        }
        true
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.actions)
            .into_iter()
            .map(|action| WindowCommand::Action(WindowId::MAIN, action))
            .collect()
    }
}
/// Cibles ouvrables avant un export (`--open`).
const OPEN: &[(&str, &str)] = &[
    ("dropdown", "bs-dropdown-menu"),
    ("modal", "exampleModal"),
    ("collapse", "collapseExample"),
    ("navbar", "navbarNav"),
    ("offcanvas", "offcanvasExample"),
];
fn main() -> karbon_ui::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut native = Vec::new();
    let (mut export, mut dump, mut size, mut scale) = (None, None, None, None);
    let mut demo = Demo {
        theme: "light",
        chrome: false,
        checked: true,
        switched: false,
        range: 50.,
        actions: Vec::new(),
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--export" => {
                export = Some(
                    args.next()
                        .ok_or_else(|| Error::application("--export expects a PNG path"))?,
                )
            }
            "--dump" => {
                dump = Some(
                    args.next()
                        .ok_or_else(|| Error::application("--dump expects a text path"))?,
                )
            }
            "--size" => {
                let value = args
                    .next()
                    .ok_or_else(|| Error::application("--size expects WIDTHxHEIGHT"))?;
                let (w, h) = value
                    .split_once('x')
                    .ok_or_else(|| Error::application("example: --size 1280x900"))?;
                let parse = |v: &str| v.parse::<u32>().map_err(Error::application);
                size = Some((parse(w)?, parse(h)?));
            }
            "--scale" => {
                let value = args
                    .next()
                    .ok_or_else(|| Error::application("--scale expects a factor, e.g. 1.5"))?;
                scale = Some(value.parse::<f32>().map_err(Error::application)?);
            }
            "--theme" => {
                demo.theme = match args.next().as_deref() {
                    Some("dark") => "dark",
                    Some("light") => "light",
                    _ => return Err(Error::application("--theme expects light or dark")),
                }
            }
            "--open" => {
                let name = args.next().unwrap_or_default();
                for target in name.split(',') {
                    let (_, key) = OPEN.iter().find(|(n, _)| *n == target).ok_or_else(|| {
                        Error::application(
                            "--open expects dropdown, modal, collapse, navbar or offcanvas",
                        )
                    })?;
                    bootstrap::set_shown(key, true);
                }
            }
            "--tab" => {
                let pane = args.next().unwrap_or_default();
                for p in ["home-pane", "profile-pane", "contact-pane"] {
                    bootstrap::set_shown(p, p == pane);
                }
            }
            // Aperçu de la barre de titre de la fenêtre dans un export.
            "--title-bar" => demo.chrome = true,
            // Interrupteur coché (état `:checked` dans un export).
            "--switch" => demo.switched = true,
            "--report" => {
                println!("{}", bs::REPORT);
                return Ok(());
            }
            "--help" => {
                println!(
                    "Bootstrap 5.3 · karbon-ui\n--export <image.png> [--size 1280x2000] [--scale 1.5] [--theme light|dark] [--open dropdown,modal,collapse,navbar,offcanvas] [--tab profile-pane] [--switch]\n--dump <boxes.txt> : laid-out boxes (same options); --title-bar : show the window title bar in an export
--report : compatibility of bootstrap.css"
                );
                return Ok(());
            }
            _ => native.push(arg),
        }
    }
    let options = WindowOptions::from_args(native)?;
    let (width, height) = size.unwrap_or((1280, 900));
    let preview = BenchmarkOptions {
        width,
        height,
        scale: scale.unwrap_or(1.5),
        title_bar: false,
        reduced_motion: true,
        renderer: options.renderer,
        ..Default::default()
    };
    if let Some(path) = dump {
        // Boîtes placées (classes et bornes logiques), pour comparer au navigateur.
        let text = diagnostics::inspect_preview(preview, demo.clone(), |root| {
            let mut out = String::new();
            dump_boxes(root, 0, &mut out);
            out
        })?;
        std::fs::write(&path, text).map_err(Error::application)?;
        if export.is_none() {
            return Ok(());
        }
    }
    if let Some(path) = export {
        let image = diagnostics::render_preview(preview, demo)?;
        diagnostics::write_png(&image, std::path::Path::new(&path))?;
        println!("{path} ({} × {})", image.width, image.height);
        return Ok(());
    }
    // Fenêtre interactive : barre de titre de la page, aux couleurs du thème Bootstrap
    // choisi, à la place de la barre générique du moteur.
    demo.chrome = true;
    platform::run(
        WindowOptions {
            title: "Bootstrap 5.3 · karbon-ui",
            width: width as i32,
            height: height as i32,
            title_bar: false,
            light: demo.theme == "light",
            ..options
        },
        demo,
    )
}
/// Une ligne par boîte visible : profondeur, classes, bornes logiques.
fn dump_boxes(node: &karbon_ui::ui::element::Element, depth: usize, out: &mut String) {
    let r = node.bounds();
    if r.right > r.left && r.bottom > r.top {
        out.push_str(&format!(
            "{:indent$}{} {:.1} {:.1} {:.1} {:.1}
",
            "",
            node.classes().join("."),
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            indent = depth
        ));
    }
    for child in &node.children {
        dump_boxes(child, depth + 1, out);
    }
}
