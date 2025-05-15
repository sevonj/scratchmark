//! Expandable folder widget for library browser
//!

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::Binding;
    use gtk::glib;

    use gtk::Label;
    use gtk::{CompositeTemplate, TemplateChild};

    use crate::data::SheetObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_sheet_button.ui")]
    pub struct LibrarySheetButton {
        #[template_child]
        pub(super) sheet_name_label: TemplateChild<Label>,

        pub(super) sheet_object: RefCell<Option<SheetObject>>,
        pub(super) bindings: RefCell<Vec<Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibrarySheetButton {
        const NAME: &'static str = "LibrarySheet";
        type Type = super::LibrarySheetButton;
        type ParentType = gtk::ToggleButton;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibrarySheetButton {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for LibrarySheetButton {}
    impl ButtonImpl for LibrarySheetButton {}
    impl ToggleButtonImpl for LibrarySheetButton {}
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;

use crate::data::SheetObject;

glib::wrapper! {
    pub struct LibrarySheetButton(ObjectSubclass<imp::LibrarySheetButton>)
        @extends gtk::ToggleButton, gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl LibrarySheetButton {
    pub fn new(data: &SheetObject) -> Self {
        let this: Self = Object::builder().build();
        this.bind(data);
        this
    }

    pub fn path(&self) -> PathBuf {
        self.imp()
            .sheet_object
            .borrow()
            .as_ref()
            .expect("LibrarySheetButton data uninitialized")
            .path()
    }

    fn bind(&self, data: &SheetObject) {
        self.imp().sheet_object.replace(Some(data.clone()));

        let title_label = self.imp().sheet_name_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let title_binding = data
            .bind_property("stem", &title_label, "label")
            .sync_create()
            .build();
        bindings.push(title_binding);
    }
}
