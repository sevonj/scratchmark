mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::glib::subclass::Signal;
    use gtk::prelude::*;

    use gtk::Button;
    use gtk::CompositeTemplate;
    use gtk::Image;
    use gtk::Label;
    use gtk::Revealer;
    use gtk::ToggleButton;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/project_selector_entry.ui")]
    pub struct ProjectSelectorEntry {
        //#[template_child]
        //reveal_toggle: TemplateChild<ToggleButton>,
        //#[template_child]
        //revealer: TemplateChild<Revealer>,
        #[template_child]
        pub(super) select_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub(super) remove_button: TemplateChild<Button>,
        #[template_child]
        pub(super) title: TemplateChild<Label>,

        pub(super) path: RefCell<PathBuf>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectSelectorEntry {
        const NAME: &'static str = "ProjectSelectorEntry";
        type Type = super::ProjectSelectorEntry;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProjectSelectorEntry {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            /*let revealer: &Revealer = self.revealer.as_ref();
            self.reveal_toggle
                .bind_property("active", revealer, "reveal-child")
                .bidirectional()
                .sync_create()
                .build();*/

            self.select_button.connect_clicked(clone!(
                #[weak]
                obj,
                move |select_toggle| {
                    // Don't let the button change state by itself
                    select_toggle.set_active(!select_toggle.is_active());
                    obj.emit_by_name("select-clicked", &[])
                }
            ));

            self.remove_button.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| { obj.emit_by_name("remove-clicked", &[]) }
            ));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("select-clicked").build(),
                    Signal::builder("remove-clicked").build(),
                ]
            })
        }
    }

    impl WidgetImpl for ProjectSelectorEntry {}
    impl BinImpl for ProjectSelectorEntry {}
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;

glib::wrapper! {
    pub struct ProjectSelectorEntry(ObjectSubclass<imp::ProjectSelectorEntry>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ProjectSelectorEntry {
    pub fn new(path: PathBuf) -> Self {
        let this: Self = Object::builder().build();

        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        this.imp().title.set_text(&name);
        this.imp().path.replace(path);

        this
    }

    pub fn path(&self) -> PathBuf {
        self.imp().path.borrow().clone()
    }

    pub fn set_active(&self, active: bool) {
        self.imp().select_button.set_active(active);
    }
}
