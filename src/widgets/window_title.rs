mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::prelude::*;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::WindowTitle)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/window_title.ui")]
    pub struct WindowTitle {
        #[template_child]
        window_title: TemplateChild<adw::WindowTitle>,

        #[property(get, set, nullable)]
        filename: RefCell<Option<String>>,
        #[property(get, set)]
        unsaved_changes: Cell<bool>,
        #[property(get, set)]
        focus_mode: Cell<bool>,
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

            obj.connect_notify(None, move |obj, _| {
                // On any property change
                obj.imp().update_window_title();
            });
        }
    }

    impl WidgetImpl for WindowTitle {}
    impl BinImpl for WindowTitle {}

    impl WindowTitle {
        fn update_window_title(&self) {
            let Some(filename) = self.filename.borrow().as_ref().cloned() else {
                self.window_title.set_title("Scratchmark");
                return;
            };

            let unsaved = if self.unsaved_changes.get() {
                "⦁ "
            } else {
                ""
            };

            let focus = if self.focus_mode.get() {
                " (Focus Mode)"
            } else {
                ""
            };

            self.window_title
                .set_title(&format!("{unsaved}{filename}{focus}"));
        }
    }
}

use gtk::glib;
use gtk::glib::Object;

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
