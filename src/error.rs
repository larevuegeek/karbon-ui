// Type d'erreur de la bibliothèque et alias `Result`.
use crate::task::TaskError;
use crate::window::WindowId;
use std::{borrow::Cow, fmt, io};

/// `Result` with [`Error`] as the default error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error returned by the fallible APIs of this crate.
///
/// New variants may be added in minor releases; match with a wildcard arm.
///
/// [`Error`] converts to and from [`std::io::Error`], so `?` keeps working in functions
/// returning `std::io::Result`:
///
/// ```no_run
/// use karbon_ui::{platform, ui::prelude::*, window::WindowOptions};
/// # struct App;
/// # impl Application for App {
/// #     type Message = ();
/// #     fn view(&self, _: &ViewContext) -> Element { text("t", "") }
/// #     fn update(&mut self, _: ()) -> bool { false }
/// # }
/// fn main() -> std::io::Result<()> {
///     let options = WindowOptions::from_args(std::env::args().skip(1))?;
///     platform::run(options, App)?;
///     Ok(())
/// }
/// ```
///
/// Callers that need to react to a specific failure match on the variants:
///
/// ```
/// use karbon_ui::{Error, window::WindowOptions};
///
/// match WindowOptions::from_args(["--bogus".to_string()]) {
///     Err(Error::InvalidWindowOptions { reason }) => eprintln!("usage error: {reason}"),
///     Err(other) => eprintln!("{other}"),
///     Ok(_) => {}
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A value passed to the library is invalid: an element tree that fails layout
    /// validation, out-of-range style, scene, widget or benchmark values, and similar
    /// programming errors.
    InvalidInput {
        /// Description of the first problem found.
        reason: Cow<'static, str>,
    },
    /// [`WindowOptions`](crate::window::WindowOptions) are inconsistent, or a
    /// command-line flag given to
    /// [`WindowOptions::from_args`](crate::window::WindowOptions::from_args) is unknown.
    InvalidWindowOptions {
        /// Description of the problem.
        reason: Cow<'static, str>,
    },
    /// Two windows share the same [`WindowId`], or a window with this ID is already open.
    DuplicateWindowId(WindowId),
    /// An event loop is already running on the calling thread.
    AlreadyRunning,
    /// The operation is not available: no backend exists for this platform, or the
    /// feature is not implemented.
    Unsupported {
        /// What is not supported.
        reason: &'static str,
    },
    /// A native OS call failed.
    Platform {
        /// Name of the failing native operation.
        operation: &'static str,
        /// `HRESULT` of the failure; Win32 error codes are converted with
        /// `HRESULT_FROM_WIN32`.
        code: i32,
    },
    /// The GPU device was removed or reset (driver update, crash, adapter change).
    ///
    /// Windows recover from it by recreating their renderer; the error only reaches the
    /// application when the recovery fails too.
    DeviceLost {
        /// Name of the native operation that observed the loss.
        operation: &'static str,
        /// `HRESULT` reported by the device (for example `DXGI_ERROR_DEVICE_REMOVED`).
        code: i32,
    },
    /// An operation did not complete within its deadline.
    TimedOut {
        /// Name of the operation that timed out.
        operation: &'static str,
    },
    /// The operation was cancelled through its
    /// [`CancellationToken`](crate::task::CancellationToken).
    Cancelled,
    /// An image could not be loaded, decoded or built.
    Image(ImageError),
    /// A background task produced no value.
    Task(TaskError),
    /// Application code panicked while a window message was handled; the window was
    /// closed.
    Panicked {
        /// The panic payload when it is a string, otherwise a placeholder.
        message: String,
    },
    /// Error returned by application code, for example from
    /// [`Application::load`](crate::application::Application::load). Build it with
    /// [`Error::application`].
    Application(Box<dyn std::error::Error + Send + Sync>),
    /// File or stream I/O failed.
    Io(io::Error),
}

impl Error {
    /// Wrap an error produced by application code, so that it can be returned from
    /// [`Application::load`](crate::application::Application::load) and other callbacks
    /// that return [`Result`].
    ///
    /// ```
    /// use karbon_ui::Error;
    ///
    /// fn parse_port(text: &str) -> karbon_ui::Result<u16> {
    ///     text.parse().map_err(Error::application)
    /// }
    /// assert!(matches!(parse_port("x"), Err(Error::Application(_))));
    /// ```
    pub fn application(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Application(error.into())
    }
    /// [`Error::InvalidInput`] with `reason`.
    pub(crate) fn invalid_input(reason: impl Into<Cow<'static, str>>) -> Self {
        Self::InvalidInput {
            reason: reason.into(),
        }
    }
    /// [`Error::InvalidWindowOptions`] with `reason`.
    pub(crate) fn invalid_window_options(reason: impl Into<Cow<'static, str>>) -> Self {
        Self::InvalidWindowOptions {
            reason: reason.into(),
        }
    }
    /// The [`std::io::ErrorKind`] closest to this error, used by the conversion to
    /// [`std::io::Error`].
    pub fn io_kind(&self) -> io::ErrorKind {
        match self {
            Self::InvalidInput { .. } | Self::InvalidWindowOptions { .. } => {
                io::ErrorKind::InvalidInput
            }
            Self::DuplicateWindowId(_) | Self::AlreadyRunning => io::ErrorKind::AlreadyExists,
            Self::Unsupported { .. } => io::ErrorKind::Unsupported,
            Self::TimedOut { .. } => io::ErrorKind::TimedOut,
            Self::Cancelled => io::ErrorKind::Interrupted,
            Self::Image(_) => io::ErrorKind::InvalidData,
            Self::Io(error) => error.kind(),
            Self::Platform { .. }
            | Self::DeviceLost { .. }
            | Self::Task(_)
            | Self::Panicked { .. }
            | Self::Application(_) => io::ErrorKind::Other,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput { reason } => f.write_str(reason),
            Self::InvalidWindowOptions { reason } => write!(f, "invalid window options: {reason}"),
            Self::DuplicateWindowId(id) => write!(f, "duplicate window id {}", id.0),
            Self::AlreadyRunning => f.write_str("an event loop is already running on this thread"),
            Self::Unsupported { reason } => write!(f, "unsupported: {reason}"),
            Self::Platform { operation, code } => {
                write!(f, "{operation} failed (HRESULT 0x{:08X})", *code as u32)
            }
            Self::DeviceLost { operation, code } => write!(
                f,
                "graphics device lost during {operation} (HRESULT 0x{:08X})",
                *code as u32
            ),
            Self::TimedOut { operation } => write!(f, "{operation} timed out"),
            Self::Cancelled => f.write_str("operation cancelled"),
            Self::Image(error) => error.fmt(f),
            Self::Task(error) => error.fmt(f),
            Self::Panicked { message } => {
                write!(
                    f,
                    "application panicked while handling a window message: {message}"
                )
            }
            // Erreurs enveloppées : leur message est repris tel quel (source() le prolonge).
            Self::Application(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    /// The wrapped error's own source: wrapped errors are transparent, their message is
    /// already part of this error's [`Display`](fmt::Display) output.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Image(error) => error.source(),
            Self::Task(error) => error.source(),
            Self::Application(error) => error.source(),
            Self::Io(error) => error.source(),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    /// Wrap an I/O error; an `io::Error` that itself wraps an [`Error`] is unwrapped.
    fn from(error: io::Error) -> Self {
        if error.get_ref().is_some_and(|inner| inner.is::<Error>()) {
            if let Some(inner) = error.into_inner()
                && let Ok(inner) = inner.downcast::<Error>()
            {
                return *inner;
            }
            unreachable!("io::Error wrapping a karbon_ui::Error")
        }
        Self::Io(error)
    }
}
impl From<Error> for io::Error {
    /// Convert to an `io::Error` of kind [`Error::io_kind`]; an [`Error::Io`] is returned
    /// unchanged, any other error is kept as the inner error.
    fn from(error: Error) -> Self {
        match error {
            Error::Io(error) => error,
            error => io::Error::new(error.io_kind(), error),
        }
    }
}
impl From<ImageError> for Error {
    fn from(error: ImageError) -> Self {
        Self::Image(error)
    }
}
impl From<TaskError> for Error {
    fn from(error: TaskError) -> Self {
        Self::Task(error)
    }
}

/// Reason an image could not be loaded, decoded or built; carried by [`Error::Image`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageError {
    /// The data is neither PNG nor JPEG.
    UnsupportedFormat,
    /// The encoded data is truncated or inconsistent.
    Malformed {
        /// What is wrong with the data.
        reason: &'static str,
    },
    /// The file, the source dimensions or the decoded image exceed the configured limits.
    TooLarge {
        /// Which limit was exceeded.
        reason: &'static str,
    },
    /// The [`ImageLoadOptions`](crate::resources::ImageLoadOptions) limits are zero or
    /// above their maximum.
    InvalidLimits,
    /// The image has a zero dimension, its pixel buffer does not hold
    /// `width * height * 4` bytes, or a colour channel exceeds its alpha (the pixels are
    /// not premultiplied).
    InvalidPixels,
    /// The native decoder rejected the data.
    Decode,
}
impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat => {
                f.write_str("unsupported image format, expected PNG or JPEG")
            }
            Self::Malformed { reason } => write!(f, "malformed image: {reason}"),
            Self::TooLarge { reason } => write!(f, "image too large: {reason}"),
            Self::InvalidLimits => f.write_str("invalid image loading limits"),
            Self::InvalidPixels => f.write_str(
                "invalid image pixels, expected width * height premultiplied BGRA pixels",
            ),
            Self::Decode => f.write_str("image data could not be decoded"),
        }
    }
}
impl std::error::Error for ImageError {}

/// Vrai si `result` est un refus de validation ([`Error::InvalidInput`]).
#[cfg(test)]
pub(crate) fn rejected<T>(result: Result<T>) -> bool {
    matches!(result, Err(Error::InvalidInput { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_thread_safe_and_round_trip_through_io_errors() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<Error>();
        let io: io::Error = Error::invalid_input("bad value").into();
        assert_eq!(io.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(io.to_string(), "bad value");
        assert!(matches!(
            Error::from(io),
            Error::InvalidInput { reason } if reason == "bad value"
        ));
        let original = io::Error::new(io::ErrorKind::NotFound, "missing");
        let error = Error::from(original);
        assert!(matches!(&error, Error::Io(e) if e.kind() == io::ErrorKind::NotFound));
        assert_eq!(io::Error::from(error).kind(), io::ErrorKind::NotFound);
        assert_eq!(
            io::Error::from(Error::Cancelled).kind(),
            io::ErrorKind::Interrupted
        );
    }
    #[test]
    fn messages_are_lowercase_english_without_trailing_period() {
        let messages = [
            Error::Platform {
                operation: "CreateWindowExW",
                code: 0x8007_0005_u32 as i32,
            }
            .to_string(),
            Error::DeviceLost {
                operation: "Present",
                code: 0x887A_0005_u32 as i32,
            }
            .to_string(),
            Error::DuplicateWindowId(WindowId(2)).to_string(),
            Error::Image(ImageError::TooLarge {
                reason: "file exceeds the size limit",
            })
            .to_string(),
            Error::Task(TaskError::QueueFull).to_string(),
            Error::Panicked {
                message: "boom".into(),
            }
            .to_string(),
        ];
        assert_eq!(messages[0], "CreateWindowExW failed (HRESULT 0x80070005)");
        assert_eq!(
            messages[1],
            "graphics device lost during Present (HRESULT 0x887A0005)"
        );
        for message in &messages {
            assert!(!message.ends_with('.'), "{message}");
        }
        // Le premier message commence par le nom de l'opération native.
        for message in &messages[1..] {
            assert!(!message.starts_with(char::is_uppercase), "{message}");
        }
    }
    #[test]
    fn application_errors_keep_their_message_and_source_chain() {
        #[derive(Debug)]
        struct Outer(io::Error);
        impl fmt::Display for Outer {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("config unreadable")
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        let error = Error::application(Outer(io::Error::other("disk")));
        assert_eq!(error.to_string(), "config unreadable");
        let source = std::error::Error::source(&error).unwrap();
        assert_eq!(source.to_string(), "disk");
        assert!(matches!(
            Error::application("plain text"),
            Error::Application(_)
        ));
    }
}
