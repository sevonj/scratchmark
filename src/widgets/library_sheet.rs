//! Expandable folder widget for library browser
//!

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::Binding;
    use gtk::glib;

    use gtk::Button;
    use gtk::Label;
    use gtk::{CompositeTemplate, TemplateChild};

    use crate::data::SheetObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_sheet.ui")]
    pub struct LibrarySheet {
        #[template_child]
        pub(super) sheet_button: TemplateChild<Button>,
        #[template_child]
        pub(super) sheet_name_label: TemplateChild<Label>,

        pub(super) sheet_object: RefCell<Option<SheetObject>>,
        pub(super) bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibrarySheet {
        const NAME: &'static str = "LibrarySheet";
        type Type = super::LibrarySheet;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibrarySheet {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for LibrarySheet {}
    impl BinImpl for LibrarySheet {}
}

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;

use crate::data::SheetObject;

glib::wrapper! {
    pub struct LibrarySheet(ObjectSubclass<imp::LibrarySheet>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for LibrarySheet {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibrarySheet {
    pub fn bind(&self, data: &SheetObject) {
        self.imp().sheet_object.replace(Some(data.clone()));

        let title_label = self.imp().sheet_name_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let title_binding = data
            .bind_property("stem", &title_label, "label")
            .sync_create()
            .build();
        bindings.push(title_binding);
    }

    pub fn unbind(&self) {
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
