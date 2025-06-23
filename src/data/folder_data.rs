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
        #[property(name = "root", get, set, type = bool, member = root)]
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
    pub fn new(path: PathBuf, root: bool) -> Self {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        Object::builder()
            .property("path", path)
            .property("root", root)
            .property("name", name)
            .build()
    }
}

#[derive(Default, Debug)]
pub struct FolderData {
    pub path: PathBuf,
    /// Root dir of library
    pub root: bool,
    /// Use for display
    pub name: String,
}
