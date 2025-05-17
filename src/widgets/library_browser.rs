//! Library browser is located in the left sidebar.
//!

mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;

    use crate::widgets::LibrarySheetButton;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_browser.ui")]
    pub struct LibraryBrowser {
        #[template_child]
        pub(super) library_root: TemplateChild<gtk::Box>,

        pub(super) selected_sheet_button: RefCell<Option<glib::WeakRef<LibrarySheetButton>>>,
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
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("sheet-selected")
                        .param_types([PathBuf::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryBrowser {}
    impl BinImpl for LibraryBrowser {}
}

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use glib::closure_local;
use gtk::glib;
use gtk::prelude::*;

use crate::widgets::LibrarySheetButton;
use crate::{data::FolderObject, util::path_builtin_library};

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
    pub fn refresh_content(&self) {
        let library_root = &self.imp().library_root;

        let data = FolderObject::new(path_builtin_library());
        let folder = LibraryFolder::new_root(&data);
        folder.refresh_content();

        library_root.append(&folder);

        let this = self;
        folder.connect_closure(
            "sheet-clicked",
            false,
            closure_local!(
                #[weak]
                this,
                move |_folder: LibraryFolder, button: LibrarySheetButton| {
                    if let Some(old) = this
                        .imp()
                        .selected_sheet_button
                        .borrow()
                        .as_ref()
                        .and_then(|f| f.upgrade())
                    {
                        old.set_active(false);
                    }

                    this.imp()
                        .selected_sheet_button
                        .replace(Some(button.downgrade()));

                    let path = button.path();
                    this.emit_by_name::<()>("sheet-selected", &[&path]);
                }
            ),
        );

        folder.connect_closure(
            "sheet-delete-requested",
            false,
            closure_local!(
                move |_folder: super::LibraryFolder, button: LibrarySheetButton| {
                    let path = button.path();
                    std::fs::remove_file(path).expect("file delet failed");
                }
            ),
        );
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
}
