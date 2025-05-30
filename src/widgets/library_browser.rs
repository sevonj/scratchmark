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

            let vbox = &self.library_root_vbox;
            let root_folder =
                LibraryFolder::new_root(&FolderObject::new(path_builtin_library(), true));
            vbox.append(&root_folder);
            self.add_folder(root_folder);
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
        fn add_folder(&self, folder: LibraryFolder) {
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
                "folder-added",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryFolder, button: LibraryFolder| {
                        this.add_folder(button);
                    }
                ),
            );

            folder.connect_closure(
                "sheet-added",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryFolder, button: LibrarySheet| {
                        this.add_sheet(button);
                    }
                ),
            );

            folder.connect_closure(
                "folder-removed",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryFolder, path: PathBuf| {
                        this.unlist_folder(path);
                    }
                ),
            );

            folder.connect_closure(
                "sheet-removed",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryFolder, path: PathBuf| {
                        this.unlist_sheet(path);
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

            folder.refresh_content();
            let k = folder.path();
            self.folders.borrow_mut().insert(k, folder);
        }

        fn add_sheet(&self, sheet: LibrarySheet) {
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

        fn unlist_folder(&self, path: PathBuf) {
            self.folders.borrow_mut().remove(&path);
        }

        fn unlist_sheet(&self, path: PathBuf) {
            self.sheets.borrow_mut().remove(&path);
        }
    }
}

use std::path::PathBuf;

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

    pub fn refresh_content(&self) {
        self.root_folder().refresh_content();
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
}
