mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use super::FolderData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::FolderObject)]
    pub struct FolderObject {
        #[property(name = "path", get, set, type = PathBuf, member = path)]
        #[property(name = "name", get, set, type = String, member = name)]
        pub data: RefCell<FolderData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FolderObject {
        const NAME: &'static str = "LibraryFolderObject";
        type Type = super::FolderObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FolderObject {}
}

use std::{fs::DirEntry, path::PathBuf};

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct FolderObject(ObjectSubclass<imp::FolderObject>);
}

impl FolderObject {
    pub fn new(path: PathBuf) -> Self {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        Object::builder()
            .property("path", path)
            .property("name", name)
            .build()
    }

    pub fn content(&self) -> Vec<DirEntry> {
        let mut entries = vec![];

        let Ok(readdir) = self.path().read_dir() else {
            return entries;
        };

        for entry in readdir.flatten() {
            entries.push(entry);
        }

        entries
    }
}

#[derive(Default, Debug)]
pub struct FolderData {
    pub path: PathBuf,
    /// Use for display
    pub name: String,
}
