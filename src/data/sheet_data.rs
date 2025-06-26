mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use super::SheetData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::SheetObject)]
    pub struct SheetObject {
        #[property(name = "path", get, set, type = PathBuf, member = path)]
        #[property(name = "depth", get, set, type = u32, member = depth)]
        #[property(name = "stem", get, set, type = String, member = stem)]
        pub data: RefCell<SheetData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SheetObject {
        const NAME: &'static str = "LibrarySheetObject";
        type Type = super::SheetObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SheetObject {}
}

use std::path::PathBuf;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct SheetObject(ObjectSubclass<imp::SheetObject>);
}

impl SheetObject {
    pub fn new(path: PathBuf, depth: u32) -> Self {
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        Object::builder()
            .property("path", path)
            .property("depth", depth)
            .property("stem", stem)
            .build()
    }
}

#[derive(Default, Debug)]
pub struct SheetData {
    pub path: PathBuf,
    pub depth: u32,
    /// Use for display
    pub stem: String,
}
