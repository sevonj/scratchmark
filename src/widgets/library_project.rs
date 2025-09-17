mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::closure_local;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;

    use crate::data::FolderObject;
    use crate::data::ProjectObject;
    use crate::data::SheetObject;
    use crate::widgets::LibraryFolder;
    use crate::widgets::LibrarySheet;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library_project.ui")]
    pub struct LibraryProject {
        #[template_child]
        pub(super) project_root_vbox: TemplateChild<gtk::Box>,

        pub(super) folders: RefCell<HashMap<PathBuf, LibraryFolder>>,
        pub(super) sheets: RefCell<HashMap<PathBuf, LibrarySheet>>,
        pub(super) project_object: RefCell<Option<ProjectObject>>,

        /// Is this a builtin project (drafts)
        pub(super) is_builtin: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryProject {
        const NAME: &'static str = "LibraryProject";
        type Type = super::LibraryProject;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibraryProject {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("folder-added")
                        .param_types([LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("sheet-added")
                        .param_types([LibrarySheet::static_type()])
                        .build(),
                    Signal::builder("sheet-selected")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-rename-requested")
                        .param_types([LibraryFolder::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("sheet-rename-requested")
                        .param_types([LibrarySheet::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-delete-requested")
                        .param_types([LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("sheet-delete-requested")
                        .param_types([LibrarySheet::static_type()])
                        .build(),
                    Signal::builder("folder-trash-requested")
                        .param_types([LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("sheet-trash-requested")
                        .param_types([LibrarySheet::static_type()])
                        .build(),
                    Signal::builder("close-project-requested").build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryProject {}
    impl BinImpl for LibraryProject {}

    impl LibraryProject {
        pub(super) fn setup_root(&self, root_folder: LibraryFolder) {
            let path = root_folder.path();
            self.project_object
                .replace(Some(ProjectObject::new(path.clone())));
            self.project_object
                .borrow()
                .as_ref()
                .unwrap()
                .refresh_content();

            let vbox = &self.project_root_vbox;

            vbox.append(&root_folder);
            self.connect_folder(root_folder.clone());

            self.folders.borrow_mut().insert(path, root_folder);
        }

        pub(super) fn refresh_content(&self) {
            self.project_object
                .borrow()
                .as_ref()
                .unwrap()
                .refresh_content();

            self.prune();

            let binding = self.project_object.borrow();
            let lib = binding.as_ref().unwrap();

            // Add new
            let mut added_folders = vec![];
            for (path, folder_state) in lib.data().folders.iter() {
                if !self.folders.borrow().contains_key(path) {
                    let folder =
                        LibraryFolder::new(&FolderObject::new(path.clone(), folder_state.depth));
                    self.connect_folder(folder.clone());
                    added_folders.push(folder.clone());
                    self.folders.borrow_mut().insert(path.clone(), folder);
                }
            }
            for folder in &added_folders {
                let path = folder.path();
                let parent_path = path.parent().unwrap();
                let parent = self.folders.borrow().get(parent_path).unwrap().clone();
                parent.add_subfolder(folder.clone());
            }

            for (path, sheet_state) in lib.data().sheets.iter() {
                if !self.sheets.borrow().contains_key(path) {
                    let sheet =
                        LibrarySheet::new(&SheetObject::new(path.clone(), sheet_state.depth));
                    self.obj().emit_by_name::<()>("sheet-added", &[&sheet]);

                    self.sheets.borrow_mut().insert(path.clone(), sheet.clone());

                    let parent_path = path.parent().unwrap();
                    let parent = self.folders.borrow().get(parent_path).unwrap().clone();
                    parent.add_sheet(sheet);
                }
            }
        }

        /// Remove widgets for entries that don't exist in the library anymore
        fn prune(&self) {
            let binding = self.project_object.borrow();
            let project = binding.as_ref().unwrap();

            let mut folders = self.folders.borrow_mut();
            let mut sheets = self.sheets.borrow_mut();
            let mut dead_folders = vec![];
            let mut dead_sheets = vec![];
            for (path, folder) in folders.iter() {
                // Do not delete library root
                if *path == project.path() {
                    continue;
                }
                if !project.data().folders.contains_key(path) {
                    dead_folders.push(path.clone());

                    let parent_path = path.parent().unwrap();
                    let parent = folders.get(parent_path).unwrap().clone();
                    parent.remove_subfolder(folder)
                }
            }
            for (path, sheet) in sheets.iter() {
                if !project.data().sheets.contains_key(path) {
                    dead_sheets.push(path.clone());

                    let parent_path = path.parent().unwrap();
                    let parent = folders.get(parent_path).unwrap().clone();
                    parent.remove_sheet(sheet)
                }
            }
            for path in dead_folders {
                folders
                    .remove(&path)
                    .expect("dead folder entry disappeared?");
            }
            for path in dead_sheets {
                sheets.remove(&path).expect("dead sheet entry disappeared?");
            }
        }

        fn connect_folder(&self, folder: LibraryFolder) {
            self.obj().emit_by_name::<()>("folder-added", &[&folder]);

            folder.connect_closure(
                "folder-created",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryFolder, _path: PathBuf| {
                        this.refresh_content();
                    }
                ),
            );

            folder.connect_closure(
                "sheet-created",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryFolder, _path: PathBuf| {
                        this.refresh_content();
                    }
                ),
            );
        }
    }
}

use std::path::{Path, PathBuf};

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::closure_local;
use sourceview5::prelude::*;

use glib::Object;

use crate::data::FolderObject;
use crate::util::path_builtin_library;

use crate::widgets::LibraryFolder;
use crate::widgets::LibrarySheet;

glib::wrapper! {
    pub struct LibraryProject(ObjectSubclass<imp::LibraryProject>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LibraryProject {
    /// New standard project
    pub fn new(path: PathBuf) -> Self {
        let this: Self = Object::builder().build();
        this.imp().is_builtin.replace(false);
        let root = LibraryFolder::new_project_root(&FolderObject::new(path.clone(), 0));
        root.connect_closure(
            "close-project-requested",
            false,
            closure_local!(
                #[weak]
                this,
                move |_: LibraryFolder| {
                    this.emit_by_name::<()>("close-project-requested", &[]);
                }
            ),
        );
        this.imp().setup_root(root);
        this
    }

    /// Builtin drafts project
    pub fn new_draft_table() -> Self {
        let this: Self = Object::builder().build();
        this.imp().is_builtin.replace(true);
        let root = LibraryFolder::new_drafts_root(&FolderObject::new(path_builtin_library(), 0));
        this.imp().setup_root(root);
        this
    }

    pub fn path(&self) -> PathBuf {
        self.root_folder().path()
    }

    pub fn root_folder(&self) -> LibraryFolder {
        let path = self.imp().project_object.borrow().as_ref().unwrap().path();
        self.imp().folders.borrow().get(&path).unwrap().clone()
    }

    pub fn is_builtin(&self) -> bool {
        self.imp().is_builtin.get()
    }

    pub fn expanded_folder_paths(&self) -> Vec<String> {
        let mut paths = vec![];
        for (path, folder) in self.imp().folders.borrow().iter() {
            if folder.is_expanded() {
                paths.push(path.to_str().unwrap().to_owned());
            }
        }
        paths
    }

    pub fn get_folder(&self, path: &Path) -> Option<LibraryFolder> {
        self.imp().folders.borrow().get(path).cloned()
    }

    pub fn get_sheet(&self, path: &Path) -> Option<LibrarySheet> {
        self.imp().sheets.borrow().get(path).cloned()
    }

    pub fn refresh_content(&self) {
        self.imp().refresh_content();
    }
}
