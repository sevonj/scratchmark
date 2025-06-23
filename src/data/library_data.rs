mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use super::LibraryData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::LibraryObject)]
    pub struct LibraryObject {
        #[property(name = "path", get, set, type = PathBuf, member = path)]
        #[property(name = "displayName", get, set, type = String, member = display_name)]
        pub data: RefCell<LibraryData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryObject {
        const NAME: &'static str = "LibraryObject";
        type Type = super::LibraryObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for LibraryObject {}
}

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct LibraryObject(ObjectSubclass<imp::LibraryObject>);
}

impl LibraryObject {
    pub fn new(path: PathBuf) -> Self {
        let display_name = path.file_name().unwrap().to_string_lossy().into_owned();
        Object::builder()
            .property("path", path)
            .property("displayName", display_name)
            .build()
    }

    pub fn data(&self) -> std::cell::Ref<'_, LibraryData> {
        self.imp().data.borrow()
    }

    pub fn refresh_content(&self) {
        let imp = self.imp();

        self.prune();

        let library_path = imp.data.borrow().path.clone();
        let mut search_stack = vec![library_path];
        let mut found_folders = vec![];
        let mut found_files = vec![];

        loop {
            let Some(folder) = search_stack.pop() else {
                break;
            };
            let Ok(entries) = folder.read_dir() else {
                continue;
            };
            for entry in entries {
                let Ok(entry) = entry else {
                    continue;
                };
                let Ok(metadata) = entry.metadata() else {
                    continue;
                };
                if metadata.is_dir() {
                    search_stack.push(entry.path());
                    found_folders.push(entry.path());
                } else {
                    found_files.push(entry.path());
                }
            }
        }

        let mut data = imp.data.borrow_mut();
        for path in found_folders {
            if data.folders.contains_key(&path) {
                continue;
            }
            data.folders.insert(path, FolderState::default());
        }

        for path in found_files {
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                continue;
            }
            if data.sheets.contains(&path) {
                continue;
            }
            data.sheets.insert(path);
        }
    }

    fn prune(&self) {
        let mut dead_folders = vec![];
        for (path, _) in self.imp().data.borrow_mut().folders.iter() {
            if !path.exists() {
                dead_folders.push(path.clone());
            }
        }
        for path in dead_folders {
            self.imp().data.borrow_mut().folders.remove(&path);
        }
        let mut dead_sheets = vec![];
        for path in self.imp().data.borrow_mut().sheets.iter() {
            if !path.exists() {
                dead_sheets.push(path.clone());
            }
        }
        for path in dead_sheets {
            self.imp().data.borrow_mut().sheets.remove(&path);
        }
    }
}

#[derive(Default, Debug)]
pub struct FolderState {
    pub expanded: bool,
}

impl FolderState {
    pub fn expanded() -> Self {
        Self { expanded: true }
    }
}

#[derive(Default, Debug)]
pub struct LibraryData {
    pub path: PathBuf,
    pub display_name: String,
    /// Every folder in the library, except root
    pub folders: HashMap<PathBuf, FolderState>,
    /// Every sheet in the library
    pub sheets: HashSet<PathBuf>,
}
