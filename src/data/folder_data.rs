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
        #[property(name = "depth", get, set, type = u32, member = depth)]
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

use std::path::PathBuf;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct FolderObject(ObjectSubclass<imp::FolderObject>);
}

impl FolderObject {
    pub fn new(path: PathBuf, depth: u32) -> Self {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        Object::builder()
            .property("path", path)
            .property("depth", depth)
            .property("name", name)
            .build()
    }

    pub fn is_root(&self) -> bool {
        self.depth() == 0
    }
}

#[derive(Default, Debug)]
pub struct FolderData {
    pub path: PathBuf,
    pub depth: u32,
    /// Use for display
    pub name: String,
}
