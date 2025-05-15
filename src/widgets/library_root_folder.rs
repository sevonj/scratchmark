//! Library root folder widget for library browser
//!

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use adw::subclass::prelude::*;
    use glib::Binding;
    use glib::subclass::Signal;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::{CompositeTemplate, TemplateChild};

    use crate::data::FolderObject;
    use crate::widgets::LibrarySheetButton;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_root_folder.ui")]
    pub struct LibraryRootFolder {
        #[template_child]
        pub(super) subdir_vbox: TemplateChild<gtk::Box>,

        pub(super) folder_object: RefCell<Option<FolderObject>>,
        pub(super) _bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryRootFolder {
        const NAME: &'static str = "LibraryRootFolder";
        type Type = super::LibraryRootFolder;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibraryRootFolder {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("sheet-clicked")
                        .param_types([LibrarySheetButton::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryRootFolder {}
    impl BinImpl for LibraryRootFolder {}
}

use adw::subclass::prelude::*;
use glib::Object;
use glib::closure_local;
use gtk::glib;
use gtk::prelude::*;

use crate::data::FolderObject;
use crate::widgets::LibrarySheetButton;

use super::LibraryFolder;

glib::wrapper! {
    pub struct LibraryRootFolder(ObjectSubclass<imp::LibraryRootFolder>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LibraryRootFolder {
    pub fn new(data: &FolderObject) -> Self {
        let this: Self = Object::builder().build();
        this.bind(data);
        this
    }

    pub fn refresh_content(&self) {
        let opt = self.imp().folder_object.borrow();
        let folder = opt.as_ref().expect("FolderObject not bound");

        let entries = folder.content();

        for entry in entries {
            if !entry.metadata().is_ok_and(|meta| meta.is_dir()) {
                return;
            }
            let data = FolderObject::new(entry.path());
            let folder = LibraryFolder::new(&data);
            self.imp().subdir_vbox.append(&folder);
            folder.refresh_content();

            let this = self;
            folder.connect_closure(
                "sheet-clicked",
                false,
                closure_local!(
                    #[weak]
                    this,
                    move |_folder: LibraryFolder, button: LibrarySheetButton| {
                        this.emit_by_name::<()>("sheet-clicked", &[&button]);
                    }
                ),
            );
        }
    }

    fn bind(&self, data: &FolderObject) {
        self.imp().folder_object.replace(Some(data.clone()));
    }
}
