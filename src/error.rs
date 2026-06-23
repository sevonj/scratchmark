use std::error::Error;

use gettextrs::gettext;

#[derive(Debug, PartialEq, Eq)]
pub enum ScratchmarkError {
    FileCreateFail,
    FileOpenFail,
    FolderCreateFail,
    ItemMoveFail,
    InvalidChars,
    FileChanged,
    InvalidPath,
    IsRootDir,
    NotRootDir,
}

impl Error for ScratchmarkError {}

impl std::fmt::Display for ScratchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ScratchmarkError::*;

        match self {
            FileCreateFail => write!(f, "{}", gettext("Couldn't create file")),
            FileOpenFail => write!(f, "{}", gettext("Couldn't access file")),
            FolderCreateFail => write!(f, "{}", gettext("Couldn't create folder")),
            ItemMoveFail => write!(f, "{}", gettext("Couldn't move item")),
            InvalidChars => write!(f, "{}", gettext("File contains invalid characters")),
            FileChanged => write!(f, "{}", gettext("File has changed on disk")),
            InvalidPath => write!(f, "{}", gettext("Invalid path")),
            IsRootDir => write!(f, "This action can't be done to a project root folder"),
            NotRootDir => write!(f, "This action can only be done to a project root folder"),
        }
    }
}
