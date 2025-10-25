mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;
    use gtk::glib::Properties;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::WindowTitle)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/window_title.ui")]
    pub struct WindowTitle {
        #[template_child]
        window_title: TemplateChild<adw::WindowTitle>,

        #[property(get, set, nullable)]
        filename: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WindowTitle {
        const NAME: &'static str = "WindowTitle";
        type Type = super::WindowTitle;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for WindowTitle {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            obj.connect_notify(Some("filename"), move |this, _| {
                this.imp().update_window_title();
            });
        }
    }

    impl WidgetImpl for WindowTitle {}
    impl BinImpl for WindowTitle {}

    impl WindowTitle {
        fn update_window_title(&self) {
            let binding = self.filename.borrow();
            let title_text = binding.as_ref().map_or("Scratchmark", |s| &s);
            self.window_title.set_title(title_text);
        }
    }
}

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct WindowTitle(ObjectSubclass<imp::WindowTitle>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for WindowTitle {
    fn default() -> Self {
        Object::builder().build()
    }
}
