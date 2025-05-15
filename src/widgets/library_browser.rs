//! Library browser is located in the left sidebar.
//!

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::{
        CompositeTemplate,
        subclass::widget::{CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetImpl},
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_browser.ui")]
    pub struct LibraryBrowser {
        #[template_child]
        pub(super) library_root: TemplateChild<gtk::Box>,
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
    }

    impl WidgetImpl for LibraryBrowser {}
    impl BinImpl for LibraryBrowser {}
}

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{
    glib::{self},
    prelude::BoxExt,
};

use crate::{data::FolderObject, util::path_builtin_library};

use super::LibraryRootFolder;

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
        let folder = LibraryRootFolder::default();
        folder.bind(&data);
        folder.refresh_content();

        library_root.append(&folder);
    }
}
