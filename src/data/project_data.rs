mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use super::ProjectData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ProjectObject)]
    pub struct ProjectObject {
        #[property(name = "path", get, set, type = PathBuf, member = path)]
        #[property(name = "displayName", get, set, type = String, member = display_name)]
        pub data: RefCell<ProjectData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectObject {
        const NAME: &'static str = "ProjectObject";
        type Type = super::ProjectObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectObject {}
}

use std::collections::HashMap;
use std::path::PathBuf;

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct ProjectObject(ObjectSubclass<imp::ProjectObject>);
}

impl ProjectObject {
    pub fn new(path: PathBuf) -> Self {
        let display_name = path.file_name().unwrap().to_string_lossy().into_owned();
        Object::builder()
            .property("path", path)
            .property("displayName", display_name)
            .build()
    }

    pub fn data(&self) -> std::cell::Ref<'_, ProjectData> {
        self.imp().data.borrow()
    }

    pub fn refresh_content(&self) {
        let imp = self.imp();

        self.prune();

        let root_path = imp.data.borrow().path.clone();
        let mut search_stack: Vec<(PathBuf, u32)> = vec![(root_path, 0)];
        let mut found_folders: Vec<(PathBuf, u32)> = vec![];
        let mut found_files: Vec<(PathBuf, u32)> = vec![];

        loop {
            let Some((folder, depth)) = search_stack.pop() else {
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
                    search_stack.push((entry.path(), depth + 1));
                    found_folders.push((entry.path(), depth + 1));
                } else {
                    found_files.push((entry.path(), depth + 1));
                }
            }
        }

        let mut data = imp.data.borrow_mut();
        for (path, depth) in found_folders {
            if data.folders.contains_key(&path) {
                continue;
            }
            data.folders.insert(path, FolderState::new(depth));
        }

        for (path, depth) in found_files {
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                continue;
            }
            if data.sheets.contains_key(&path) {
                continue;
            }
            data.sheets.insert(path, SheetState::new(depth));
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
        for (path, _) in self.imp().data.borrow_mut().sheets.iter() {
            if !path.exists() {
                dead_sheets.push(path.clone());
            }
        }
        for path in dead_sheets {
            self.imp().data.borrow_mut().sheets.remove(&path);
        }
    }
}

#[derive(Debug)]
pub struct FolderState {
    pub depth: u32,
}

impl FolderState {
    pub fn new(depth: u32) -> Self {
        Self { depth }
    }
}

#[derive(Debug)]
pub struct SheetState {
    pub depth: u32,
}

impl SheetState {
    pub fn new(depth: u32) -> Self {
        Self { depth }
    }
}

#[derive(Default, Debug)]
pub struct ProjectData {
    pub path: PathBuf,
    pub display_name: String,
    /// Every folder in the project, except root
    pub folders: HashMap<PathBuf, FolderState>,
    /// Every sheet in the project
    pub sheets: HashMap<PathBuf, SheetState>,
}
