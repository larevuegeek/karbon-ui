//! Fichiers lus relativement au fichier source de l'appel (`.ui`, `.css`, `.svg`),
//! suivis par Cargo (`include_bytes!`), et littéraux de chaîne Rust décodés.
use crate::{Error, Result};
use proc_macro::{Span, TokenStream};
use std::path::{Path, PathBuf};
#[derive(Clone)]
pub struct Source {
    pub display: String,
    pub dir: PathBuf,
    pub package: PathBuf,
}
impl Source {
    pub fn call(input: &TokenStream) -> Self {
        let span = input
            .clone()
            .into_iter()
            .next()
            .map_or(Span::call_site(), |t| t.span());
        let package = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
        let file = span
            .local_file()
            .unwrap_or_else(|| package.join("src/lib.rs"));
        Self {
            display: span.file(),
            dir: file.parent().unwrap_or(&package).to_path_buf(),
            package,
        }
    }
}
pub struct Reader {
    pub source: Source,
    pub stack: Vec<PathBuf>,
    pub dependencies: Vec<PathBuf>,
    bytes: usize,
}
impl Reader {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            stack: Vec::new(),
            dependencies: Vec::new(),
            bytes: 0,
        }
    }
    pub fn text(&mut self, path: &str, span: Span) -> Result<(String, Source, PathBuf)> {
        let target = self.source.dir.join(path).canonicalize().map_err(|e| {
            Error::new(
                format!("{}: include {path:?}: {e}", self.source.display),
                span,
            )
        })?;
        let package = self
            .source
            .package
            .canonicalize()
            .map_err(|e| Error::new(e.to_string(), span))?;
        if !target.starts_with(&package) {
            return Err(Error::new(
                "Template includes must stay inside their Cargo package",
                span,
            ));
        }
        if self.stack.contains(&target) {
            return Err(Error::new(
                format!("Circular template include: {}", target.display()),
                span,
            ));
        }
        if self.stack.len() >= 32 {
            return Err(Error::new("Template include depth exceeds 32", span));
        }
        let text = std::fs::read_to_string(&target).map_err(|e| {
            Error::new(
                format!("Cannot read template {}: {e}", target.display()),
                span,
            )
        })?;
        // Marque d'ordre des octets (fichier enregistré « UTF-8 avec BOM ») et fins de ligne
        // Windows : le même fichier se compile de la même façon quel que soit l'éditeur.
        let text = text
            .strip_prefix('\u{feff}')
            .unwrap_or(&text)
            .replace("\r\n", "\n");
        self.bytes += text.len();
        if self.bytes > 2 * 1024 * 1024 {
            return Err(Error::new("Templates exceed the 2 MiB source limit", span));
        }
        let display = target
            .strip_prefix(&package)
            .unwrap_or(&target)
            .display()
            .to_string();
        let source = Source {
            display,
            dir: target.parent().unwrap().to_path_buf(),
            package,
        };
        self.dependencies.push(target.clone());
        Ok((text, source, target))
    }
}
pub fn string(value: &str) -> std::result::Result<String, String> {
    if let Some(raw) = value.strip_prefix('r') {
        let at = raw.find('"').ok_or("Expected a quoted string")?;
        let hashes = &raw[..at];
        if hashes.chars().any(|c| c != '#') {
            return Err("Expected a string literal".into());
        }
        let end = format!("\"{hashes}");
        return raw[at + 1..]
            .strip_suffix(&end)
            .map(str::to_owned)
            .ok_or_else(|| "Invalid raw string".into());
    }
    let inner = value
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or("Expected a string literal")?;
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next().ok_or("Incomplete string escape")? {
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            '\'' => out.push('\''),
            'x' => {
                let a = chars
                    .next()
                    .and_then(|c| c.to_digit(16))
                    .ok_or("Invalid ASCII escape")?;
                let b = chars
                    .next()
                    .and_then(|c| c.to_digit(16))
                    .ok_or("Invalid ASCII escape")?;
                let n = a * 16 + b;
                if n > 127 {
                    return Err("ASCII escape must be below 0x80".into());
                }
                out.push(char::from_u32(n).unwrap());
            }
            '\n' => {
                while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
                    chars.next();
                }
            }
            'u' => {
                if chars.next() != Some('{') {
                    return Err("Invalid Unicode escape".into());
                }
                let mut hex = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(c) => hex.push(c),
                        None => return Err("Incomplete Unicode escape".into()),
                    }
                }
                let code = u32::from_str_radix(&hex.replace('_', ""), 16)
                    .map_err(|_| "Invalid Unicode escape")?;
                out.push(char::from_u32(code).ok_or("Invalid Unicode scalar")?);
            }
            _ => {
                return Err(
                    "Use a raw string for paths with backslashes, or escape them as \\\\".into(),
                );
            }
        }
    }
    Ok(out)
}
pub fn dependency(path: &Path) -> String {
    format!(
        "const _: &str = include_str!({:?});",
        path.to_string_lossy()
    )
}
