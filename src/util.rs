use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use gtk::glib;

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

/// Returns the first free filepath in series of
/// "/path/to/New folder",
/// "/path/to/New folder (2)",
/// "/path/to/New folder (3)", ...
pub fn untitled_folder_path(dir: PathBuf) -> PathBuf {
    assert!(dir.is_dir());
    let path = dir.join("New folder");
    if !path.exists() {
        return path;
    }
    let mut attempt = 2;
    loop {
        let filename = format!("New folder ({attempt})");
        let path = dir.join(filename);
        if !path.exists() {
            return path;
        }
        attempt += 1;
    }
}

/// Returns the first free filepath in series of
/// "/path/to/Untitled.md",
/// "/path/to/Untitled (2).md",
/// "/path/to/Untitled (3).md", ...
pub fn untitled_sheet_path(dir: PathBuf) -> PathBuf {
    assert!(dir.is_dir());
    let path = dir.join("Untitled.md");
    if !path.exists() {
        return path;
    }
    let mut attempt = 2;
    loop {
        let filename = format!("Untitled ({attempt}).md");
        let path = dir.join(filename);
        if !path.exists() {
            return path;
        }
        attempt += 1;
    }
}

pub fn create_folder(path: &Path) {
    std::fs::create_dir(path).expect("folder create fail");
}

pub fn create_sheet_file(path: &Path) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .expect("file create fail");

    let stem = path.file_stem().unwrap().to_string_lossy();
    let contents = format!("# {stem}\n\n");
    file.write_all(contents.as_bytes())
        .expect("failed to write template to new file");
}
