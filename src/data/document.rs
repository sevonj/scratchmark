mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use gtk::glib;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use gtk::glib::Properties;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::Document)]
    pub struct Document {
        #[property(get, set)]
        pub(super) path: RefCell<PathBuf>,
        #[property(get, set)]
        pub(super) depth: Cell<u32>,
        #[property(get, set)]
        pub(super) stem: RefCell<String>,
        #[property(get, set)]
        pub(super) is_selected: Cell<bool>,
        #[property(get, set)]
        pub(super) is_open_in_editor: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Document {
        const NAME: &'static str = "Document";
        type Type = super::Document;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Document {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("selected").build(),
                    Signal::builder("duplicated").build(),
                    Signal::builder("rename-requested")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("trash-requested").build(),
                    Signal::builder("delete-requested").build(),
                ]
            })
        }
    }
}

use std::path::PathBuf;

use gtk::glib;
use gtk::prelude::*;

use gtk::gio::Cancellable;
use gtk::gio::FileCopyFlags;
use gtk::glib::Object;
use gtk::glib::object::ObjectExt;

use crate::error::ScratchmarkError;
use crate::util::file_actions;

glib::wrapper! {
    pub struct Document(ObjectSubclass<imp::Document>);
}

impl Document {
    pub fn new(path: PathBuf, depth: u32) -> Self {
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        Object::builder()
            .property("path", path)
            .property("depth", depth)
            .property("stem", stem)
            .build()
    }

    pub fn select(&self) {
        self.emit_by_name::<()>("selected", &[]);
    }

    pub fn duplicate(&self) {
        self.emit_by_name::<()>("duplicated", &[]);

        let self_path = self.path();
        let self_file = gtk::gio::File::for_path(&self_path);
        let dupe_path = file_actions::incremented_path(self_path);
        let dupe_file = gtk::gio::File::for_path(&dupe_path);
        self_file
            .copy(&dupe_file, FileCopyFlags::NONE, None::<&Cancellable>, None)
            .expect("File dupe failed");
        self.emit_by_name::<()>("duplicated", &[]);
    }

    pub fn rename(&self, path: PathBuf) -> Result<(), ScratchmarkError> {
        if !path.parent().is_some_and(|p| p.is_dir()) {
            return Err(ScratchmarkError::InvalidPath);
        }
        self.emit_by_name::<()>("rename-requested", &[&path]);
        Ok(())
    }

    pub fn trash(&self) {
        self.emit_by_name::<()>("trash-requested", &[]);
    }

    pub fn delete(&self) {
        self.emit_by_name::<()>("delete-requested", &[]);
    }
}

#[cfg(test)]
mod tests {

    use gtk::glib::closure_local;

    use super::*;

    const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn test_move_valid_path() {
        std::fs::create_dir_all(PathBuf::from(PROJECT_ROOT).join("test")).unwrap();
        let doc = Document::new("path/to/".into(), 1);
        assert!(
            doc.rename(PathBuf::from(PROJECT_ROOT).join("test").join("new_file.md"))
                .is_ok()
        );
    }

    #[test]
    fn test_move_invalid_path_noparent() {
        let doc = Document::new("path/to/".into(), 1);
        doc.connect_closure(
            "rename-requested",
            false,
            closure_local!(move |_doc: Document, _path: PathBuf| {
                assert!(false, "Signal emitted");
            }),
        );
        assert_eq!(
            doc.rename(PathBuf::from("/")),
            Err(ScratchmarkError::InvalidPath)
        );
    }
}
