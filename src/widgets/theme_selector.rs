mod imp {
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::prelude::*;
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::ThemeSelector)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/theme_selector.ui")]
    pub struct ThemeSelector {
        #[property(get, set)]
        theme: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ThemeSelector {
        const NAME: &'static str = "ThemeSelector";
        type Type = super::ThemeSelector;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_property_action("theme", "theme");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ThemeSelector {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();
        }
    }

    impl WidgetImpl for ThemeSelector {}
    impl BinImpl for ThemeSelector {}

    impl ThemeSelector {}
}

use gtk::glib;
use gtk::glib::Object;

glib::wrapper! {
    pub struct ThemeSelector(ObjectSubclass<imp::ThemeSelector>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ThemeSelector {
    fn default() -> Self {
        Object::builder().build()
    }
}
