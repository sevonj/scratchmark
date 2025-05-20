use gtk::glib;
use std::path::PathBuf;

use crate::APP_ID;

#[derive(Debug)]
pub enum FilenameStatus {
    Ok,
    IsEmpty,
    HasIllegalChars,
}

impl From<&str> for FilenameStatus {
    fn from(stem: &str) -> Self {
        if stem.is_empty() {
            return FilenameStatus::IsEmpty;
        }
        if stem.contains("/") {
            return FilenameStatus::HasIllegalChars;
        }
        FilenameStatus::Ok
    }
}

impl FilenameStatus {
    pub fn is_ok(&self) -> bool {
        match self {
            Self::Ok => true,
            Self::IsEmpty => false,
            Self::HasIllegalChars => false,
        }
    }

    pub fn complaint_message(&self) -> Option<&str> {
        match self {
            FilenameStatus::Ok => None,
            FilenameStatus::IsEmpty => None,
            FilenameStatus::HasIllegalChars => Some("Invalid name"),
        }
    }
}

/// User data directory
pub fn path_userdata() -> PathBuf {
    let path = glib::user_data_dir().join(APP_ID);
    std::fs::create_dir_all(&path).expect("Couldn't create dir for userdata");
    path
}

/// Library directory inside userdata
pub fn path_builtin_library() -> PathBuf {
    let path = path_userdata().join("library");
    std::fs::create_dir_all(&path).expect("Couldn't create dir for builtin_library");
    path
}
