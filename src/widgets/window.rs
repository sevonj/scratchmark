mod imp {
    use std::cell::Cell;
    use std::cell::OnceCell;
    use std::cell::RefCell;
    use std::path::PathBuf;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::closure_local;
    use gtk::glib;

    use adw::AboutDialog;
    use adw::ApplicationWindow;
    use adw::HeaderBar;
    use adw::NavigationPage;
    use adw::OverlaySplitView;
    use adw::Toast;
    use adw::ToastOverlay;
    use adw::ToolbarStyle;
    use adw::ToolbarView;
    use gtk::Builder;
    use gtk::Button;
    use gtk::CompositeTemplate;
    use gtk::EventControllerMotion;
    use gtk::Revealer;
    use gtk::ToggleButton;
    use gtk::gio::Settings;
    use gtk::gio::SettingsBindFlags;
    use gtk::gio::SimpleAction;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib::Properties;
    use gtk::glib::VariantTy;
    use gtk::pango::FontDescription;

    use crate::APP_ID;
    use crate::config;
    use crate::data::Document;
    use crate::data::Folder;
    use crate::data::SortMethod;
    use crate::error::ScratchmarkError;
    use crate::util::file_actions;

    use crate::widgets::Editor;
    use crate::widgets::EditorPlaceholder;
    use crate::widgets::LibraryView;
    use crate::widgets::MarkdownFormatBar;
    use crate::widgets::PreferencesDialog;
    use crate::widgets::WindowTitle;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::Window)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/window.ui")]
    pub struct Window {
        #[template_child]
        top_split: TemplateChild<OverlaySplitView>,

        #[template_child]
        sidebar_page: TemplateChild<NavigationPage>,
        #[template_child]
        sidebar_toolbar_view: TemplateChild<ToolbarView>,
        #[template_child]
        sidebar_toggle: TemplateChild<ToggleButton>,
        /// Bound to setting. Does not directly map to sidebar visibility, because even when this
        /// is true, the sidebar can be hidden by focus mode or too narrow window.
        #[property(get, set)]
        show_sidebar: Cell<bool>,

        #[template_child]
        main_page: TemplateChild<NavigationPage>,
        #[template_child]
        main_toolbar_view: TemplateChild<ToolbarView>,
        #[template_child]
        main_header_revealer: TemplateChild<Revealer>,
        #[template_child]
        main_header_bar: TemplateChild<HeaderBar>,
        #[template_child]
        window_title: TemplateChild<WindowTitle>,

        #[template_child]
        toast_overlay: TemplateChild<ToastOverlay>,
        #[template_child]
        unfullscreen_button: TemplateChild<Button>,

        #[template_child]
        format_bar: TemplateChild<MarkdownFormatBar>,
        #[template_child]
        format_bar_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        editor_sidebar_toggle: TemplateChild<ToggleButton>,

        library_view: LibraryView,
        editor: RefCell<Option<Editor>>,

        motion_controller: EventControllerMotion,

        #[property(get, set)]
        focus_mode_enabled: Cell<bool>,
        #[property(get, set)]
        focus_mode_active: Cell<bool>,
        focus_mode_cursor_position: Cell<(f64, f64)>,

        settings: OnceCell<Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            MarkdownFormatBar::ensure_type();
            WindowTitle::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            #[cfg(debug_assertions)]
            {
                obj.add_css_class("devel");
            }

            let settings = Settings::new(APP_ID);
            #[cfg(not(feature = "generatescreenshots"))]
            {
                settings
                    .bind("win-width", obj.as_ref(), "default-width")
                    .build();
                settings
                    .bind("win-height", obj.as_ref(), "default-height")
                    .build();
                settings
                    .bind("win-is-maximized", obj.as_ref(), "maximized")
                    .build();
                settings
                    .bind("library-show-sidebar", obj.as_ref(), "show-sidebar")
                    .build();
                let editor_sidebar_toggle: &ToggleButton = self.editor_sidebar_toggle.as_ref();
                settings
                    .bind("editor-show-sidebar", editor_sidebar_toggle, "active")
                    .build();
                let format_bar: &MarkdownFormatBar = self.format_bar.as_ref();
                settings
                    .bind("editor-show-formatbar", format_bar, "visible")
                    .build();
                settings
                    .bind("focus-mode-enabled", obj.as_ref(), "focus-mode-enabled")
                    .build();
                let window_title: &WindowTitle = self.window_title.as_ref();
                settings
                    .bind("focus-mode-enabled", window_title, "focus-mode")
                    .build();
                let library_view: &LibraryView = self.library_view.as_ref();
                settings
                    .bind(
                        "library-ignore-hidden-files",
                        library_view,
                        "ignore-hidden-files",
                    )
                    .flags(SettingsBindFlags::GET)
                    .build();
            }
            #[cfg(feature = "generatescreenshots")]
            {
                use gtk::EventControllerKey;
                use gtk::gdk::Key;

                obj.set_size_request(
                    settings.default_value("win-width").unwrap().get().unwrap(),
                    settings.default_value("win-height").unwrap().get().unwrap(),
                );
                obj.set_resizable(false);

                let key_controller = EventControllerKey::new();
                key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
                key_controller.connect_key_pressed(clone!(
                    #[weak]
                    obj,
                    #[upgrade_or]
                    glib::Propagation::Proceed,
                    move |_, key, _, _| {
                        let imp = obj.imp();
                        let sidebar_toggle: &ToggleButton = imp.sidebar_toggle.as_ref();
                        let editor_sidebar_toggle: &ToggleButton =
                            imp.editor_sidebar_toggle.as_ref();
                        let format_bar: &MarkdownFormatBar = imp.format_bar.as_ref();
                        match key {
                            Key::KP_1 => {
                                sidebar_toggle.set_active(true);
                                obj.set_show_sidebar(true);
                                obj.set_focus_mode_active(false);
                                obj.set_focus_mode_enabled(false);
                                editor_sidebar_toggle.set_active(false);
                                format_bar.set_visible(false);
                                if let Some(editor) = imp.editor.borrow().as_ref() {
                                    editor.activate_action("editor.hide-search", None).unwrap();
                                    editor.scroll_to_top();
                                }
                                glib::Propagation::Stop
                            }
                            Key::KP_2 => {
                                sidebar_toggle.set_active(false);
                                obj.set_show_sidebar(false);
                                obj.set_focus_mode_active(true);
                                obj.set_focus_mode_enabled(true);
                                editor_sidebar_toggle.set_active(false);
                                format_bar.set_visible(false);
                                if let Some(editor) = imp.editor.borrow().as_ref() {
                                    editor.activate_action("editor.hide-search", None).unwrap();
                                    editor.scroll_to_top();
                                    editor.scroll_to_line(20);
                                }
                                glib::Propagation::Stop
                            }
                            Key::KP_3 => {
                                sidebar_toggle.set_active(true);
                                obj.set_focus_mode_active(false);
                                obj.set_show_sidebar(true);
                                obj.set_focus_mode_enabled(false);
                                editor_sidebar_toggle.set_active(true);
                                format_bar.set_visible(true);
                                if let Some(editor) = imp.editor.borrow().as_ref() {
                                    editor
                                        .activate_action(
                                            "editor.show-search-with-text",
                                            Some(&"turn".to_variant()),
                                        )
                                        .unwrap();
                                    editor
                                        .activate_action("editor.show-search-replace", None)
                                        .unwrap();
                                    editor.scroll_to_top();
                                    editor
                                        .activate_action("editor.show-search-replace", None)
                                        .unwrap();
                                }
                                glib::Propagation::Stop
                            }
                            _ => glib::Propagation::Proceed,
                        }
                    }
                ));
                obj.add_controller(key_controller);
            }

            obj.connect_notify(Some("focus-mode-enabled"), move |obj, _| {
                let focus_mode_enabled = obj.focus_mode_enabled();
                obj.action_set_enabled("win.enable-focus", !focus_mode_enabled);
                obj.action_set_enabled("win.disable-focus", focus_mode_enabled);
                obj.imp().set_focus_mode_active(focus_mode_enabled)
            });

            self.settings
                .set(settings)
                .expect("`settings` should not be set before calling `setup_settings`.");
            obj.add_controller(self.motion_controller.clone());

            self.motion_controller.connect_motion(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_controller, x, y| {
                    if imp.obj().focus_mode_active() {
                        // Exit focus mode if cursor moved
                        const THRESHOLD: f64 = 100.;
                        let (start_x, start_y) = imp.focus_mode_cursor_position.get();
                        let (delta_x, delta_y) = (x - start_x, y - start_y);
                        if (delta_x * delta_x + delta_y * delta_y).sqrt() > THRESHOLD {
                            imp.set_focus_mode_active(false);
                        }
                    } else {
                        imp.focus_mode_cursor_position.replace((x, y));
                    }
                }
            ));
            self.motion_controller.connect_enter(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_controller, _x, _y| imp.set_focus_mode_active(false)
            ));
            self.motion_controller.connect_leave(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_controller| imp.set_focus_mode_active(false)
            ));

            self.editor_sidebar_toggle.set_sensitive(false);

            let builder = Builder::from_resource("/org/scratchmark/Scratchmark/ui/shortcuts.ui");
            let shortcuts = builder.object("help_overlay").unwrap();
            obj.set_help_overlay(Some(&shortcuts));

            let top_split = self.top_split.get();

            self.library_view.connect_closure(
                "open-document",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: LibraryView, path: PathBuf| {
                        imp.load_document(path);
                    }
                ),
            );

            self.library_view.connect_closure(
                "path-removed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: LibraryView, path: PathBuf| {
                        let contains_edited = imp
                            .editor
                            .borrow()
                            .as_ref()
                            .is_some_and(|e| e.path().starts_with(&path));

                        if contains_edited {
                            if let Some(editor) = imp.editor.borrow().as_ref() {
                                editor.stop_file_monitor();
                                editor.set_file_changed_on_disk(false);
                            }
                            imp.close_editor_without_saving();
                        }
                    }
                ),
            );

            self.library_view.connect_closure(
                "folder-rename-requested",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: LibraryView, folder: Folder, new_path: PathBuf| {
                        assert!(!folder.is_root());

                        let old_path = folder.path();
                        let new_path = file_actions::incremented_path(new_path);
                        let contains_open_document = imp
                            .editor
                            .borrow()
                            .as_ref()
                            .is_some_and(|e| e.path().starts_with(&old_path));

                        if contains_open_document {
                            imp.editor.borrow().as_ref().unwrap().stop_file_monitor();
                        }

                        if let Err(e) = imp.library_view.move_item(old_path, new_path.clone()) {
                            imp.toast(&e.to_string());
                        }

                        if contains_open_document {
                            let open_document_path = imp.editor.borrow().as_ref().unwrap().path();
                            let relative = open_document_path.strip_prefix(folder.path()).unwrap();
                            let doc_path = new_path.join(relative);
                            imp.library_view
                                .set_open_document_path(Some(doc_path.clone()));
                            imp.editor.borrow().as_ref().unwrap().set_path(doc_path);
                        }

                        assert_eq!(
                            imp.library_view.open_document_path(),
                            imp.editor.borrow().as_ref().map(|e| e.path())
                        );

                        imp.update_window_title();
                    }
                ),
            );

            self.library_view.connect_closure(
                "document-rename-requested",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: LibraryView, doc: Document, new_path: PathBuf| {
                        let old_path = doc.path();
                        let new_path = file_actions::incremented_path(new_path);
                        let is_open_in_editor = imp
                            .editor
                            .borrow()
                            .as_ref()
                            .is_some_and(|e| e.path() == doc.path());

                        if is_open_in_editor {
                            imp.editor.borrow().as_ref().unwrap().stop_file_monitor();
                        }

                        if let Err(e) = imp.library_view.move_item(old_path, new_path.clone()) {
                            println!("{e}");
                            imp.toast("Couldn't move file.");
                        }

                        if is_open_in_editor {
                            imp.library_view
                                .set_open_document_path(Some(new_path.clone()));
                            imp.editor.borrow().as_ref().unwrap().set_path(new_path);
                        }

                        assert_eq!(
                            imp.library_view.open_document_path(),
                            imp.editor.borrow().as_ref().map(|e| e.path())
                        );

                        imp.update_window_title();
                    }
                ),
            );

            self.library_view.connect_closure(
                "close-project-requested",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |library_view: LibraryView, project_path: PathBuf| {
                        let contains_edited_file = imp
                            .editor
                            .borrow()
                            .as_ref()
                            .is_some_and(|editor| editor.path().starts_with(&project_path));

                        if contains_edited_file && let Err(e) = imp.close_editor() {
                            imp.toast(&e.to_string());
                            return;
                        }

                        library_view.remove_project(&project_path);
                    }
                ),
            );

            self.library_view.connect_closure(
                "toast",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: LibraryView, msg: String| {
                        imp.toast(&msg);
                    }
                ),
            );

            if !self.top_split.is_collapsed() {
                // Get initial state from setting.
                self.top_split.set_show_sidebar(obj.show_sidebar());
            }

            self.sidebar_toggle.connect_active_notify(clone!(
                #[weak]
                obj,
                move |sidebar_toggle| {
                    // Sidebar toggle clicked
                    if obj.imp().top_split.is_collapsed() {
                        // Window is narrow, sidebar is an overlay. Do not change the setting.
                        return;
                    }
                    if obj.focus_mode_active() {
                        return;
                    }
                    obj.set_show_sidebar(sidebar_toggle.is_active())
                }
            ));

            self.top_split.connect_collapsed_notify(clone!(
                #[weak]
                obj,
                move |top_split| {
                    if !top_split.is_collapsed() {
                        // Sidebar was uncollapsed, get uncollapsed state from setting again.
                        top_split.set_show_sidebar(obj.show_sidebar());
                    }
                }
            ));

            let sidebar_toggle: &ToggleButton = self.sidebar_toggle.as_ref();
            self.top_split
                .bind_property("show-sidebar", sidebar_toggle, "active")
                .bidirectional()
                .sync_create()
                .build();

            let format_bar_toggle: &ToggleButton = self.format_bar_toggle.as_ref();
            self.format_bar
                .bind_property("visible", format_bar_toggle, "active")
                .bidirectional()
                .sync_create()
                .build();

            format_bar_toggle.connect_active_notify(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.update_toolbar_style();
                }
            ));

            self.editor_sidebar_toggle.connect_active_notify(clone!(
                #[weak(rename_to = imp)]
                self,
                move |toggle| {
                    if let Some(editor) = imp.editor.borrow().as_ref() {
                        editor.set_show_sidebar(toggle.is_active());
                    }
                    imp.update_toolbar_style();
                }
            ));

            self.main_toolbar_view
                .set_content(Some(&EditorPlaceholder::default()));
            self.sidebar_toolbar_view
                .set_content(Some(&self.library_view));
            self.update_window_title();

            obj.connect_close_request(clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_| imp.on_close_request()
            ));

            let action = SimpleAction::new("toggle-fullscreen", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_, _| obj.set_fullscreened(!obj.is_fullscreen())
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("unfullscreen", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_, _| obj.unfullscreen()
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("toggle-focus", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_, _| obj.set_focus_mode_enabled(!obj.focus_mode_enabled())
            ));
            obj.add_action(&action);

            obj.connect_fullscreened_notify(clone!(
                #[weak (rename_to = imp)]
                self,
                move |_| imp.update_toolbar_visibility()
            ));
            self.update_toolbar_visibility();
            self.setup_fullscreen_headerbar();

            let action = SimpleAction::new("file-new", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    imp.library_view.prompt_create_document();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("folder-new", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    imp.library_view.prompt_create_subfolder();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("project-add", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    imp.library_view
                        .activate_action("library.project-add", None)
                        .unwrap();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("file-save", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    if let Some(editor) = imp.editor.borrow().as_ref() {
                        if let Err(e) = editor.save() {
                            imp.toast(&e.to_string());
                            return;
                        }
                        imp.toast("Saved");
                    }
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("file-close", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    if let Err(e) = imp.close_editor() {
                        imp.toast(&e.to_string());
                    }
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("file-rename-selected", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    imp.library_view.prompt_rename_selected();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("library-refresh", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    imp.library_view.refresh_content();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("toggle-sidebar", None);
            action.connect_activate(clone!(
                #[weak]
                top_split,
                move |_, _| {
                    let show = !top_split.shows_sidebar();
                    top_split.set_show_sidebar(show);
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("show-about", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_, _| {
                    let builder =
                        Builder::from_resource("/org/scratchmark/Scratchmark/ui/about_dialog.ui");
                    let dialog: AboutDialog = builder.object("dialog").unwrap();
                    dialog.set_version(config::VERSION);
                    dialog.present(Some(&obj));
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("preferences", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _| {
                    imp.show_preferences();
                }
            ));
            obj.add_action(&action);

            let library_actions = SimpleActionGroup::new();
            obj.insert_action_group("library", Some(&library_actions));
            let action = SimpleAction::new_stateful(
                "sort-type",
                Some(VariantTy::STRING),
                &SortMethod::default().to_string().to_variant(),
            );
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |action, param| {
                    let param = param.unwrap();
                    action.set_state(param);
                    imp.library_view
                        .set_sort_method(param.get::<String>().unwrap());
                }
            ));
            library_actions.add_action(&action);

            let editor_actions = SimpleActionGroup::new();
            obj.insert_action_group("editor", Some(&editor_actions));

            fn forward_action_to_editor(
                imp: &Window,
                name: &str,
                parameter_type: Option<&glib::VariantTy>,
                editor_actions: &SimpleActionGroup,
            ) {
                let action = SimpleAction::new(name, parameter_type);
                let name = format!("editor.{name}");
                action.connect_activate(clone!(
                    #[weak]
                    imp,
                    move |_action, param| {
                        if let Some(editor) = imp.editor.borrow().as_ref() {
                            editor.activate_action(&name, param).expect(&name);
                        }
                    }
                ));
                editor_actions.add_action(&action);
            }

            fn forward_heading_action_to_editor(
                imp: &Window,
                name: &str,
                level: i32,
                editor_actions: &SimpleActionGroup,
            ) {
                let action = SimpleAction::new(name, None);
                let name = format!("editor.{name}");
                action.connect_activate(clone!(
                    #[weak]
                    imp,
                    move |_, _| {
                        if let Some(editor) = imp.editor.borrow().as_ref() {
                            editor
                                .activate_action("editor.format-heading", Some(&level.to_variant()))
                                .expect(&name);
                        }
                    }
                ));
                editor_actions.add_action(&action);
            }

            let pi32 = Some(VariantTy::INT32);
            forward_action_to_editor(self, "format-bold", None, &editor_actions);
            forward_action_to_editor(self, "format-italic", None, &editor_actions);
            forward_action_to_editor(self, "format-heading", pi32, &editor_actions);
            forward_heading_action_to_editor(self, "format-h1", 1, &editor_actions);
            forward_heading_action_to_editor(self, "format-h2", 2, &editor_actions);
            forward_heading_action_to_editor(self, "format-h3", 3, &editor_actions);
            forward_heading_action_to_editor(self, "format-h4", 4, &editor_actions);
            forward_heading_action_to_editor(self, "format-h5", 5, &editor_actions);
            forward_heading_action_to_editor(self, "format-h6", 6, &editor_actions);
            forward_action_to_editor(self, "format-code", None, &editor_actions);
            forward_action_to_editor(self, "show-search", None, &editor_actions);
            forward_action_to_editor(self, "show-search-replace", None, &editor_actions);
            forward_action_to_editor(self, "hide-search", None, &editor_actions);
            forward_action_to_editor(self, "shiftreturn", None, &editor_actions);

            obj.connect_map(|obj| {
                obj.imp()
                    .editor_actions_set_enabled(obj.imp().editor.borrow().is_some());
            });

            self.load_state();
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}

    impl Window {
        fn settings(&self) -> &Settings {
            self.settings.get().expect("Settings uninitialized.")
        }

        fn update_window_title(&self) {
            let binding = self.editor.borrow();
            let Some(editor) = binding.as_ref() else {
                self.window_title.set_filename(None::<String>);
                return;
            };
            let filename = editor
                .path()
                .file_stem()
                .map(|d| d.to_string_lossy().to_string());
            self.window_title.set_filename(filename);
        }

        fn update_toolbar_style(&self) {
            let format_bar_open = self.format_bar_toggle.is_active();
            let editor_sidebar_open =
                self.editor.borrow().is_some() && self.editor_sidebar_toggle.is_active();
            let is_fullscreen = self.obj().is_fullscreen();
            let style = if format_bar_open || editor_sidebar_open || is_fullscreen {
                ToolbarStyle::Raised
            } else {
                ToolbarStyle::Flat
            };
            self.main_toolbar_view.set_top_bar_style(style);
        }

        fn set_focus_mode_active(&self, mut active: bool) {
            let obj = self.obj();
            if !obj.focus_mode_enabled() {
                active = false;
            }
            if let Some(editor) = self.editor.borrow().as_ref() {
                editor.set_show_sidebar(self.editor_sidebar_toggle.is_active() && !active);
            } else {
                active = false;
            }
            obj.set_focus_mode_active(active);
            self.top_split
                .set_show_sidebar(obj.show_sidebar() && !active);
            self.update_toolbar_visibility();
            self.update_toolbar_style();
        }

        fn update_toolbar_visibility(&self) {
            let obj = self.obj();
            let focus_mode_active = obj.focus_mode_active();
            let is_fullscreen = obj.is_fullscreen();

            self.main_toolbar_view
                .set_reveal_top_bars(!focus_mode_active);
            self.main_header_revealer.set_reveal_child(!is_fullscreen);

            if is_fullscreen {
                self.unfullscreen_button.set_visible(true);
                self.main_header_bar.set_show_end_title_buttons(false);
                obj.action_set_enabled("win.fullscreen", false);
                obj.action_set_enabled("win.unfullscreen", true);
            } else {
                self.unfullscreen_button.set_visible(false);
                self.main_header_bar.set_show_end_title_buttons(true);
                obj.action_set_enabled("win.fullscreen", true);
                obj.action_set_enabled("win.unfullscreen", false);
            }
            self.update_toolbar_style();
        }

        fn toast(&self, title: &str) {
            self.toast_overlay.add_toast(Toast::new(title));
        }

        fn load_state(&self) {
            #[cfg(not(feature = "generatescreenshots"))]
            {
                let settings = self.settings();

                let selected_item_path = settings.string("selected-item-path");
                if !selected_item_path.is_empty() {
                    self.library_view
                        .set_selected_item_path(Some(PathBuf::from(selected_item_path)));
                } else {
                    self.library_view.set_selected_item_path(None::<PathBuf>);
                }

                let open_projects = settings.strv("library-project-paths");
                for path in open_projects {
                    self.library_view.add_project(PathBuf::from(path));
                }

                let library_expanded_folders = settings.strv("library-expanded-folders");
                for path in library_expanded_folders {
                    self.library_view.make_visible(&PathBuf::from(path));
                }

                let open_document_path = settings.string("open-document-path");
                if !open_document_path.is_empty() {
                    let open_document_path = PathBuf::from(open_document_path);
                    if !open_document_path.exists() {
                        self.toast("Opened document has been moved or deleted.");
                    }
                    self.load_document(open_document_path);
                }
            }
            #[cfg(feature = "generatescreenshots")]
            {
                const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");
                let project_root = PathBuf::from(PROJECT_ROOT);
                self.library_view
                    .add_project(project_root.join("data/demo/Demo Project"));

                self.library_view
                    .make_visible(&project_root.join("data/demo/drafts/Notes"));
                self.library_view.make_visible(
                    &project_root.join("data/demo/Demo Project/Projects/Scratchmark"),
                );

                self.load_document(
                    project_root.join("data/demo/Demo Project/Down the Rabbit Hole.md"),
                );
            }
            self.library_view.refresh_content();
        }

        fn save_state(&self) -> Result<(), glib::BoolError> {
            #[cfg(not(feature = "generatescreenshots"))]
            {
                let settings = self.settings();

                settings.set_string(
                    "selected-item-path",
                    self.library_view
                        .selected_item_path()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap(),
                )?;

                let open_projects = self.library_view.open_projects();
                settings.set_strv("library-project-paths", open_projects)?;

                let expanded_folders = self.library_view.expanded_folders();
                settings.set_strv("library-expanded-folders", expanded_folders)?;

                let open_document_path = self
                    .editor
                    .borrow()
                    .as_ref()
                    .map(|e| e.path())
                    .unwrap_or_default();
                settings.set_string("open-document-path", open_document_path.to_str().unwrap())?;
            }
            Ok(())
        }

        fn load_document(&self, path: PathBuf) {
            self.library_view.set_selected_item_path(Some(path.clone()));

            if let Err(e) = self.close_editor() {
                self.toast(&e.to_string());
                return;
            }

            let editor = match Editor::new(path.clone()) {
                Ok(editor) => editor,
                Err(e) => {
                    self.toast(&e.to_string());
                    self.update_window_title();
                    return;
                }
            };

            let font_family = self.settings().string("editor-font-family");
            let font_size = self.settings().uint("editor-font-size");
            editor.set_font(font_family.as_str(), font_size);

            editor.set_show_sidebar(self.editor_sidebar_toggle.is_active());

            self.editor_sidebar_toggle.set_sensitive(true);

            editor.connect_closure(
                "saved",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: Editor| {
                        imp.library_view.refresh_content();
                    }
                ),
            );

            editor.connect_closure(
                "saved-as",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |editor: Editor| {
                        imp.library_view.set_open_document_path(Some(editor.path()));
                        imp.update_window_title();
                    }
                ),
            );

            editor.connect_closure(
                "buffer-changed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: Editor| {
                        imp.set_focus_mode_active(true);
                    }
                ),
            );

            let window_title: &WindowTitle = self.window_title.as_ref();
            editor
                .bind_property("unsaved-changes", window_title, "unsaved-changes")
                .sync_create()
                .build();
            self.settings()
                .bind("editor-show-minimap", &editor, "show-minimap")
                .flags(SettingsBindFlags::DEFAULT)
                .build();

            self.main_toolbar_view.set_content(Some(&editor));
            self.format_bar.bind_editor(Some(editor.clone()));
            self.editor.replace(Some(editor));
            self.library_view.set_open_document_path(Some(path));
            self.editor_actions_set_enabled(true);
            self.update_window_title();
            self.update_toolbar_style();
        }

        fn close_editor(&self) -> Result<(), ScratchmarkError> {
            if let Some(editor) = self.editor.borrow_mut().as_ref() {
                editor.save()?;
            }
            self.close_editor_without_saving();
            Ok(())
        }

        fn close_editor_without_saving(&self) {
            self.editor.replace(None);
            self.main_toolbar_view
                .set_content(Some(&EditorPlaceholder::default()));
            self.update_window_title();
            self.library_view.set_open_document_path(None::<PathBuf>);
            self.format_bar.bind_editor(None);
            self.editor_sidebar_toggle.set_sensitive(false);
            self.editor_actions_set_enabled(false);
            self.set_focus_mode_active(false);
            self.update_toolbar_style();
        }

        fn editor_actions_set_enabled(&self, enabled: bool) {
            let obj = self.obj();
            obj.action_set_enabled("win.file-save", enabled);
            obj.action_set_enabled("win.file-rename-selected", enabled);
            obj.action_set_enabled("win.file-close", enabled);
            obj.action_set_enabled("editor.format-bold", enabled);
            obj.action_set_enabled("editor.format-italic", enabled);
            obj.action_set_enabled("editor.format-heading", enabled);
            obj.action_set_enabled("editor.format-code", enabled);
            obj.action_set_enabled("editor.show-search", enabled);
            obj.action_set_enabled("editor.show-search-replace", enabled);
            obj.action_set_enabled("editor.hide-search", enabled);
            obj.action_set_enabled("editor.shiftreturn", enabled);
        }

        fn show_preferences(&self) {
            let dialog = PreferencesDialog::new(self.settings().clone());
            dialog.connect_closure(
                "font-changed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: PreferencesDialog, font: FontDescription| {
                        if let Err(e) = imp.set_editor_font(font) {
                            imp.toast(&e.to_string());
                        }
                    }
                ),
            );
            dialog.present(Some(&*self.obj()));
        }

        fn set_editor_font(&self, font: FontDescription) -> Result<(), glib::error::BoolError> {
            let family = font.family().unwrap_or_default();
            let size = font.size() as u32;

            self.settings().set_uint("editor-font-size", size)?;
            self.settings().set_string("editor-font-family", &family)?;

            if let Some(editor) = self.editor.borrow().as_ref() {
                editor.set_font(family.as_str(), size);
            };

            Ok(())
        }

        /// App quit
        fn on_close_request(&self) -> glib::Propagation {
            self.save_state().expect("Failed to save app state");
            if let Err(e) = self.close_editor() {
                self.toast(&e.to_string());
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }

        fn setup_fullscreen_headerbar(&self) {
            self.motion_controller.connect_motion(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_controller, x, y| {
                    if !imp.obj().is_fullscreen() {
                        return;
                    }

                    let root = imp.obj().root().unwrap();
                    let bounds = imp.main_header_bar.compute_bounds(&root).unwrap();
                    let x_start = bounds.x() as f64;
                    let x_end = (bounds.x() + bounds.width()) as f64;

                    if x < x_start || x_end < x {
                        imp.main_header_revealer.set_reveal_child(false);
                        return;
                    }

                    const REVEAL_THRESHOLD: f64 = 50.0;
                    const HIDE_THRESHOLD: f64 = 120.0;
                    let revealed = imp.main_header_revealer.reveals_child();

                    if revealed && y > HIDE_THRESHOLD {
                        imp.main_header_revealer.set_reveal_child(false);
                    } else if !revealed && y < REVEAL_THRESHOLD {
                        imp.main_header_revealer.set_reveal_child(true);
                    }
                }
            ));
        }
    }
}

use gtk::gio;
use gtk::glib;

use glib::Object;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        Object::builder().property("application", app).build()
    }
}
