//! RustMusic home page rebuilt from its Svelte + Tailwind sources with the compiled
//! HTML templates (`.ui` + CSS). Static data, no audio. See docs/rustmusic-html.md.
#[path = "rustmusic_html/mod.rs"]
mod music;
use karbon_ui::{
    Error,
    diagnostics::{self, BenchmarkOptions},
    platform,
    window::WindowOptions,
};
fn main() -> karbon_ui::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut native = Vec::new();
    let (mut export, mut dump, mut size, mut scale) = (None, None, None, None);
    let (mut menu, mut pointer, mut collapsed) = (None, None, false);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--hover" => {
                let value = args
                    .next()
                    .ok_or_else(|| Error::application("--hover expects X,Y (logical px)"))?;
                let (x, y) = value
                    .split_once(',')
                    .ok_or_else(|| Error::application("example: --hover 433,676"))?;
                let parse = |v: &str| v.trim().parse::<f32>().map_err(Error::application);
                pointer = Some((parse(x)?, parse(y)?));
            }
            "--export" => {
                export = Some(
                    args.next()
                        .ok_or_else(|| Error::application("--export expects a PNG path"))?,
                )
            }
            "--dump" => {
                dump = Some(
                    args.next()
                        .ok_or_else(|| Error::application("--dump expects a JSON path"))?,
                )
            }
            "--size" => {
                let value = args
                    .next()
                    .ok_or_else(|| Error::application("--size expects WIDTHxHEIGHT"))?;
                let (w, h) = value
                    .split_once('x')
                    .ok_or_else(|| Error::application("example: --size 1440x1040"))?;
                let parse = |v: &str| v.parse::<u32>().map_err(Error::application);
                size = Some((parse(w)?, parse(h)?));
            }
            "--scale" => {
                let value = args
                    .next()
                    .ok_or_else(|| Error::application("--scale expects a factor, e.g. 1.5"))?;
                scale = Some(value.parse::<f32>().map_err(Error::application)?);
            }
            "--open" => {
                let value = args
                    .next()
                    .ok_or_else(|| Error::application("--open expects a menu name"))?;
                menu = Some(music::Menu::parse(&value).ok_or_else(|| {
                    let names = music::Menu::NAMES.map(|(n, _)| n).join("|");
                    Error::application(format!("--open {value}: expected one of {names}"))
                })?);
            }
            "--sidebar" => {
                collapsed = match args.next().as_deref() {
                    Some("collapsed") => true,
                    Some("expanded") => false,
                    _ => {
                        return Err(Error::application(
                            "--sidebar expects collapsed or expanded",
                        ));
                    }
                };
            }
            "--help" => {
                let names = music::Menu::NAMES.map(|(n, _)| n).join("|");
                println!(
                    "RustMusic · accueil en templates HTML\n--export <image.png> [--size 1440x1040] [--scale 1.5] [--open {names}] [--sidebar collapsed|expanded] [--hover X,Y]\n--dump <layout.json> (mêmes options)\n--size LxH (unités logiques) --renderer=auto|gpu|software --present=… --profile\nDonnées statiques, sans lecture audio."
                );
                return Ok(());
            }
            _ => native.push(arg),
        }
    }
    let options = WindowOptions::from_args(native)?;
    let (width, height) = size.unwrap_or((1440, 1040));
    if let Some(path) = dump {
        // Bornes mesurées par le moteur natif, comme l'export.
        let json = diagnostics::inspect_preview(
            BenchmarkOptions {
                width,
                height,
                scale: scale.unwrap_or(1.5),
                title_bar: false,
                reduced_motion: true,
                renderer: options.renderer,
                pointer,
                ..Default::default()
            },
            music::MusicApp::with_menu(menu).with_sidebar(collapsed),
            music::dump::layout_json,
        )?;
        std::fs::write(&path, json).map_err(Error::application)?;
        println!("Mise en page : {path}");
        if export.is_none() {
            return Ok(());
        }
    }
    if let Some(path) = export {
        // 1440 × 1040 logical at 150 % gives the 2160 × 1556 pixels of the reference.
        let scale = scale.unwrap_or(1.5);
        let image = diagnostics::render_preview(
            BenchmarkOptions {
                width,
                height,
                scale,
                title_bar: false,
                reduced_motion: true,
                renderer: options.renderer,
                pointer,
                ..Default::default()
            },
            music::MusicApp::with_menu(menu).with_sidebar(collapsed),
        )?;
        diagnostics::write_png(&image, std::path::Path::new(&path))?;
        println!("Aperçu : {path} ({} × {})", image.width, image.height);
        return Ok(());
    }
    platform::run(
        WindowOptions {
            title: "RustMusic · templates HTML",
            width: width as i32,
            height: height as i32,
            min_width: 1100,
            min_height: 760,
            title_bar: false,
            ..options
        },
        music::MusicApp::with_menu(menu).with_sidebar(collapsed),
    )
}
