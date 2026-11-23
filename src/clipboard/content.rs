//! Ce que l'application met dans le presse-papiers et ce qu'elle en relit ; le système est
//! appelé par `platform::windows::app_clipboard` (ailleurs : non pris en charge).

/// What to put on the clipboard: a text that every program reads, and any number of
/// blocks of bytes in formats named by the application (« karbon-editor/objets »). They
/// are written together and replace what the clipboard held.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClipboardContent {
    /// Plain text, readable by any program; `None` writes no text.
    pub text: Option<String>,
    /// Blocks of bytes in the application's own formats, by format name. A name is any
    /// non-empty text; the system registers it the first time (two programs that use the
    /// same name exchange their data).
    pub formats: Vec<(String, Vec<u8>)>,
}

impl ClipboardContent {
    /// A clipboard content made of `text` alone.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            formats: Vec::new(),
        }
    }

    /// The same content with `bytes` in the format named `format` as well.
    pub fn with(mut self, format: impl Into<String>, bytes: Vec<u8>) -> Self {
        self.formats.push((format.into(), bytes));
        self
    }

    /// Vérifie ce qui sera écrit : des noms de format non vides et sans caractère nul,
    /// au plus 64 formats, au plus 256 Mio en tout.
    pub(crate) fn check(&self) -> crate::Result<()> {
        if self.formats.len() > 64 {
            return Err(invalid("more than 64 clipboard formats"));
        }
        let mut total = self.text.as_ref().map_or(0, String::len);
        for (name, bytes) in &self.formats {
            if name.is_empty() || name.contains('\0') {
                return Err(invalid("a clipboard format name is empty or contains NUL"));
            }
            total += bytes.len();
        }
        if total > 256 * 1024 * 1024 {
            return Err(invalid("clipboard content larger than 256 MiB"));
        }
        Ok(())
    }
}

fn invalid(reason: &'static str) -> crate::Error {
    crate::Error::invalid_input(reason)
}

/// Replace what the clipboard holds with `content`: its text and its formats, together.
/// Fails when the clipboard is held by another program for too long, or when the
/// content is invalid (an empty format name, more than 64 formats or 256 MiB).
pub fn write(content: &ClipboardContent) -> crate::Result<()> {
    content.check()?;
    #[cfg(windows)]
    {
        crate::platform::app_clipboard::write(content)
    }
    #[cfg(not(windows))]
    {
        Err(unsupported())
    }
}

/// Replace what the clipboard holds with `text`.
pub fn write_text(text: &str) -> crate::Result<()> {
    write(&ClipboardContent::text(text))
}

/// The text on the clipboard, or `None` when it holds no text.
pub fn read_text() -> crate::Result<Option<String>> {
    #[cfg(windows)]
    {
        crate::platform::app_clipboard::read_text()
    }
    #[cfg(not(windows))]
    {
        Err(unsupported())
    }
}

/// The bytes on the clipboard in the format named `format`, or `None` when it holds
/// none in that format (another program copied something since).
pub fn read_format(format: &str) -> crate::Result<Option<Vec<u8>>> {
    if format.is_empty() || format.contains('\0') {
        return Err(invalid("a clipboard format name is empty or contains NUL"));
    }
    #[cfg(windows)]
    {
        crate::platform::app_clipboard::read_format(format)
    }
    #[cfg(not(windows))]
    {
        Err(unsupported())
    }
}

#[cfg(not(windows))]
fn unsupported() -> crate::Error {
    crate::Error::Unsupported {
        reason: "the application clipboard is only implemented on Windows",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_content_is_checked_before_it_is_written() {
        let fine = ClipboardContent::text("Caisse").with("karbon/objets", vec![1, 2, 3]);
        assert!(fine.check().is_ok());
        assert_eq!(fine.formats.len(), 1);
        // Témoins : un nom vide, un nom avec un caractère nul, trop de formats.
        assert!(
            ClipboardContent::default()
                .with("", vec![])
                .check()
                .is_err()
        );
        assert!(
            ClipboardContent::default()
                .with("a\0b", vec![])
                .check()
                .is_err()
        );
        let mut many = ClipboardContent::default();
        for k in 0..65 {
            many = many.with(format!("f{k}"), vec![]);
        }
        assert!(many.check().is_err());
        assert!(read_format("").is_err());
    }
}
