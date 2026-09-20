use std::fmt;
use std::path::Path;

/// Diagnostics from one generation attempt. Display includes source locations.
#[derive(Debug)]
pub struct Error(pub(crate) Vec<String>);

impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(vec![message.into()])
    }

    pub(crate) fn io(path: &Path, error: std::io::Error) -> Self {
        Self::new(format!("{}: {error}", path.display()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join("\n"))
    }
}

impl std::error::Error for Error {}
