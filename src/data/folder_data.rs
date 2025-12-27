mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use gtk::glib;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use glib::Properties;

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
        const NAME: &'static str = "FolderObject";
        type Type = super::FolderObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FolderObject {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("selected").build(),
                    Signal::builder("rename-requested")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("trash-requested").build(),
                    Signal::builder("delete-requested").build(),
                    Signal::builder("close-project-requested").build(),
                    Signal::builder("subfolder-created")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("document-created")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    // Error that should be toasted to the user
                    Signal::builder("notify-err")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }
}

use std::path::PathBuf;

use gtk::glib;

use glib::Object;
use gtk::glib::object::ObjectExt;

use crate::util;

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

    pub fn select(&self) {
        self.emit_by_name::<()>("selected", &[]);
    }

    pub fn rename(&self, path: PathBuf) {
        assert!(!self.is_root());
        assert!(path.parent().is_some_and(|p| p.is_dir()));
        self.emit_by_name::<()>("rename-requested", &[&path]);
    }

    pub fn trash(&self) {
        assert!(!self.is_root());
        self.emit_by_name::<()>("trash-requested", &[]);
    }

    pub fn delete(&self) {
        assert!(!self.is_root());
        self.emit_by_name::<()>("delete-requested", &[]);
    }

    pub fn close_project(&self) {
        assert!(self.is_root());
        self.emit_by_name::<()>("close-project-requested", &[]);
    }

    pub fn create_subfolder(&self) {
        let path = util::untitled_folder_path(self.path());
        if let Err(e) = util::create_folder(&path) {
            self.emit_by_name::<()>("notify-err", &[&e.to_string()]);
            return;
        }
        self.emit_by_name::<()>("subfolder-created", &[&path]);
    }

    pub fn create_document(&self) {
        let path = util::untitled_document_path(self.path());
        if let Err(e) = util::create_document(&path) {
            self.emit_by_name::<()>("notify-err", &[&e.to_string()]);
            return;
        }
        self.emit_by_name::<()>("document-created", &[&path]);
    }
}

#[derive(Default, Debug)]
pub struct FolderData {
    pub path: PathBuf,
    pub depth: u32,
    /// Use for display
    pub name: String,
}
