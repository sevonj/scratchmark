use std::error::Error;

#[derive(Debug, PartialEq, Eq)]
pub enum ScratchmarkError {
    FileCreateFail,
    FileOpenFail,
    FolderCreateFail,
    ItemMoveFail,
    InvalidChars,
    FileChanged,
    IsRootDir,
    NotRootDir,
    InvalidPath,
}

impl Error for ScratchmarkError {}

impl std::fmt::Display for ScratchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileCreateFail => write!(f, "Couldn't create file."),
            Self::FileOpenFail => write!(f, "Couldn't read file."),
            Self::FolderCreateFail => write!(f, "Couldn't create folder."),
            Self::ItemMoveFail => write!(f, "Couldn't move item."),
            Self::InvalidChars => write!(f, "File contains invalid characters."),
            Self::FileChanged => write!(f, "File has changed on disk."),
            Self::IsRootDir => write!(f, "This action can't be done to a project root folder."),
            Self::NotRootDir => write!(f, "This action can only be done to a project root folder."),
            Self::InvalidPath => write!(f, "Invalid path."),
        }
    }
}
