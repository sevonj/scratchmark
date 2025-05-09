mod imp {
    use adw::subclass::prelude::*;

    use gtk::{
        CompositeTemplate,
        glib::{self, *},
        subclass::widget::{CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetImpl},
    };
    use sourceview5::View;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/sheet_editor.ui")]
    pub struct SheetEditor {
        #[template_child]
        pub(super) source_view: TemplateChild<View>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SheetEditor {
        const NAME: &'static str = "SheetEditor";
        type Type = super::SheetEditor;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SheetEditor {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for SheetEditor {}
    impl BinImpl for SheetEditor {}
}

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{
    glib::{self},
    prelude::TextViewExt,
};
use sourceview5::{Buffer, LanguageManager, StyleSchemeManager, prelude::BufferExt};

glib::wrapper! {
    pub struct SheetEditor(ObjectSubclass<imp::SheetEditor>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible;
}

impl Default for SheetEditor {
    fn default() -> Self {
        StyleSchemeManager::default().append_search_path("../resources/editor_style");

        let this: Self = Object::builder().build();
        this.init_sheet();
        this
    }
}

impl SheetEditor {
    pub fn new_sheet(&self) {
        self.init_sheet();
    }

    fn init_sheet(&self) {
        let imp = self.imp();
        let lang = LanguageManager::default().language("markdown").unwrap();

        let buffer = Buffer::with_language(&lang);

        let scheme_id = "theftmd";
        if let Some(style_scheme) = StyleSchemeManager::default().scheme(scheme_id) {
            buffer.set_style_scheme(Some(&style_scheme));
        } else {
            println!("Failed to load scheme with id '{scheme_id}'.")
        }

        imp.source_view.set_buffer(Some(&buffer));
    }
}
