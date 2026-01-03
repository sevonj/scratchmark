mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib;
    use sourceview5::subclass::prelude::*;

    #[derive(Debug, Default)]
    pub struct EditorTextView {}

    #[glib::object_subclass]
    impl ObjectSubclass for EditorTextView {
        const NAME: &'static str = "EditorTextView";
        type Type = super::EditorTextView;
        type ParentType = sourceview5::View;

        fn class_init(_klass: &mut Self::Class) {}

        fn instance_init(_obj: &glib::subclass::InitializingObject<Self>) {}
    }

    impl ObjectImpl for EditorTextView {
        fn constructed(&self) {
            self.parent_constructed();
            let _obj = self.obj();
        }
    }

    impl WidgetImpl for EditorTextView {}
    impl TextViewImpl for EditorTextView {}
    impl ViewImpl for EditorTextView {}

    impl EditorTextView {}
}

use gtk::glib;

use gtk::glib::Object;

glib::wrapper! {
    pub struct EditorTextView(ObjectSubclass<imp::EditorTextView>)
        @extends sourceview5::View, gtk::TextView, gtk::Widget,
        @implements gtk::Accessible, gtk::AccessibleText, gtk::Buildable, gtk::ConstraintTarget, gtk::Scrollable;
}

impl Default for EditorTextView {
    fn default() -> Self {
        Object::builder().build()
    }
}
