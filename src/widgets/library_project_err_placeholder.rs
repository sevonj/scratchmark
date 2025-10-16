mod imp {
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::gdk;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::prelude::*;

    use gtk::Builder;
    use gtk::Button;
    use gtk::CompositeTemplate;
    use gtk::Label;
    use gtk::PopoverMenu;
    use gtk::gdk::Rectangle;
    use gtk::gio::MenuModel;
    use gtk::gio::SimpleAction;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib::subclass::Signal;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library_project_err_placeholder.ui")]
    pub struct LibraryProjectErrPlaceholder {
        #[template_child]
        pub(super) button: TemplateChild<Button>,
        #[template_child]
        pub(super) title: TemplateChild<Label>,
        context_menu_popover: RefCell<Option<PopoverMenu>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryProjectErrPlaceholder {
        const NAME: &'static str = "LibraryProjectErrPlaceholder";
        type Type = super::LibraryProjectErrPlaceholder;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibraryProjectErrPlaceholder {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("close-project-requested").build()])
        }

        fn constructed(&self) {
            let obj = self.obj();

            self.parent_constructed();
            self.setup_context_menu();

            let actions = SimpleActionGroup::new();
            obj.insert_action_group("project-root", Some(&actions));

            let action = SimpleAction::new("close-project", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    obj.emit_by_name::<()>("close-project-requested", &[]);
                }
            ));
            actions.add_action(&action);
        }
    }

    impl WidgetImpl for LibraryProjectErrPlaceholder {}
    impl BinImpl for LibraryProjectErrPlaceholder {}

    impl LibraryProjectErrPlaceholder {
        fn setup_context_menu(&self) {
            let resource_path =
                "/org/scratchmark/Scratchmark/ui/library_project_err_placeholder_context_menu.ui";
            let obj = self.obj();

            let builder = Builder::from_resource(resource_path);
            let popover = builder
                .object::<MenuModel>("context-menu")
                .expect("LibraryProjectErrPlaceholder context-menu model failed");
            let menu = PopoverMenu::builder()
                .menu_model(&popover)
                .has_arrow(false)
                .build();
            let button: &Button = self.button.as_ref();
            menu.set_parent(button);
            let _ = self.context_menu_popover.replace(Some(menu));

            let gesture = gtk::GestureClick::new();
            gesture.set_button(gdk::ffi::GDK_BUTTON_SECONDARY as u32);
            gesture.connect_released(clone!(
                #[weak(rename_to = this)]
                self,
                move |gesture, _n, x, y| {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    if let Some(popover) = this.context_menu_popover.borrow().as_ref() {
                        popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
                        popover.popup();
                    };
                }
            ));
            self.button.add_controller(gesture);

            obj.connect_destroy(move |obj| {
                if let Some(popover) = obj.imp().context_menu_popover.take() {
                    popover.unparent();
                }
            });
        }
    }
}

use std::path::Path;

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct LibraryProjectErrPlaceholder(ObjectSubclass<imp::LibraryProjectErrPlaceholder>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LibraryProjectErrPlaceholder {
    pub fn new(path: &Path) -> Self {
        let this: LibraryProjectErrPlaceholder = Object::builder().build();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        this.imp().title.set_text(&name);
        this
    }
}
