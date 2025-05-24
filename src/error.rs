use std::error::Error;

#[derive(Debug)]
pub enum TheftMDError {
    FileOpenFail,
    InvalidChars,
    FileChanged,
}

impl Error for TheftMDError {}

impl std::fmt::Display for TheftMDError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileOpenFail => write!(f, "Failed to read file."),
            Self::InvalidChars => write!(f, "File contains invalid characters."),
            Self::FileChanged => write!(f, "File has changed on disk."),
        }
    }
}
