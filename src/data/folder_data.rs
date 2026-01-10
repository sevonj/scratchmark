mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use gtk::glib;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use glib::Properties;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::FolderObject)]
    pub struct FolderObject {
        #[property(get, set)]
        pub(super) path: RefCell<PathBuf>,
        #[property(get, set)]
        pub(super) depth: Cell<u32>,
        #[property(get, set)]
        pub(super) name: RefCell<String>,
        #[property(get, set)]
        pub(super) is_selected: Cell<bool>,
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

use crate::error::ScratchmarkError;
use crate::util::file_actions;

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

    pub fn rename(&self, path: PathBuf) -> Result<(), ScratchmarkError> {
        if self.is_root() {
            return Err(ScratchmarkError::IsRootDir);
        }
        if !path.parent().is_some_and(|p| p.is_dir()) {
            return Err(ScratchmarkError::InvalidPath);
        }
        self.emit_by_name::<()>("rename-requested", &[&path]);
        Ok(())
    }

    pub fn trash(&self) -> Result<(), ScratchmarkError> {
        if self.is_root() {
            return Err(ScratchmarkError::IsRootDir);
        }
        self.emit_by_name::<()>("trash-requested", &[]);
        Ok(())
    }

    pub fn delete(&self) -> Result<(), ScratchmarkError> {
        if self.is_root() {
            return Err(ScratchmarkError::IsRootDir);
        }
        self.emit_by_name::<()>("delete-requested", &[]);
        Ok(())
    }

    pub fn close_project(&self) -> Result<(), ScratchmarkError> {
        if !self.is_root() {
            return Err(ScratchmarkError::NotRootDir);
        }
        self.emit_by_name::<()>("close-project-requested", &[]);
        Ok(())
    }

    pub fn create_subfolder(&self) -> Result<(), ScratchmarkError> {
        let path = file_actions::untitled_folder_path(self.path());
        file_actions::create_folder(&path)?;
        self.emit_by_name::<()>("subfolder-created", &[&path]);
        Ok(())
    }

    pub fn create_document(&self) -> Result<(), ScratchmarkError> {
        let path = file_actions::untitled_document_path(self.path());
        file_actions::create_document(&path)?;
        self.emit_by_name::<()>("document-created", &[&path]);
        Ok(())
    }

    pub fn notify_err(&self, msg: &str) {
        self.emit_by_name::<()>("notify-err", &[&msg]);
    }
}

#[cfg(test)]
mod tests {

    use gtk::glib::closure_local;

    use super::*;

    const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

    const ROOT: u32 = 0;
    const NOT_ROOT: u32 = 1;

    #[test]
    fn test_move_valid_path() {
        std::fs::create_dir_all(PathBuf::from(PROJECT_ROOT).join("test")).unwrap();
        let folder = FolderObject::new("path/to/".into(), NOT_ROOT);
        assert!(
            folder
                .rename(PathBuf::from(PROJECT_ROOT).join("test").join("new_folder"))
                .is_ok()
        );
    }

    #[test]
    fn test_move_invalid_path_noparent() {
        let folder = FolderObject::new("path/to/".into(), NOT_ROOT);
        folder.connect_closure(
            "rename-requested",
            false,
            closure_local!(move |_folder: FolderObject, _path: PathBuf| {
                assert!(false, "Signal emitted");
            }),
        );
        assert_eq!(
            folder.rename(PathBuf::from("/")),
            Err(ScratchmarkError::InvalidPath)
        );
    }

    #[test]
    fn test_cant_move_if_root() {
        let folder = FolderObject::new("path/to/".into(), ROOT);
        folder.connect_closure(
            "rename-requested",
            false,
            closure_local!(move |_folder: FolderObject, _path: PathBuf| {
                assert!(false, "Signal emitted");
            }),
        );
        assert_eq!(
            folder.rename("path/to2/".into()),
            Err(ScratchmarkError::IsRootDir)
        );
    }

    #[test]
    fn test_cant_trash_if_root() {
        let folder = FolderObject::new("path/to/".into(), ROOT);
        folder.connect_closure(
            "trash-requested",
            false,
            closure_local!(move |_folder: FolderObject| {
                assert!(false, "Signal emitted");
            }),
        );
        assert_eq!(folder.trash(), Err(ScratchmarkError::IsRootDir));
    }

    #[test]
    fn test_cant_delete_if_root() {
        let folder = FolderObject::new("path/to/".into(), ROOT);
        folder.connect_closure(
            "delete-requested",
            false,
            closure_local!(move |_folder: FolderObject| {
                assert!(false, "Signal emitted");
            }),
        );
        assert_eq!(folder.delete(), Err(ScratchmarkError::IsRootDir));
    }
}
