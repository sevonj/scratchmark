mod imp {
    use std::cell::RefCell;
    use std::fs::File;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::subclass::Signal;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::Button;
    use gtk::{CompositeTemplate, TemplateChild};
    use sourceview5::View;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/sheet_editor.ui")]
    pub struct SheetEditor {
        #[template_child]
        pub(super) source_view: TemplateChild<View>,

        #[template_child]
        pub(super) close_sheet_button: TemplateChild<Button>,

        pub(super) file: RefCell<Option<File>>,
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

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

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
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("file open fail");
        let mut text = String::new();
        file.read_to_string(&mut text).expect("TODO read to string");

        let lang = LanguageManager::default().language("markdown").unwrap();
        let buffer = Buffer::with_language(&lang);
        buffer.set_text(&text);

        let this: Self = Object::builder().build();
        this.load_buffer_style_scheme(&buffer);
        this.imp().file.replace(Some(file));
        this.imp().source_view.set_monospace(true);
        this.imp().source_view.set_buffer(Some(&buffer));
        this
    }

    pub fn save(&self) {
        let buffer = self.imp().source_view.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let text = buffer.text(&start, &end, true).to_string();
        let bytes = text.as_bytes();

        let Some(ref mut file) = *self.imp().file.borrow_mut() else {
            panic!("SheetEditor file_lock uninitialized");
        };

        file.seek(SeekFrom::Start(0)).expect("seek failed");
        file.set_len(0).expect("clear failed");
        file.write_all(bytes).expect("write failed");
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
