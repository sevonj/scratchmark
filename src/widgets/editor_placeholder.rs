//! Placeholder status page when editor is not open
//!

mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib;

    use gtk::CompositeTemplate;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor_placeholder.ui")]
    pub struct EditorPlaceholder {}

    #[glib::object_subclass]
    impl ObjectSubclass for EditorPlaceholder {
        const NAME: &'static str = "EditorPlaceholder";
        type Type = super::EditorPlaceholder;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EditorPlaceholder {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for EditorPlaceholder {}
    impl BinImpl for EditorPlaceholder {}
}

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct EditorPlaceholder(ObjectSubclass<imp::EditorPlaceholder>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for EditorPlaceholder {
    fn default() -> Self {
        Object::builder().build()
    }
}
