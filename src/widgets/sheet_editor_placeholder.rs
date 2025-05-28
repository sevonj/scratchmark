//! Placeholder status page when editor is not open
//!

mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib;

    use gtk::CompositeTemplate;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/sheet_editor_placeholder.ui")]
    pub struct SheetEditorPlaceholder {}

    #[glib::object_subclass]
    impl ObjectSubclass for SheetEditorPlaceholder {
        const NAME: &'static str = "SheetEditorPlaceholder";
        type Type = super::SheetEditorPlaceholder;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SheetEditorPlaceholder {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for SheetEditorPlaceholder {}
    impl BinImpl for SheetEditorPlaceholder {}
}

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct SheetEditorPlaceholder(ObjectSubclass<imp::SheetEditorPlaceholder>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for SheetEditorPlaceholder {
    fn default() -> Self {
        Object::builder().build()
    }
}
