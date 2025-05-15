//! Library root folder widget for library browser
//!

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::Binding;
    use gtk::glib;

    use gtk::{CompositeTemplate, TemplateChild};

    use crate::data::FolderObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_root_folder.ui")]
    pub struct LibraryRootFolder {
        #[template_child]
        pub(super) subdir_vbox: TemplateChild<gtk::Box>,

        pub(super) folder_object: RefCell<Option<FolderObject>>,
        pub(super) bindings: RefCell<Vec<Binding>>,
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
    }

    impl WidgetImpl for LibraryRootFolder {}
    impl BinImpl for LibraryRootFolder {}
}

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;

use crate::data::FolderObject;

use super::LibraryFolder;

glib::wrapper! {
    pub struct LibraryRootFolder(ObjectSubclass<imp::LibraryRootFolder>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible;
}

impl Default for LibraryRootFolder {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibraryRootFolder {
    pub fn refresh_content(&self) {
        let opt = self.imp().folder_object.borrow();
        let folder = opt.as_ref().expect("FolderObject not bound");

        let entries = folder.content();

        for entry in entries {
            if !entry.metadata().is_ok_and(|meta| meta.is_dir()) {
                return;
            }
            let data = FolderObject::new(entry.path());
            let folder = LibraryFolder::default();
            folder.bind(&data);
            self.imp().subdir_vbox.append(&folder);
            folder.refresh_content();
        }
    }

    pub fn bind(&self, data: &FolderObject) {
        self.imp().folder_object.replace(Some(data.clone()));
    }

    pub fn unbind(&self) {
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
