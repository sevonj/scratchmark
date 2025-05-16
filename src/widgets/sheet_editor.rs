mod imp {
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::subclass::Signal;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::{CompositeTemplate, TemplateChild};

    use gtk::Button;
    use sourceview5::View;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/sheet_editor.ui")]
    pub struct SheetEditor {
        #[template_child]
        pub(super) source_view: TemplateChild<View>,

        #[template_child]
        pub(super) close_sheet_button: TemplateChild<Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SheetEditor {
        const NAME: &'static str = "SheetEditor";
        type Type = super::SheetEditor;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SheetEditor {
        fn constructed(&self) {
            self.parent_constructed();

            let close_sheet_button = self.close_sheet_button.get();
            let obj = self.obj();
            close_sheet_button.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.emit_by_name::<()>("close-requested", &[]);
                }
            ));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("close-requested").build()])
        }
    }

    impl WidgetImpl for SheetEditor {}
    impl BinImpl for SheetEditor {}
}

use std::{fs::File, io::Read, path::PathBuf};

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;
use sourceview5::prelude::*;

use sourceview5::{Buffer, LanguageManager, StyleSchemeManager};

glib::wrapper! {
    pub struct SheetEditor(ObjectSubclass<imp::SheetEditor>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SheetEditor {
    pub fn new(path: PathBuf) -> Self {
        let this: Self = Object::builder().build();
        this.imp().source_view.set_monospace(true);
        match File::open(path) {
            Ok(f) => this.init_sheet(Some(f)),
            Err(_) => this.init_sheet(None),
        };
        this
    }

    fn init_sheet(&self, f: Option<File>) {
        let mut text = String::new();
        if let Some(mut f) = f {
            let _ = f.read_to_string(&mut text);
        }

        let imp = self.imp();
        let lang = LanguageManager::default().language("markdown").unwrap();

        let buffer = Buffer::with_language(&lang);
        buffer.set_text(&text);
        self.load_buffer_style_scheme(&buffer);

        imp.source_view.set_buffer(Some(&buffer));
    }

    fn load_buffer_style_scheme(&self, buffer: &Buffer) {
        let scheme_id = "theftmd";

        // Try fetching the scheme
        if let Some(style_scheme) = StyleSchemeManager::default().scheme(scheme_id) {
            buffer.set_style_scheme(Some(&style_scheme));
            return;
        }

        // --- ONLY IF NOT PACKAGED
        // Failed, install path
        const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
        StyleSchemeManager::default()
            .append_search_path(format!("{MANIFEST_DIR}/resources/editor_style").as_str());
        // --- //

        // Try fetching the scheme again
        if let Some(style_scheme) = StyleSchemeManager::default().scheme(scheme_id) {
            buffer.set_style_scheme(Some(&style_scheme));
            return;
        }

        println!("Failed to load scheme with id '{scheme_id}'.")
    }
}
