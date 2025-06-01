mod imp {
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{clone, closure_local};
    use gtk::gio;
    use gtk::glib;

    use adw::{AlertDialog, Banner};
    use gio::{File, FileMonitor, FileMonitorFlags, SimpleActionGroup};
    use glib::Properties;
    use glib::subclass::Signal;
    use gtk::{Button, CompositeTemplate, TemplateChild};
    use sourceview5::View;

    use crate::util;

    use super::NOT_CANCELLABLE;

    #[derive(Debug, Properties, CompositeTemplate, Default)]
    #[properties(wrapper_type = super::SheetEditor)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/sheet_editor.ui")]
    pub struct SheetEditor {
        #[template_child]
        pub(super) source_view: TemplateChild<View>,

        #[template_child]
        pub(super) file_changed_banner: TemplateChild<Banner>,
        #[template_child]
        pub(super) close_sheet_button: TemplateChild<Button>,

        pub(super) file: RefCell<Option<File>>,
        pub(super) filemon: RefCell<Option<FileMonitor>>,
        pub(super) path: RefCell<Option<PathBuf>>,

        #[property(get, set)]
        pub(super) file_changed: Cell<bool>,
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

    #[glib::derived_properties]
    impl ObjectImpl for SheetEditor {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            self.close_sheet_button.get().connect_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.emit_by_name::<()>("close-requested", &[]);
                }
            ));

            self.file_changed_banner.connect_button_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    let heading = "File changed";
                    let body = "The file has changed on disk.";
                    let dialog = AlertDialog::new(
                        Some(heading),
                        Some(body), // once told me the world is gonna roll me
                    );
                    dialog.add_response("discard", "Discard changes");
                    dialog.add_response("overwrite", "Overwrite file");
                    dialog.add_response("keep-both", "Keep both");
                    dialog.set_response_appearance("keep-both", adw::ResponseAppearance::Suggested);
                    dialog
                        .set_response_appearance("overwrite", adw::ResponseAppearance::Destructive);
                    dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
                    dialog.connect_closure(
                        "response",
                        false,
                        closure_local!(
                            #[weak]
                            obj,
                            move |_: AlertDialog, response: String| {
                                if response == "keep-both" {
                                    let new_path = util::incremented_path(obj.path());
                                    obj.set_path(new_path);
                                    obj.imp().file_changed.set(false);
                                    obj.imp().file_changed_banner.set_revealed(false);
                                    if let Err(e) = obj.save() {
                                        obj.emit_by_name::<()>("toast", &[&e.to_string()]);
                                        return;
                                    };
                                    obj.emit_by_name::<()>("saved-as", &[]);
                                } else if response == "overwrite" {
                                    obj.imp().file_changed.set(false);
                                    if let Err(e) = obj.save() {
                                        obj.emit_by_name::<()>("toast", &[&e.to_string()]);
                                        return;
                                    };
                                    obj.imp().file_changed.set(false);
                                    obj.imp().file_changed_banner.set_revealed(false);
                                } else if response == "discard" {
                                    let file = gio::File::for_path(obj.path());
                                    match util::read_file_to_string(&file) {
                                        Ok(text) => {
                                            obj.imp().source_view.buffer().set_text(&text);
                                            obj.imp().file_changed.set(false);
                                            obj.imp().file_changed_banner.set_revealed(false);
                                        }
                                        Err(e) => {
                                            obj.emit_by_name::<()>("toast", &[&e.to_string()])
                                        }
                                    }
                                }
                            }
                        ),
                    );
                    dialog.present(Some(&obj));
                }
            ));

            let actions = SimpleActionGroup::new();
            obj.insert_action_group("editor", Some(&actions));

            let action = gio::SimpleAction::new("close", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    obj.emit_by_name::<()>("close-requested", &[]);
                }
            ));
            actions.add_action(&action);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("close-requested").build(),
                    Signal::builder("saved-as").build(),
                    Signal::builder("toast")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for SheetEditor {}
    impl BinImpl for SheetEditor {}

    impl SheetEditor {
        pub(super) fn setup_filemon(&self) {
            let Some(ref mut file) = *self.file.borrow_mut() else {
                panic!("SheetEditor file uninitialized");
            };
            let filemon = file
                .monitor(FileMonitorFlags::NONE, NOT_CANCELLABLE)
                .expect("Editor: Failed to create file monitor");
            filemon.connect_changed(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _, _, _| {
                    this.file_changed.set(true);
                    this.file_changed_banner.set_revealed(true);
                }
            ));

            self.file_changed.set(false);
            self.filemon.replace(Some(filemon));
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::gio;
use gtk::gio::FileCreateFlags;
use gtk::glib;
use gtk::prelude::*;
use sourceview5::prelude::*;

use gio::Cancellable;
use glib::Object;
use sourceview5::{Buffer, LanguageManager, StyleSchemeManager};

#[cfg(feature = "installed")]
use crate::APP_ID;
use crate::error::ScratchmarkError;
use crate::util;

const NOT_CANCELLABLE: Option<&Cancellable> = None;

glib::wrapper! {
    pub struct SheetEditor(ObjectSubclass<imp::SheetEditor>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SheetEditor {
    pub fn new(path: PathBuf) -> Result<Self, ScratchmarkError> {
        let file = gio::File::for_path(&path);
        let text = util::read_file_to_string(&file)?;
        let lang = LanguageManager::default().language("markdown").unwrap();
        let buffer = Buffer::with_language(&lang);
        buffer.set_text(&text);

        let this: Self = Object::builder().build();
        this.load_buffer_style_scheme(&buffer);
        this.imp().file.replace(Some(file));
        this.imp().path.replace(Some(path));
        this.imp().source_view.set_monospace(true);
        this.imp().source_view.set_buffer(Some(&buffer));
        this.imp().setup_filemon();
        Ok(this)
    }

    pub fn save(&self) -> Result<(), ScratchmarkError> {
        if self.imp().file_changed.get() {
            return Err(ScratchmarkError::FileChanged);
        }
        self.imp().filemon.borrow().as_ref().unwrap().cancel();

        let buffer = self.imp().source_view.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let text = buffer.text(&start, &end, true).to_string();
        let bytes = text.as_bytes();
        {
            let Some(ref mut file) = *self.imp().file.borrow_mut() else {
                panic!("SheetEditor file uninitialized");
            };

            let output_stream = file
                .replace(None, false, FileCreateFlags::NONE, NOT_CANCELLABLE)
                .unwrap();

            output_stream.write_all(bytes, NOT_CANCELLABLE).unwrap();
            output_stream.flush(NOT_CANCELLABLE).unwrap();
        }
        self.imp().setup_filemon();
        Ok(())
    }

    pub fn path(&self) -> PathBuf {
        let opt = self.imp().path.borrow();
        opt.as_ref()
            .expect("SheetEditor: path uninitialized")
            .clone()
    }

    pub fn set_path(&self, path: PathBuf) {
        let file = gio::File::for_path(&path);
        self.imp().file.replace(Some(file));
        self.imp().path.replace(Some(path));
        self.imp().setup_filemon();
    }

    fn load_buffer_style_scheme(&self, buffer: &Buffer) {
        let scheme_id = "scratchmark";

        // Try fetching the scheme
        if let Some(style_scheme) = StyleSchemeManager::default().scheme(scheme_id) {
            buffer.set_style_scheme(Some(&style_scheme));
            return;
        }

        #[cfg(feature = "installed")]
        {
            for dir in glib::system_data_dirs() {
                let path = dir.join(APP_ID).join("editor_schemes");
                StyleSchemeManager::default().append_search_path(path.to_str().unwrap());
            }
        }
        #[cfg(not(feature = "installed"))]
        {
            const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
            StyleSchemeManager::default()
                .append_search_path(format!("{MANIFEST_DIR}/data/editor_schemes").as_str());
        }

        // Try fetching the scheme again
        if let Some(style_scheme) = StyleSchemeManager::default().scheme(scheme_id) {
            buffer.set_style_scheme(Some(&style_scheme));
            return;
        }

        println!("Failed to load scheme with id '{scheme_id}'.")
    }
}
