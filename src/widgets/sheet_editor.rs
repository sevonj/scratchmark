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
    use gtk::gio::SimpleAction;
    use gtk::glib;

    use adw::AlertDialog;
    use adw::Banner;
    use gio::File;
    use gio::FileMonitor;
    use gio::FileMonitorFlags;
    use gio::SimpleActionGroup;
    use glib::Properties;
    use glib::Regex;
    use glib::RegexCompileFlags;
    use glib::RegexMatchFlags;
    use glib::VariantTy;
    use glib::subclass::Signal;
    use gtk::CompositeTemplate;
    use gtk::CssProvider;
    use gtk::TemplateChild;
    use gtk::TextMark;
    use sourceview5::View;

    use crate::util;
    use crate::widgets::EditorSearchBar;

    use super::NOT_CANCELLABLE;

    #[derive(Debug, Properties, CompositeTemplate, Default)]
    #[properties(wrapper_type = super::SheetEditor)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/sheet_editor.ui")]
    pub struct SheetEditor {
        #[template_child]
        pub(super) source_view: TemplateChild<View>,
        pub(super) source_view_css_provider: CssProvider,

        #[template_child]
        pub(super) search_bar: TemplateChild<EditorSearchBar>,
        #[template_child]
        pub(super) file_changed_banner: TemplateChild<Banner>,

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

            // Deprecated, but the only way to do this at the moment?
            // https://gnome.pages.gitlab.gnome.org/gtksourceview/gtksourceview5/class.View.html#changing-the-font
            #[allow(deprecated)]
            self.source_view.style_context().add_provider(
                &self.source_view_css_provider,
                gtk::ffi::GTK_STYLE_PROVIDER_PRIORITY_USER as u32,
            );

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
                move |_, _| {
                    let buffer = this.source_view.buffer();
                    let Some((start, end)) = buffer.selection_bounds() else {
                        return;
                    };
                    let offset = start.offset();
                    let selection = buffer.text(&start, &end, false);

                    let is_bold = selection.len() >= 4
                        && selection.starts_with("**")
                        && selection.ends_with("**");
                    let is_italic = !is_bold
                        && selection.len() >= 2
                        && selection.starts_with("*")
                        && selection.ends_with("*");

                    let replacement = if is_bold {
                        selection[2..(selection.len() - 2)].to_owned()
                    } else if is_italic {
                        format!("*{selection}*")
                    } else {
                        format!("**{selection}**")
                    };

                    buffer.delete_selection(true, true);
                    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
                    buffer.insert(&mut iter, &replacement);

                    let ins = buffer.iter_at_offset(offset);
                    let bound = buffer.iter_at_offset(offset + replacement.len() as i32);
                    buffer.select_range(&ins, &bound);
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-italic", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    let buffer = this.source_view.buffer();
                    let Some((start, end)) = buffer.selection_bounds() else {
                        return;
                    };
                    let offset = start.offset();
                    let selection = buffer.text(&start, &end, false);

                    let is_bold = selection.len() >= 4
                        && selection.starts_with("**")
                        && selection.ends_with("**");
                    let is_italic = !is_bold
                        && selection.len() >= 2
                        && selection.starts_with("*")
                        && selection.ends_with("*");

                    let replacement = if is_bold || is_italic {
                        selection[1..(selection.len() - 1)].to_owned()
                    } else {
                        format!("*{selection}*")
                    };

                    buffer.delete_selection(true, true);
                    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
                    buffer.insert(&mut iter, &replacement);

                    let ins = buffer.iter_at_offset(offset);
                    let bound = buffer.iter_at_offset(offset + replacement.len() as i32);
                    buffer.select_range(&ins, &bound);
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-heading", Some(VariantTy::INT32));
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, param| {
                    let heading_size: i32 = param.unwrap().get().unwrap();
                    let buffer = this.source_view.buffer();
                    let insert = buffer.get_insert();
                    let insert_iter = buffer.iter_at_mark(&insert);
                    let current_line = insert_iter.line();
                    let Some(mut start) = buffer.iter_at_line(current_line) else {
                        return;
                    };
                    let mut end = buffer
                        .iter_at_line(current_line + 1)
                        .unwrap_or_else(|| buffer.end_iter());
                    if end.line() != current_line {
                        end.backward_char();
                    }

                    let old_line = buffer.text(&start, &end, false);

                    let new_header = String::from("#").repeat(heading_size as usize) + " ";

                    let any_size_heading = Regex::new(
                        "^##* ",
                        RegexCompileFlags::DEFAULT,
                        RegexMatchFlags::DEFAULT,
                    )
                    .unwrap()
                    .unwrap();
                    let any_size_match =
                        any_size_heading.match_(old_line.as_gstr(), RegexMatchFlags::DEFAULT);

                    let replacement = if old_line.starts_with(&new_header) {
                        old_line[(new_header.len())..].to_owned()
                    } else if any_size_match
                        .as_ref()
                        .map(|m| m.matches())
                        .unwrap_or(false)
                    {
                        let old_header_len = any_size_match.unwrap().fetch(0).unwrap().len();
                        let without_header = &old_line[old_header_len..];
                        format!("{new_header}{without_header}")
                    } else {
                        format!("{new_header}{old_line}")
                    };

                    buffer.delete(&mut start, &mut end);
                    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
                    buffer.insert(&mut iter, &replacement);

                    this.source_view.grab_focus();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("format-code", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    let buffer = this.source_view.buffer();
                    let Some((start, end)) = buffer.selection_bounds() else {
                        return;
                    };
                    let offset = start.offset();
                    let selection = buffer.text(&start, &end, false);

                    let is_code = selection.len() >= 2
                        && selection.starts_with("`")
                        && selection.ends_with("`");

                    let replacement = if is_code {
                        selection[1..(selection.len() - 1)].to_owned()
                    } else {
                        format!("`{selection}`")
                    };

                    buffer.delete_selection(true, true);
                    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
                    buffer.insert(&mut iter, &replacement);

                    let ins = buffer.iter_at_offset(offset);
                    let bound = buffer.iter_at_offset(offset + replacement.len() as i32);
                    buffer.select_range(&ins, &bound);
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
use gtk::glib;
use gtk::prelude::*;
use sourceview5::prelude::*;

use gtk::gio::Cancellable;
use gtk::gio::FileCreateFlags;
use gtk::glib::Object;
use sourceview5::Buffer;
use sourceview5::LanguageManager;
use sourceview5::SearchContext;
use sourceview5::SearchSettings;
use sourceview5::StyleSchemeManager;

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
        let file = gtk::gio::File::for_path(&path);
        let text = util::read_file_to_string(&file)?;
        let lang = LanguageManager::default().language("markdown").unwrap();
        let buffer = Buffer::with_language(&lang);
        buffer.set_text(&text);

        let search_settings = SearchSettings::default();
        search_settings.set_wrap_around(true);
        let search_context = SearchContext::new(&buffer, Some(&search_settings));

        let this: Self = Object::builder().build();
        this.load_buffer_style_scheme(&buffer);
        this.imp().file.replace(Some(file));
        this.imp().path.replace(Some(path));
        this.imp().source_view.set_monospace(true);
        this.imp().source_view.set_buffer(Some(&buffer));
        this.imp().search_bar.set_search_context(search_context);
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
        let formatted = format!("textview {{font-family: {family}; font-size: {size}pt;}}");
        self.imp()
            .source_view_css_provider
            .load_from_string(&formatted);
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
