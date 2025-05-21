//! Library browser is located in the left sidebar.
//!

mod imp {
    use std::cell::RefCell;
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
    use crate::widgets::LibrarySheetButton;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_browser.ui")]
    pub struct LibraryBrowser {
        #[template_child]
        pub(super) library_root: TemplateChild<gtk::Box>,

        pub(super) selected_sheet_button: RefCell<Option<glib::WeakRef<LibrarySheetButton>>>,
        pub(super) library_folder: RefCell<Option<LibraryFolder>>,
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
            let this = self;
            let obj = self.obj();

            let library_root = &self.library_root;

            let data = FolderObject::new(path_builtin_library());
            let folder = LibraryFolder::new_root(&data);
            folder.refresh_content();

            library_root.append(&folder);

            folder.connect_closure(
                "sheet-clicked",
                false,
                closure_local!(
                    #[weak]
                    this,
                    move |_folder: LibraryFolder, button: LibrarySheetButton| {
                        let path = button.path();
                        this.obj().emit_by_name::<()>("sheet-selected", &[&path]);
                    }
                ),
            );

            folder.connect_closure(
                "folder-delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryFolder, folder: LibraryFolder| {
                        obj.emit_by_name::<()>("folder-delete-requested", &[&folder]);
                    }
                ),
            );

            folder.connect_closure(
                "sheet-renamed",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryFolder, button: LibrarySheetButton, new_path: PathBuf| {
                        obj.emit_by_name::<()>("sheet-renamed", &[&button, &new_path]);
                    }
                ),
            );

            folder.connect_closure(
                "sheet-delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryFolder, sheet: LibrarySheetButton| {
                        obj.emit_by_name::<()>("sheet-delete-requested", &[&sheet]);
                    }
                ),
            );

            self.library_folder.replace(Some(folder));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("sheet-selected")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("sheet-renamed")
                        .param_types([LibrarySheetButton::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-delete-requested")
                        .param_types([LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("sheet-delete-requested")
                        .param_types([LibrarySheetButton::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryBrowser {}
    impl BinImpl for LibraryBrowser {}
}

use std::path::Path;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;

use super::LibrarySheetButton;

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
    pub fn refresh_content(&self) {
        self.imp()
            .library_folder
            .borrow()
            .as_ref()
            .expect("LibraryBrowser: library folder uninitialized")
            .refresh_content();
    }

    pub fn clear_selected_sheet(&self) {
        if let Some(selected) = self
            .imp()
            .selected_sheet_button
            .take()
            .and_then(|f| f.upgrade())
        {
            selected.set_active(false);
        }
    }

    pub fn select_sheet_button(&self, button: LibrarySheetButton) {
        if let Some(old) = self
            .imp()
            .selected_sheet_button
            .borrow()
            .as_ref()
            .and_then(|f| f.upgrade())
        {
            old.set_active(false);
        }
        button.set_active(true);

        self.imp()
            .selected_sheet_button
            .replace(Some(button.downgrade()));
    }

    pub fn select_sheet_by_path(&self, path: &Path) {
        self.clear_selected_sheet();

        let Ok(path) = path.canonicalize() else {
            return;
        };

        if let Some(button) = self
            .imp()
            .library_folder
            .borrow()
            .as_ref()
            .expect("LibraryBrowser: library folder uninitialized")
            .find_sheet_button(&path)
        {
            self.select_sheet_button(button);
        }
    }
}
