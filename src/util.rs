use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use gtk::gio::prelude::*;

use gtk::gio::Cancellable;
use gtk::gio::File;
use gtk::glib::GString;

use gtk::glib::user_data_dir;

use crate::APP_ID;
use crate::error::ScratchmarkError;

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
    let path = user_data_dir().join(APP_ID);
    std::fs::create_dir_all(&path).expect("Couldn't create dir for userdata");
    path
}

/// Library directory inside userdata
pub fn path_builtin_library() -> PathBuf {
    path_userdata().join("library")
}

/// Create if doesn't exist
pub fn create_builtin_library() {
    let path = path_builtin_library();
    std::fs::create_dir_all(&path).expect("Couldn't create dir for builtin_library");
}

/// Returns an unused filepath with a placeholder name
pub fn untitled_folder_path(dir: PathBuf) -> PathBuf {
    assert!(dir.is_dir());
    let path = dir.join("New folder");
    incremented_path(path)
}

/// Returns an unused filepath with a placeholder name
pub fn untitled_document_path(dir: PathBuf) -> PathBuf {
    assert!(dir.is_dir());
    let path = dir.join("Untitled.md");
    incremented_path(path)
}

/// Increments filename until if finds an unused path.
/// "filename.md" becomes:
/// "filename.md",
/// or "filename (2).md",
/// or "filename (3).md", ...
/// Also works for folders.
pub fn incremented_path(path: PathBuf) -> PathBuf {
    assert!(path.parent().is_some_and(|p| p.is_dir()));
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().unwrap().to_string_lossy();
    let ext = path.extension().map(|e| e.to_string_lossy());

    let mut attempt = 2;
    loop {
        let mut new_name = format!("{stem} ({attempt})");
        if let Some(ext) = &ext {
            new_name = format!("{new_name}.{ext}");
        }
        let mut new_path = path.clone();
        new_path.set_file_name(new_name);
        if !new_path.exists() {
            return new_path;
        }
        attempt += 1;
    }
}

pub fn create_folder(path: &Path) -> Result<(), ScratchmarkError> {
    if let Err(e) = std::fs::create_dir(path) {
        println!("{e}");
        return Err(ScratchmarkError::FolderCreateFail);
    }
    Ok(())
}

pub fn create_document(path: &Path) -> Result<(), ScratchmarkError> {
    let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(f) => f,
        Err(e) => {
            println!("{e}");
            return Err(ScratchmarkError::FileCreateFail);
        }
    };
    let stem = path.file_stem().unwrap().to_string_lossy();
    let contents = format!("# {stem}\n\n");
    if let Err(e) = file.write_all(contents.as_bytes()) {
        println!("{e}");
        return Err(ScratchmarkError::FileCreateFail);
    }
    Ok(())
}

pub fn read_file_to_string(file: &File) -> Result<GString, ScratchmarkError> {
    let slice = match FileExtManual::load_contents(file, None::<&Cancellable>) {
        Ok((slice, _)) => slice,
        Err(_) => return Err(ScratchmarkError::FileOpenFail),
    };
    let text = match GString::from_utf8_checked(slice.to_vec()) {
        Ok(text) => text,
        Err(_) => return Err(ScratchmarkError::InvalidChars),
    };
    Ok(text)
}

pub fn move_folder(original_path: &Path, new_path: &Path) -> Result<(), ScratchmarkError> {
    if std::fs::exists(new_path).unwrap() {
        println!("Move folder: already exists")
    }
    if let Err(e) = copy_folder_recurse(original_path, new_path) {
        println!("{e}");
        return Err(ScratchmarkError::FolderMoveFail);
    };
    if let Err(e) = std::fs::remove_dir_all(original_path) {
        println!("{e}");
        return Err(ScratchmarkError::FolderMoveFail);
    };
    Ok(())
}

fn copy_folder_recurse(original_path: &Path, new_path: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(new_path)?;
    for entry in std::fs::read_dir(original_path)? {
        let entry = entry?;
        let entry_dest = PathBuf::from(new_path).join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_folder_recurse(&entry.path(), &entry_dest)?;
        } else {
            std::fs::copy(entry.path(), &entry_dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = env!("CARGO_MANIFEST_DIR");

    fn test_root() -> PathBuf {
        PathBuf::from(ROOT).join("test")
    }

    #[test]
    fn test_incremented_path() {
        let dir = test_root().join("file_increment");
        std::fs::create_dir_all(&dir).unwrap();

        let files = vec![
            "new file.md",
            "new file (2).md",
            "new file (3).md",
            "new file (4).md",
            // "new file (5).md",
            "new file (6).md",
            // "new file (7).md",
        ];

        let folders = vec![
            "new folder",
            "new folder (2)",
            // "new folder (3)",
            "new folder (4)",
        ];

        for file in &files {
            let path = dir.join(file);
            OpenOptions::new()
                .write(true)
                .create(true)
                .open(path)
                .unwrap();
        }
        for folder in &folders {
            let path = dir.join(folder);
            if path.is_dir() {
                continue;
            }
            std::fs::create_dir(path).unwrap();
        }

        for file in &files {
            assert!(dir.join(file).is_file());
        }
        for folder in &folders {
            assert!(dir.join(folder).is_dir());
        }

        let result_file = incremented_path(dir.join("new file.md"));
        let expected_file = dir.join("new file (5).md");
        let result_folder = incremented_path(dir.join("new folder"));
        let expected_folder = dir.join("new folder (3)");

        assert!(!expected_file.exists());
        assert!(!expected_folder.exists());

        assert_eq!(result_file, expected_file);
        assert_eq!(result_folder, expected_folder);
    }
}
