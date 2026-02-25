mod imp {
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::glib;
    use sourceview5::Map;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor/minimap.ui")]
    pub struct Minimap {
        #[template_child]
        pub(super) map: TemplateChild<Map>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Minimap {
        const NAME: &'static str = "Minimap";
        type Type = super::Minimap;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Minimap {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for Minimap {}
    impl BinImpl for Minimap {}
}

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::Object;
use sourceview5::prelude::*;

use super::text_view::EditorTextView;

glib::wrapper! {
    pub struct Minimap(ObjectSubclass<imp::Minimap>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Minimap {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl Minimap {
    pub fn bind(&self, view: &EditorTextView) {
        self.imp().map.set_view(view);
    }
}
