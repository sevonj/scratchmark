use std::error::Error;

#[derive(Debug)]
pub enum ScratchmarkError {
    FileCreateFail,
    FileOpenFail,
    FolderCreateFail,
    FolderMoveFail,
    InvalidChars,
    FileChanged,
}

impl Error for ScratchmarkError {}

impl std::fmt::Display for ScratchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileCreateFail => write!(f, "Couldn't create file."),
            Self::FileOpenFail => write!(f, "Couldn't read file."),
            Self::FolderCreateFail => write!(f, "Couldn't create folder."),
            Self::FolderMoveFail => write!(f, "Couldn't move folder."),
            Self::InvalidChars => write!(f, "File contains invalid characters."),
            Self::FileChanged => write!(f, "File has changed on disk."),
        }
    }
}
