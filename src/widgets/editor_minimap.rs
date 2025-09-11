mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib;

    use gtk::CompositeTemplate;
    use sourceview5::Map;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor_minimap.ui")]
    pub struct EditorMinimap {
        #[template_child]
        pub(super) map: TemplateChild<Map>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditorMinimap {
        const NAME: &'static str = "EditorMinimap";
        type Type = super::EditorMinimap;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EditorMinimap {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for EditorMinimap {}
    impl BinImpl for EditorMinimap {}
}

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::glib;
use sourceview5::{View, prelude::MapExt};

glib::wrapper! {
    pub struct EditorMinimap(ObjectSubclass<imp::EditorMinimap>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for EditorMinimap {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl EditorMinimap {
    pub fn bind(&self, view: &View) {
        self.imp().map.set_view(view);
    }
}
