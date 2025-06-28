mod imp {
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{clone, closure_local};
    use gtk::gio;
    use gtk::glib;
    use sourceview5::prelude::*;

    use adw::{AlertDialog, Banner};
    use gio::{File, FileMonitor, FileMonitorFlags, SimpleActionGroup};
    use glib::Properties;
    use glib::subclass::Signal;
    use gtk::{
        Button, CompositeTemplate, Entry, Label, SearchBar, TemplateChild, TextIter, ToggleButton,
    };
    use sourceview5::{SearchContext, SearchSettings, View};

    use crate::util;

    use super::NOT_CANCELLABLE;

    #[derive(Debug, Properties, CompositeTemplate, Default)]
    #[properties(wrapper_type = super::SheetEditor)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/sheet_editor.ui")]
    pub struct SheetEditor {
        #[template_child]
        pub(super) source_view: TemplateChild<View>,

        #[template_child]
        search_bar: TemplateChild<SearchBar>,
        #[template_child]
        search_entry: TemplateChild<Entry>,
        #[template_child]
        search_replace_entry: TemplateChild<Entry>,
        #[template_child]
        search_occurrences_label: TemplateChild<Label>,
        #[template_child]
        search_prev_button: TemplateChild<Button>,
        #[template_child]
        search_next_button: TemplateChild<Button>,
        #[template_child]
        search_replace_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        search_replace_button: TemplateChild<Button>,
        #[template_child]
        search_replace_all_button: TemplateChild<Button>,
        #[template_child]
        search_replace_buttons_container: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) file_changed_banner: TemplateChild<Banner>,

        pub(super) file: RefCell<Option<File>>,
        pub(super) filemon: RefCell<Option<FileMonitor>>,
        pub(super) path: RefCell<Option<PathBuf>>,

        #[property(get, set)]
        pub(super) file_changed: Cell<bool>,
        search_settings: RefCell<Option<SearchSettings>>,
        search_context: RefCell<Option<SearchContext>>,
        search_position: Cell<Option<i32>>,
        search_occurrences: Cell<Option<i32>>,
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

            self.search_entry.connect_changed(clone!(
                #[weak(rename_to = this)]
                self,
                move |search_entry: &Entry| {
                    this.search_settings
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .set_search_text(Some(&search_entry.text()));
                }
            ));
            self.search_entry.connect_activate(clone!(
                #[weak]
                obj,
                move |_: &Entry| {
                    obj.activate_action("editor.search-next", None).unwrap();
                }
            ));
            self.search_replace_entry.connect_activate(clone!(
                #[weak]
                obj,
                move |_: &Entry| {
                    obj.activate_action("editor.commit-replace", None).unwrap();
                }
            ));

            let search_replace_toggle: &ToggleButton = self.search_replace_toggle.as_ref();
            self.search_replace_buttons_container
                .bind_property("visible", search_replace_toggle, "active")
                .bidirectional()
                .sync_create()
                .build();
            self.search_replace_entry
                .bind_property("visible", search_replace_toggle, "active")
                .bidirectional()
                .sync_create()
                .build();

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

            let action = gio::SimpleAction::new("show-search", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.search_bar.set_search_mode(true);
                    this.search_entry.grab_focus();
                    this.search_context
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .set_highlight(true);
                    this.search_replace_buttons_container.set_visible(false);
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("show-search-replace", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.search_bar.set_search_mode(true);
                    this.search_replace_entry.grab_focus();
                    this.search_context
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .set_highlight(true);
                    this.search_replace_buttons_container.set_visible(true);
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("hide-search", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.search_bar.set_search_mode(false);
                    this.search_context
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .set_highlight(false);
                    this.search_replace_buttons_container.set_visible(false);
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("search-prev", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _| {
                    if this.search_occurrences.get().unwrap_or(0) < 1 {
                        return;
                    }
                    let search_context_bind = this.search_context.borrow();
                    let search_context = search_context_bind.as_ref().unwrap();
                    let mark = search_context.buffer().get_insert();
                    let iter = search_context.buffer().iter_at_mark(&mark);
                    search_context.backward_async(
                        &iter,
                        NOT_CANCELLABLE,
                        clone!(
                            #[weak]
                            this,
                            move |result| {
                                match result {
                                    Ok((start, end, _wrapped)) => {
                                        this.update_search_position(Some((start, end)))
                                    }
                                    Err(_) => this.update_search_position(None),
                                }
                            }
                        ),
                    );
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("search-next", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _| {
                    if this.search_occurrences.get().unwrap_or(0) < 1 {
                        return;
                    }
                    let search_context_bind = this.search_context.borrow();
                    let search_context = search_context_bind.as_ref().unwrap();
                    let mark = search_context.buffer().selection_bound();
                    let iter = search_context.buffer().iter_at_mark(&mark);
                    search_context.forward_async(
                        &iter,
                        NOT_CANCELLABLE,
                        clone!(
                            #[weak]
                            this,
                            move |result| {
                                match result {
                                    Ok((start, end, _wrapped)) => {
                                        this.update_search_position(Some((start, end)))
                                    }
                                    Err(_) => this.update_search_position(None),
                                }
                            }
                        ),
                    );
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("commit-replace", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _| {
                    if this.search_occurrences.get().unwrap_or(0) < 1 {
                        return;
                    }
                    let search_context_bind = this.search_context.borrow();
                    let search_context = search_context_bind.as_ref().unwrap();
                    let mark = search_context.buffer().get_insert();
                    let iter = search_context.buffer().iter_at_mark(&mark);
                    let text = this.search_replace_entry.text();

                    search_context.forward_async(
                        &iter,
                        NOT_CANCELLABLE,
                        clone!(
                            #[weak]
                            this,
                            move |result| {
                                let search_context_bind = this.search_context.borrow();
                                let search_context = search_context_bind.as_ref().unwrap();
                                match result {
                                    Ok((mut match_start, mut match_end, _wrapped)) => {
                                        let _ = search_context.replace(
                                            &mut match_start,
                                            &mut match_end,
                                            &text,
                                        );
                                    }
                                    Err(_) => this.update_search_position(None),
                                }
                            }
                        ),
                    );
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("commit-replace-all", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _| {
                    let search_context_bind = this.search_context.borrow();
                    let search_context = search_context_bind.as_ref().unwrap();
                    let text = this.search_replace_entry.text();
                    let _ = search_context.replace_all(&text);
                }
            ));
            actions.add_action(&action);

            // This action is a workaround to capture <Shift>Return from the Entry
            let action = gio::SimpleAction::new("shiftreturn", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _| {
                    let Some(currently_focused) = this.obj().root().and_then(|r| r.focus()) else {
                        return;
                    };
                    let search_entry: &Entry = this.search_entry.as_ref();
                    let replace_entry: &Entry = this.search_replace_entry.as_ref();
                    if currently_focused.is_ancestor(search_entry) {
                        this.obj()
                            .activate_action("editor.search-prev", None)
                            .unwrap();
                    } else if currently_focused.is_ancestor(replace_entry) {
                        this.obj()
                            .activate_action("editor.commit-replace-all", None)
                            .unwrap();
                    }
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
        pub(super) fn set_search_context(&self, search_context: SearchContext) {
            search_context.connect_occurrences_count_notify(clone!(
                #[weak(rename_to = this)]
                self,
                move |search_context: &SearchContext| {
                    let cnt = search_context.occurrences_count();
                    this.search_occurrences.replace(Some(cnt));

                    let found_any = cnt > 0;
                    this.search_prev_button.set_sensitive(found_any); // TODO: Disable action instead
                    this.search_next_button.set_sensitive(found_any); // TODO: Disable action instead
                    this.search_replace_all_button.set_sensitive(found_any); // TODO: Disable action instead
                    if !found_any {
                        this.update_search_position(None);
                        this.update_search_occurrence_text();
                        return;
                    }

                    let mark = search_context.buffer().get_insert();
                    let iter = search_context.buffer().iter_at_mark(&mark);
                    search_context.forward_async(
                        &iter,
                        NOT_CANCELLABLE,
                        clone!(
                            #[weak]
                            this,
                            move |result| {
                                match result {
                                    Ok((start, end, _wrapped)) => {
                                        this.update_search_position(Some((start, end)))
                                    }
                                    Err(_) => this.update_search_position(None),
                                }
                            }
                        ),
                    );
                    this.update_search_occurrence_text();
                }
            ));
            let search_settings = search_context.settings();
            self.search_context.replace(Some(search_context));
            self.search_settings.replace(Some(search_settings));
        }

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

        fn update_search_position(&self, result: Option<(TextIter, TextIter)>) {
            let Some((match_start, match_end)) = result else {
                self.search_position.replace(None);
                self.update_search_occurrence_text();
                self.search_replace_button.set_sensitive(false);
                return;
            };

            let search_context_bind = self.search_context.borrow();
            let search_context = search_context_bind.as_ref().unwrap();
            let pos = search_context.occurrence_position(&match_start, &match_end);
            self.search_replace_button.set_sensitive(pos >= 1);
            self.search_position.replace(Some(pos));
            self.update_search_occurrence_text();

            search_context
                .buffer()
                .select_range(&match_start, &match_end);

            let mark = search_context.buffer().get_insert();
            self.source_view.scroll_to_mark(&mark, 0.0, false, 0.5, 0.5);
        }

        fn update_search_occurrence_text(&self) {
            let pos = match self.search_position.get() {
                Some(value) if value >= 1 => value.to_string(),
                _ => "?".into(),
            };
            let cnt = match self.search_occurrences.get() {
                Some(value) if value >= 0 => value.to_string(),
                _ => "?".into(),
            };
            self.search_occurrences_label
                .set_text(&format!("{pos} of {cnt}"));
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use sourceview5::prelude::*;

use gio::{Cancellable, FileCreateFlags};
use glib::Object;
use sourceview5::{Buffer, LanguageManager, SearchContext, SearchSettings, StyleSchemeManager};

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

        let search_settings = SearchSettings::default();
        search_settings.set_wrap_around(true);
        let search_context = SearchContext::new(&buffer, Some(&search_settings));

        let this: Self = Object::builder().build();
        this.load_buffer_style_scheme(&buffer);
        this.imp().file.replace(Some(file));
        this.imp().path.replace(Some(path));
        this.imp().source_view.set_monospace(true);
        this.imp().source_view.set_buffer(Some(&buffer));
        this.imp().set_search_context(search_context);
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
