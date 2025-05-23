mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use gtk::gio;
    use gtk::glib;
    use gtk::prelude::*;

    use gio::File;
    use glib::subclass::Signal;
    use gtk::{Button, CompositeTemplate, TemplateChild};
    use sourceview5::View;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/sheet_editor.ui")]
    pub struct SheetEditor {
        #[template_child]
        pub(super) source_view: TemplateChild<View>,

        #[template_child]
        pub(super) close_sheet_button: TemplateChild<Button>,

        pub(super) file: RefCell<Option<File>>,
        pub(super) path: RefCell<Option<PathBuf>>,
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

use std::error::Error;
use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::gio;
use gtk::gio::FileCreateFlags;
use gtk::glib;
use gtk::prelude::*;
use sourceview5::prelude::*;

use gio::{Cancellable, File};
use glib::{GString, Object};
use sourceview5::{Buffer, LanguageManager, StyleSchemeManager};

#[derive(Debug)]
pub enum SheetEditorError {
    FileOpenFail,
    InvalidChars,
}

impl Error for SheetEditorError {}

impl std::fmt::Display for SheetEditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SheetEditorError::FileOpenFail => write!(f, "Failed to read file."),
            SheetEditorError::InvalidChars => write!(f, "File contains invalid characters."),
        }
    }
}

const NOT_CANCELLABLE: Option<&Cancellable> = None;

glib::wrapper! {
    pub struct SheetEditor(ObjectSubclass<imp::SheetEditor>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SheetEditor {
    pub fn new(path: PathBuf) -> Result<Self, SheetEditorError> {
        let file = File::for_path(&path);
        let slice = match file.load_contents(NOT_CANCELLABLE) {
            Ok((slice, _)) => slice,
            Err(_) => return Err(SheetEditorError::FileOpenFail),
        };

        let text = match GString::from_utf8_checked(slice.to_vec()) {
            Ok(text) => text,
            Err(_) => return Err(SheetEditorError::InvalidChars),
        };
        let lang = LanguageManager::default().language("markdown").unwrap();
        let buffer = Buffer::with_language(&lang);
        buffer.set_text(&text);

        let this: Self = Object::builder().build();
        this.load_buffer_style_scheme(&buffer);
        this.imp().file.replace(Some(file));
        this.imp().path.replace(Some(path));
        this.imp().source_view.set_monospace(true);
        this.imp().source_view.set_buffer(Some(&buffer));
        Ok(this)
    }

    pub fn save(&self) {
        let buffer = self.imp().source_view.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let text = buffer.text(&start, &end, true).to_string();
        let bytes = text.as_bytes();

        let Some(ref mut file) = *self.imp().file.borrow_mut() else {
            panic!("SheetEditor file uninitialized");
        };

        let output_stream = file
            .replace(None, false, FileCreateFlags::NONE, NOT_CANCELLABLE)
            .unwrap();

        output_stream.write_all(bytes, NOT_CANCELLABLE).unwrap();
        output_stream.flush(NOT_CANCELLABLE).unwrap();
    }

    pub fn path(&self) -> PathBuf {
        let opt = self.imp().path.borrow();
        opt.as_ref()
            .expect("SheetEditor: path uninitialized")
            .clone()
    }

    pub fn set_path(&self, path: PathBuf) {
        let file = File::for_path(&path);
        self.imp().file.replace(Some(file));
        self.imp().path.replace(Some(path));
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
