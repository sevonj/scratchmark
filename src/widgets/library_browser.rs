//! Library browser is located in the left sidebar.
//!

mod imp {
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
    use crate::data::LibraryObject;
    use crate::data::SheetObject;
    use crate::util::path_builtin_library;
    use crate::widgets::LibraryFolder;
    use crate::widgets::LibrarySheet;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library_browser.ui")]
    pub struct LibraryBrowser {
        #[template_child]
        pub(super) library_root_vbox: TemplateChild<gtk::Box>,

        pub(super) folders: RefCell<HashMap<PathBuf, LibraryFolder>>,
        pub(super) sheets: RefCell<HashMap<PathBuf, LibrarySheet>>,
        pub(super) selected_sheet: RefCell<Option<PathBuf>>,
        pub(super) library_object: RefCell<Option<LibraryObject>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryBrowser {
        const NAME: &'static str = "LibraryBrowser";
        type Type = super::LibraryBrowser;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibraryBrowser {
        fn constructed(&self) {
            self.parent_constructed();
            self.library_object
                .replace(Some(LibraryObject::new(path_builtin_library())));
            self.library_object
                .borrow()
                .as_ref()
                .unwrap()
                .refresh_content();

            let vbox = &self.library_root_vbox;
            let root_folder =
                LibraryFolder::new_root(&FolderObject::new(path_builtin_library(), true));
            vbox.append(&root_folder);
            self.connect_folder(root_folder);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
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
                ]
            })
        }
    }

    impl WidgetImpl for LibraryBrowser {}
    impl BinImpl for LibraryBrowser {}

    impl LibraryBrowser {
        pub(super) fn refresh_content(&self) {
            self.library_object
                .borrow()
                .as_ref()
                .unwrap()
                .refresh_content();

            self.prune();

            let binding = self.library_object.borrow();
            let lib = binding.as_ref().unwrap();

            // Add new
            let mut added_folders = vec![];
            for (path, _folder_state) in lib.data().folders.iter() {
                if !self.folders.borrow().contains_key(path) {
                    let folder = LibraryFolder::new(&FolderObject::new(path.clone(), false));
                    self.connect_folder(folder.clone());
                    added_folders.push(folder);
                }
            }
            for folder in &added_folders {
                let path = folder.path();
                let parent_path = path.parent().unwrap();
                let parent = self.folders.borrow().get(parent_path).unwrap().clone();
                parent.add_subfolder(folder.clone());
            }

            for path in lib.data().sheets.iter() {
                if !self.sheets.borrow().contains_key(path) {
                    let sheet = LibrarySheet::new(&SheetObject::new(path.clone()));
                    self.connect_sheet(sheet.clone());

                    let parent_path = path.parent().unwrap();
                    let parent = self.folders.borrow().get(parent_path).unwrap().clone();
                    parent.add_sheet(sheet);
                }
            }
        }

        /// Remove widgets for entries that don't exist in the library anymore
        fn prune(&self) {
            let binding = self.library_object.borrow();
            let lib = binding.as_ref().unwrap();

            let mut folders = self.folders.borrow_mut();
            let mut sheets = self.sheets.borrow_mut();
            let mut dead_folders = vec![];
            let mut dead_sheets = vec![];
            for (path, folder) in folders.iter() {
                // Do not delete library root
                if *path == lib.path() {
                    continue;
                }
                if !lib.data().folders.contains_key(path) {
                    dead_folders.push(path.clone());

                    let parent_path = path.parent().unwrap();
                    let parent = folders.get(parent_path).unwrap().clone();
                    parent.remove_subfolder(folder)
                }
            }
            for (path, sheet) in sheets.iter() {
                if !lib.data().sheets.contains(path) {
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
            let obj = self.obj();

            folder.connect_closure(
                "rename-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |folder: LibraryFolder, new_path: PathBuf| {
                        obj.emit_by_name::<()>("folder-rename-requested", &[&folder, &new_path]);
                    }
                ),
            );

            folder.connect_closure(
                "sheet-created",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryFolder, path: PathBuf| {
                        obj.emit_by_name::<()>("sheet-selected", &[&path]);
                    }
                ),
            );

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

            folder.connect_closure(
                "trash-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryFolder, folder: LibraryFolder| {
                        obj.emit_by_name::<()>("folder-trash-requested", &[&folder]);
                    }
                ),
            );

            folder.connect_closure(
                "delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryFolder, folder: LibraryFolder| {
                        obj.emit_by_name::<()>("folder-delete-requested", &[&folder]);
                    }
                ),
            );

            let k = folder.path();
            self.folders.borrow_mut().insert(k, folder);
        }

        fn connect_sheet(&self, sheet: LibrarySheet) {
            let obj = self.obj();

            sheet.connect_closure(
                "selected",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |sheet: LibrarySheet| {
                        sheet.set_active(false);
                        let path = sheet.path();
                        obj.emit_by_name::<()>("sheet-selected", &[&path]);
                    }
                ),
            );

            sheet.connect_closure(
                "duplicated",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibrarySheet| {
                        obj.refresh_content();
                    }
                ),
            );

            sheet.connect_closure(
                "rename-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |sheet: LibrarySheet, new_path: PathBuf| {
                        obj.emit_by_name::<()>("sheet-rename-requested", &[&sheet, &new_path]);
                    }
                ),
            );

            sheet.connect_closure(
                "trash-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |button: LibrarySheet| {
                        obj.emit_by_name::<()>("sheet-trash-requested", &[&button]);
                    }
                ),
            );

            sheet.connect_closure(
                "delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |button: LibrarySheet| {
                        obj.emit_by_name::<()>("sheet-delete-requested", &[&button]);
                    }
                ),
            );

            if let Some(selected) = self.selected_sheet.borrow().as_ref() {
                if sheet.path() == *selected {
                    sheet.set_active(true);
                }
            }

            let k = sheet.path();
            self.sheets.borrow_mut().insert(k, sheet);
        }
    }
}

use std::path::{Path, PathBuf};

use adw::subclass::prelude::*;
use gtk::glib;

use glib::Object;

use crate::util::path_builtin_library;

use super::LibraryFolder;

glib::wrapper! {
    pub struct LibraryBrowser(ObjectSubclass<imp::LibraryBrowser>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for LibraryBrowser {
    fn default() -> Self {
        let this: Self = Object::builder().build();
        this.refresh_content();
        this
    }
}

impl LibraryBrowser {
    pub fn root_folder(&self) -> LibraryFolder {
        self.imp()
            .folders
            .borrow()
            .get(&path_builtin_library())
            .unwrap()
            .clone()
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

    pub fn refresh_content(&self) {
        self.imp().refresh_content();
    }

    pub fn selected_sheet(&self) -> Option<PathBuf> {
        self.imp().selected_sheet.borrow().clone()
    }

    pub fn set_selected_sheet(&self, path: Option<PathBuf>) {
        if let Some(old_path) = self.imp().selected_sheet.borrow().as_ref() {
            if let Some(old_button) = self.imp().sheets.borrow().get(old_path) {
                old_button.set_active(false);
            }
        }

        if let Some(path) = &path {
            if let Some(button) = self.imp().sheets.borrow().get(path) {
                button.set_active(true);
            }
        };

        self.imp().selected_sheet.replace(path);
    }

    pub fn rename_selected_sheet(&self) {
        let Some(selected_path) = self.selected_sheet() else {
            return;
        };

        if let Some(sheet) = self.imp().sheets.borrow().get(&selected_path) {
            sheet.prompt_rename();
        }
    }
}
