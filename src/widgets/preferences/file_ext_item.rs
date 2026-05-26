mod imp {
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::Button;
    use gtk::CompositeTemplate;
    use gtk::Label;
    use gtk::glib;
    use gtk::glib::CollationKey;
    use gtk::glib::clone;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/preferences/file_ext_item.ui")]
    pub struct PreferencesFileExtItem {
        #[template_child]
        pub(super) label: TemplateChild<Label>,
        #[template_child]
        remove_button: TemplateChild<Button>,

        pub(super) collation_key: OnceLock<CollationKey>,
        pub(super) ext: OnceLock<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesFileExtItem {
        const NAME: &'static str = "PreferencesFileExtItem";
        type Type = super::PreferencesFileExtItem;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesFileExtItem {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("remove").build()])
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            self.remove_button.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| obj.emit_by_name::<()>("remove", &[])
            ));
        }
    }

    impl WidgetImpl for PreferencesFileExtItem {}
    impl BoxImpl for PreferencesFileExtItem {}

    impl PreferencesFileExtItem {}
}

use gtk::glib;
use gtk::glib::CollationKey;
use gtk::glib::Object;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct PreferencesFileExtItem(ObjectSubclass<imp::PreferencesFileExtItem>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl PreferencesFileExtItem {
    /// ext: Extension without dot.
    pub fn new(ext: String) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        imp.collation_key.set(CollationKey::from(&ext)).unwrap();
        imp.label.set_label(&format!(".{ext}"));
        imp.ext.set(ext).unwrap();
        obj
    }

    pub fn ext(&self) -> &str {
        self.imp().ext.get().unwrap()
    }

    pub fn collation_key(&self) -> &CollationKey {
        self.imp().collation_key.get().unwrap()
    }
}
