mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use gtk::glib;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use gtk::glib::Properties;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::DocumentObject)]
    pub struct DocumentObject {
        #[property(get, set)]
        pub(super) path: RefCell<PathBuf>,
        #[property(get, set)]
        pub(super) depth: RefCell<u32>,
        #[property(get, set)]
        pub(super) stem: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DocumentObject {
        const NAME: &'static str = "DocumentObject";
        type Type = super::DocumentObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for DocumentObject {
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
use crate::util;

glib::wrapper! {
    pub struct DocumentObject(ObjectSubclass<imp::DocumentObject>);
}

impl DocumentObject {
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
        let dupe_path = util::incremented_path(self_path);
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
        let doc = DocumentObject::new("path/to/".into(), 1);
        assert!(
            doc.rename(PathBuf::from(PROJECT_ROOT).join("test").join("new_file.md"))
                .is_ok()
        );
    }

    #[test]
    fn test_move_invalid_path_noparent() {
        let doc = DocumentObject::new("path/to/".into(), 1);
        doc.connect_closure(
            "rename-requested",
            false,
            closure_local!(move |_doc: DocumentObject, _path: PathBuf| {
                assert!(false, "Signal emitted");
            }),
        );
        assert_eq!(
            doc.rename(PathBuf::from("/")),
            Err(ScratchmarkError::InvalidPath)
        );
    }
}
