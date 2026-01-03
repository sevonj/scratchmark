mod editor_text_view;
mod formatting;
mod markdown_buffer;
mod minimap;
mod regex;

mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::closure_local;
    use gtk::gio;
    use gtk::glib;

    use adw::AlertDialog;
    use adw::Banner;
    use adw::OverlaySplitView;
    use gio::File;
    use gio::FileMonitor;
    use gio::FileMonitorFlags;
    use gio::SimpleActionGroup;
    use glib::Properties;
    use glib::VariantTy;
    use glib::subclass::Signal;
    use gtk::CompositeTemplate;
    use gtk::TemplateChild;
    use gtk::TextMark;
    use gtk::gio::SimpleAction;

    use super::editor_text_view::EditorTextView;
    use super::formatting;
    use super::minimap::Minimap;
    use crate::data::DocumentStats;
    use crate::util;
    use crate::widgets::EditorDocStats;
    use crate::widgets::EditorSearchBar;

    use super::NOT_CANCELLABLE;

    #[derive(Debug, Properties, CompositeTemplate, Default)]
    #[properties(wrapper_type = super::Editor)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor/editor.ui")]
    pub struct Editor {
        #[template_child]
        pub(super) source_view: TemplateChild<EditorTextView>,
        #[template_child]
        pub(super) document_stats: TemplateChild<EditorDocStats>,
        pub(super) document_stats_data: Cell<DocumentStats>,

        #[template_child]
        pub(super) search_bar: TemplateChild<EditorSearchBar>,
        #[template_child]
        pub(super) file_changed_on_disk_banner: TemplateChild<Banner>,
        #[template_child]
        pub(super) editor_split: TemplateChild<OverlaySplitView>,
        #[template_child]
        pub(super) minimap: TemplateChild<Minimap>,
        #[property(get, set)]
        pub(super) show_minimap: Cell<bool>,

        pub(super) file: RefCell<Option<File>>,
        pub(super) filemon: RefCell<Option<FileMonitor>>,
        pub(super) path: RefCell<Option<PathBuf>>,

        #[property(get, set)]
        pub(super) file_changed_on_disk: Cell<bool>,
        #[property(get, set)]
        pub(super) unsaved_changes: Cell<bool>,
        #[property(get, set)]
        pub(super) show_sidebar: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Editor {
        const NAME: &'static str = "Editor";
        type Type = super::Editor;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            EditorTextView::ensure_type();
            EditorDocStats::ensure_type();
            Minimap::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Editor {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            self.search_bar.connect_closure(
                "scroll-to-mark",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: EditorSearchBar, mark: TextMark| {
                        this.source_view.scroll_to_mark(&mark, 0.0, false, 0.5, 0.5);
                    }
                ),
            );

            self.editor_split
                .bind_property("show_sidebar", obj.as_ref(), "show_sidebar")
                .sync_create()
                .bidirectional()
                .build();

            self.file_changed_on_disk_banner
                .connect_button_clicked(clone!(
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
                        dialog.set_response_appearance(
                            "keep-both",
                            adw::ResponseAppearance::Suggested,
                        );
                        dialog.set_response_appearance(
                            "overwrite",
                            adw::ResponseAppearance::Destructive,
                        );
                        dialog.set_response_appearance(
                            "discard",
                            adw::ResponseAppearance::Destructive,
                        );
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
                                        obj.imp().file_changed_on_disk.set(false);
                                        obj.imp().file_changed_on_disk_banner.set_revealed(false);
                                        if let Err(e) = obj.save() {
                                            obj.emit_by_name::<()>("toast", &[&e.to_string()]);
                                            return;
                                        };
                                        obj.emit_by_name::<()>("saved-as", &[]);
                                    } else if response == "overwrite" {
                                        obj.imp().file_changed_on_disk.set(false);
                                        if let Err(e) = obj.save() {
                                            obj.emit_by_name::<()>("toast", &[&e.to_string()]);
                                            return;
                                        };
                                        obj.imp().file_changed_on_disk.set(false);
                                        obj.imp().file_changed_on_disk_banner.set_revealed(false);
                                    } else if response == "discard" {
                                        let file = gio::File::for_path(obj.path());
                                        match util::read_file_to_string(&file) {
                                            Ok(text) => {
                                                obj.imp().source_view.buffer().set_text(&text);
                                                obj.imp().file_changed_on_disk.set(false);
                                                obj.imp()
                                                    .file_changed_on_disk_banner
                                                    .set_revealed(false);
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

            self.minimap.bind(&self.source_view);
            self.minimap
                .bind_property("visible", obj.as_ref(), "show_minimap")
                .sync_create()
                .bidirectional()
                .build();

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

            // This action is a workaround to capture <Shift>Return from the Entry
            let action = SimpleAction::new("shiftreturn", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _| {
                    this.search_bar
                        .activate_action("search.shiftreturn", None)
                        .unwrap();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("show-search", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.search_bar
                        .activate_action("search.search", None)
                        .unwrap();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("show-search-replace", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.search_bar
                        .activate_action("search.search-replace", None)
                        .unwrap();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("hide-search", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.search_bar
                        .activate_action("search.hide", None)
                        .unwrap();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-bold", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| formatting::format_bold(&this.source_view.buffer())
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-italic", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| formatting::format_italic(&this.source_view.buffer())
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-strikethrough", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| formatting::format_strikethrough(&this.source_view.buffer())
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-highlight", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| formatting::format_highlight(&this.source_view.buffer())
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-heading", Some(VariantTy::INT32));
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, param| {
                    let heading_size: i32 = param.unwrap().get().unwrap();
                    formatting::format_heading(&this.source_view.buffer(), heading_size);
                    this.source_view.grab_focus();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-blockquote", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| formatting::format_blockquote(&this.source_view.buffer())
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-code", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| formatting::format_code(&this.source_view.buffer())
            ));
            actions.add_action(&action);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("close-requested").build(),
                    Signal::builder("saved-as").build(),
                    Signal::builder("stats-changed").build(),
                    Signal::builder("buffer-changed").build(),
                    Signal::builder("toast")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for Editor {}
    impl BinImpl for Editor {}

    impl Editor {
        pub(super) fn setup_filemon(&self) {
            let Some(ref mut file) = *self.file.borrow_mut() else {
                panic!("Editor file uninitialized");
            };
            let filemon = file
                .monitor(FileMonitorFlags::NONE, NOT_CANCELLABLE)
                .expect("Editor: Failed to create file monitor");
            filemon.connect_changed(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _, _, _| {
                    this.file_changed_on_disk.set(true);
                    this.file_changed_on_disk_banner.set_revealed(true);
                }
            ));

            self.file_changed_on_disk.set(false);
            self.filemon.replace(Some(filemon));
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use glib::clone;
use gtk::glib;
use gtk::prelude::*;
use sourceview5::prelude::*;

use gtk::gio::Cancellable;
use gtk::gio::FileCreateFlags;
use gtk::glib::Object;
use sourceview5::SearchContext;
use sourceview5::SearchSettings;
use sourceview5::StyleSchemeManager;

use crate::config::PKGDATADIR;
use crate::data::DocumentStats;
use crate::error::ScratchmarkError;
use crate::util;
use markdown_buffer::MarkdownBuffer;

const NOT_CANCELLABLE: Option<&Cancellable> = None;

glib::wrapper! {
    pub struct Editor(ObjectSubclass<imp::Editor>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Editor {
    pub fn new(path: PathBuf) -> Result<Self, ScratchmarkError> {
        let file = gtk::gio::File::for_path(&path);
        let text = util::read_file_to_string(&file)?;
        let buffer = MarkdownBuffer::default();
        buffer.set_text(&text);

        let search_settings = SearchSettings::default();
        search_settings.set_wrap_around(true);
        let search_context = SearchContext::new(&buffer, Some(&search_settings));

        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        Self::load_buffer_style_scheme(&buffer);
        imp.file.replace(Some(file));
        imp.path.replace(Some(path));
        imp.source_view.set_monospace(true);
        imp.source_view.set_buffer(Some(&buffer));
        imp.search_bar.set_search_context(search_context);
        imp.setup_filemon();
        buffer.connect_changed(clone!(
            #[weak]
            obj,
            move |buffer: &MarkdownBuffer| {
                obj.on_buffer_changed(buffer);
            }
        ));
        imp.source_view.connect_paste_clipboard(clone!(
            #[weak]
            buffer,
            move |_| {
                buffer.open_paste();
            }
        ));
        obj.refresh_document_stats(&buffer);
        Ok(obj)
    }

    pub fn save(&self) -> Result<(), ScratchmarkError> {
        let imp = self.imp();
        if imp.file_changed_on_disk.get() {
            return Err(ScratchmarkError::FileChanged);
        }
        imp.filemon.borrow().as_ref().unwrap().cancel();

        let buffer = imp.source_view.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let text = buffer.text(&start, &end, true).to_string();
        let bytes = text.as_bytes();
        {
            let Some(ref mut file) = *imp.file.borrow_mut() else {
                panic!("Editor file uninitialized");
            };

            let output_stream = file
                .replace(None, false, FileCreateFlags::NONE, NOT_CANCELLABLE)
                .unwrap();

            output_stream.write_all(bytes, NOT_CANCELLABLE).unwrap();
            output_stream.flush(NOT_CANCELLABLE).unwrap();
        }
        imp.setup_filemon();
        self.set_unsaved_changes(false);
        Ok(())
    }

    pub fn path(&self) -> PathBuf {
        let opt = self.imp().path.borrow();
        opt.as_ref().expect("Editor: path uninitialized").clone()
    }

    pub fn set_path(&self, path: PathBuf) {
        let file = gtk::gio::File::for_path(&path);
        self.imp().file.replace(Some(file));
        self.imp().path.replace(Some(path));
        self.imp().setup_filemon();
    }

    /// For preventing "file changed" banner when renaming the file or such.
    pub fn cancel_filemon(&self) {
        self.imp().filemon.borrow().as_ref().unwrap().cancel();
    }

    pub fn set_font(&self, family: &str, size: u32) {
        self.imp().source_view.set_font(family, size);
    }

    pub fn document_stats(&self) -> DocumentStats {
        self.imp().document_stats_data.get()
    }

    fn refresh_document_stats(&self, buffer: &MarkdownBuffer) {
        let imp = self.imp();
        let stats = buffer.stats();
        imp.document_stats.set_stats(&stats);
        imp.document_stats_data.replace(stats);
        self.emit_by_name::<()>("stats-changed", &[]);
    }

    fn load_buffer_style_scheme(buffer: &MarkdownBuffer) {
        let scheme_id = "scratchmark";

        // Try fetching the scheme
        if let Some(style_scheme) = StyleSchemeManager::default().scheme(scheme_id) {
            buffer.set_style_scheme(Some(&style_scheme));
            return;
        }

        // Fetch failed, add paths and try again
        StyleSchemeManager::default().append_search_path(&format!("{PKGDATADIR}/editor_schemes"));
        #[cfg(not(feature = "installed"))]
        {
            const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
            StyleSchemeManager::default()
                .append_search_path(format!("{MANIFEST_DIR}/data/editor_schemes").as_str());
        }

        if let Some(style_scheme) = StyleSchemeManager::default().scheme(scheme_id) {
            buffer.set_style_scheme(Some(&style_scheme));
            return;
        }

        println!("Failed to load scheme with id '{scheme_id}'.")
    }

    fn on_buffer_changed(&self, buffer: &MarkdownBuffer) {
        self.refresh_document_stats(buffer);
        self.set_unsaved_changes(true);
        self.emit_by_name::<()>("buffer-changed", &[]);
    }
}
