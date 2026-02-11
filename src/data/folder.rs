mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use std::time::SystemTime;

    use gtk::glib;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use gtk::glib::CollationKey;
    use gtk::glib::Properties;

    use super::FolderType;
    use crate::data::Document;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::Folder)]
    pub struct Folder {
        #[property(get, set)]
        pub(super) path: RefCell<PathBuf>,
        #[property(get, set)]
        pub(super) depth: Cell<u32>,
        #[property(get, set)]
        pub(super) name: RefCell<String>,
        #[property(get, set)]
        pub(super) is_selected: Cell<bool>,

        pub(super) subfolders: RefCell<HashMap<PathBuf, super::Folder>>,
        pub(super) documents: RefCell<HashMap<PathBuf, Document>>,

        pub(super) kind: OnceLock<FolderType>,
        pub(super) modified: RefCell<Option<SystemTime>>,
        pub(super) collation_key: OnceLock<CollationKey>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Folder {
        const NAME: &'static str = "Folder";
        type Type = super::Folder;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Folder {
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
                    Signal::builder("metadata-changed").build(),
                ]
            })
        }
    }
}

use std::cell::Ref;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib;
use gtk::glib::closure_local;

use gtk::glib::CollationKey;
use gtk::glib::Object;
use gtk::glib::object::ObjectExt;

use crate::error::ScratchmarkError;
use crate::util::file_actions;

use super::Document;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderType {
    Subfolder,
    ProjectRoot,
    DraftsRoot,
}

glib::wrapper! {
    pub struct Folder(ObjectSubclass<imp::Folder>);
}

impl Folder {
    pub fn modified(&self) -> SystemTime {
        self.imp().modified.borrow().unwrap()
    }

    pub fn collation_key(&self) -> &CollationKey {
        self.imp().collation_key.get().unwrap()
    }

    pub fn new_subfolder(path: PathBuf, depth: u32, modified: SystemTime) -> Self {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let collation_key = CollationKey::from(&name);

        let obj: Self = Object::builder()
            .property("path", path)
            .property("depth", depth)
            .property("name", name)
            .build();

        let imp = obj.imp();
        imp.kind.set(FolderType::Subfolder).unwrap();
        imp.modified.replace(Some(modified));
        imp.collation_key.set(collation_key).unwrap();

        obj
    }

    pub fn new_project_root(path: PathBuf) -> Self {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let collation_key = CollationKey::from(&name);

        let obj: Self = Object::builder()
            .property("path", path)
            .property("depth", 0_u32)
            .property("name", name)
            .build();

        let imp = obj.imp();
        imp.kind.set(FolderType::ProjectRoot).unwrap();
        imp.modified.replace(Some(SystemTime::now()));
        imp.collation_key.set(collation_key).unwrap();

        obj
    }

    /// Special root folder for builtin drafts project
    pub fn new_drafts_root(path: PathBuf) -> Self {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let collation_key = CollationKey::from(&name);

        let obj: Self = Object::builder()
            .property("path", path)
            .property("depth", 0_u32)
            .property("name", "Drafts")
            .build();

        let imp = obj.imp();
        imp.kind.set(FolderType::DraftsRoot).unwrap();
        imp.modified.replace(Some(SystemTime::now()));
        imp.collation_key.set(collation_key).unwrap();

        obj
    }

    pub fn kind(&self) -> FolderType {
        *self.imp().kind.get().unwrap()
    }

    pub fn is_root(&self) -> bool {
        match self.kind() {
            FolderType::Subfolder => false,
            FolderType::ProjectRoot | FolderType::DraftsRoot => true,
        }
    }

    pub fn documents(&self) -> Ref<'_, HashMap<PathBuf, Document>> {
        self.imp().documents.borrow()
    }

    pub fn subfolders(&self) -> Ref<'_, HashMap<PathBuf, Folder>> {
        self.imp().subfolders.borrow()
    }

    pub fn add_document(&self, doc: Document) {
        self.on_child_modified_changed(doc.modified());
        doc.connect_closure(
            "metadata-changed",
            true,
            closure_local!(
                #[weak(rename_to = obj)]
                self,
                move |doc: Document| {
                    obj.on_child_modified_changed(doc.modified());
                }
            ),
        );
        self.imp().documents.borrow_mut().insert(doc.path(), doc);
    }

    pub fn add_subfolder(&self, folder: Folder) {
        self.on_child_modified_changed(folder.modified());
        folder.connect_closure(
            "metadata-changed",
            true,
            closure_local!(
                #[weak(rename_to = obj)]
                self,
                move |folder: Folder| {
                    obj.on_child_modified_changed(folder.modified());
                }
            ),
        );
        self.imp()
            .subfolders
            .borrow_mut()
            .insert(folder.path(), folder);
    }

    pub fn remove_document(&self, path: &Path) {
        self.imp().documents.borrow_mut().remove(path);
    }

    pub fn remove_subfolder(&self, path: &Path) {
        self.imp().subfolders.borrow_mut().remove(path);
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

    pub fn create_subfolder<P: AsRef<Path>>(&self, name: P) -> Result<(), ScratchmarkError> {
        let path = file_actions::incremented_path(self.path().join(name));
        file_actions::create_folder(&path)?;
        self.emit_by_name::<()>("subfolder-created", &[&path]);
        Ok(())
    }

    pub fn create_subfolder_unnamed(&self) -> Result<(), ScratchmarkError> {
        self.create_subfolder("New folder")
    }

    pub fn create_document<P: AsRef<Path>>(&self, name: P) -> Result<(), ScratchmarkError> {
        let path = file_actions::incremented_path(self.path().join(name));
        file_actions::create_document(&path)?;
        self.emit_by_name::<()>("document-created", &[&path]);
        Ok(())
    }

    pub fn create_document_untitled(&self) -> Result<(), ScratchmarkError> {
        self.create_document("Untitled.md")
    }

    pub fn notify_err(&self, msg: &str) {
        self.emit_by_name::<()>("notify-err", &[&msg]);
    }

    fn on_child_modified_changed(&self, child_modified: SystemTime) {
        if child_modified > self.modified() {
            self.imp().modified.borrow_mut().replace(child_modified);
            self.emit_by_name::<()>("metadata-changed", &[]);
        }
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
        let folder = Folder::new_subfolder("path/to/".into(), 1, SystemTime::now());
        assert!(
            folder
                .rename(PathBuf::from(PROJECT_ROOT).join("test").join("new_folder"))
                .is_ok()
        );
    }

    #[test]
    fn test_move_invalid_path_noparent() {
        let folder = Folder::new_subfolder("path/to/".into(), 1, SystemTime::now());
        folder.connect_closure(
            "rename-requested",
            false,
            closure_local!(move |_folder: Folder, _path: PathBuf| {
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
        let folder = Folder::new_project_root("path/to/".into());
        folder.connect_closure(
            "rename-requested",
            false,
            closure_local!(move |_folder: Folder, _path: PathBuf| {
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
        let folder = Folder::new_project_root("path/to/".into());
        folder.connect_closure(
            "trash-requested",
            false,
            closure_local!(move |_folder: Folder| {
                assert!(false, "Signal emitted");
            }),
        );
        assert_eq!(folder.trash(), Err(ScratchmarkError::IsRootDir));
    }

    #[test]
    fn test_cant_delete_if_root() {
        let folder = Folder::new_project_root("path/to/".into());
        folder.connect_closure(
            "delete-requested",
            false,
            closure_local!(move |_folder: Folder| {
                assert!(false, "Signal emitted");
            }),
        );
        assert_eq!(folder.delete(), Err(ScratchmarkError::IsRootDir));
    }
}
