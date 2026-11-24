//! Les données que porte un glisser : une sorte, un nom à montrer, une valeur typée.
use std::{any::Any, fmt, path::PathBuf, rc::Rc, sync::Arc};

/// What a dragged element carries: a kind that targets test before accepting it (« a
/// texture », « a model », « files »), a label to show in a preview, and a value of any
/// type, read back with [`get`](Self::get). Cheap to clone.
///
/// ```
/// use karbon_ui::ui::drag::Payload;
///
/// #[derive(Debug, PartialEq)]
/// struct Asset(u64);
/// let payload = Payload::new("texture", "Brique rouge", Asset(42));
/// assert!(payload.is("texture"));
/// assert_eq!(payload.get::<Asset>(), Some(&Asset(42)));
/// assert_eq!(payload.get::<String>(), None);
/// ```
#[derive(Clone)]
pub struct Payload {
    kind: Arc<str>,
    label: Arc<str>,
    value: Rc<dyn Any>,
    /// Fichiers qui la représentent hors de la fenêtre (`exported_as`).
    exported: Option<Arc<[PathBuf]>>,
}

impl Payload {
    /// Data of kind `kind`, shown as `label`, carrying `value`.
    pub fn new<T: 'static>(kind: &str, label: impl Into<Arc<str>>, value: T) -> Self {
        Self {
            kind: kind.into(),
            label: label.into(),
            value: Rc::new(value),
            exported: None,
        }
    }

    /// Let the payload leave the window as these files: dragged past the edge of the
    /// window, it becomes a drag of files of the system, which the Explorer or another
    /// program can take (the Explorer copies them where they are dropped). The source
    /// then learns the end of the drag as usual, `dropped` when a program took the
    /// files. Windows only; elsewhere the drag stays in the window.
    pub fn exported_as(mut self, files: Vec<PathBuf>) -> Self {
        self.exported = (!files.is_empty()).then(|| Arc::from(files));
        self
    }

    /// The files given to [`exported_as`](Self::exported_as).
    pub fn exported(&self) -> Option<&[PathBuf]> {
        self.exported.as_deref()
    }

    /// Files, of kind [`FILES`](Self::FILES), labelled by the name of the first one; the
    /// value is the `Vec<PathBuf>`.
    pub fn files(paths: Vec<PathBuf>) -> Self {
        let label = paths
            .first()
            .and_then(|p| p.file_name())
            .map_or(String::new(), |n| n.to_string_lossy().into_owned());
        Self::new(Self::FILES, label, paths)
    }

    /// The kind of [`files`](Self::files).
    pub const FILES: &'static str = "files";

    /// The kind given at creation.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Whether the kind is `kind`.
    pub fn is(&self, kind: &str) -> bool {
        &*self.kind == kind
    }

    /// The label given at creation.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The value, if it is a `T`.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }
}

impl fmt::Debug for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Payload")
            .field("kind", &self.kind)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl PartialEq for Payload {
    /// Same kind, same label, and the very same value (not a copy of it).
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.label == other.label
            && Rc::ptr_eq(&self.value, &other.value)
    }
}
