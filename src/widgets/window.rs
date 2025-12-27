mod imp {
    use std::cell::{Cell, OnceCell, RefCell};
    use std::path::PathBuf;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{clone, closure_local};
    use gtk::glib;

    use adw::{
        AboutDialog, AlertDialog, ApplicationWindow, HeaderBar, NavigationPage, OverlaySplitView,
        Toast, ToastOverlay, ToolbarStyle, ToolbarView,
    };
    use gtk::gio::Cancellable;
    use gtk::gio::File;
    use gtk::gio::FileCopyFlags;
    use gtk::gio::Settings;
    use gtk::gio::SettingsBindFlags;
    use gtk::gio::SimpleAction;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib::Properties;
    use gtk::glib::VariantTy;
    use gtk::pango::FontDescription;
    use gtk::{
        Builder, Button, CompositeTemplate, EventControllerMotion, MenuButton, Revealer,
        ToggleButton,
    };

    use crate::APP_ID;
    use crate::config;
    use crate::error::ScratchmarkError;
    use crate::util;

    use crate::widgets::Editor;
    use crate::widgets::EditorFormatBar;
    use crate::widgets::EditorPlaceholder;
    use crate::widgets::ItemCreatePopover;
    use crate::widgets::LibraryBrowser;
    use crate::widgets::LibraryFolder;
    use crate::widgets::LibrarySheet;
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
        sidebar_header_bar: TemplateChild<HeaderBar>,
        #[template_child]
        sidebar_toolbar_view: TemplateChild<ToolbarView>,
        #[template_child]
        sidebar_toggle: TemplateChild<ToggleButton>,
        /// Bound to setting. Does not directly map to sidebar visibility, because even when this
        /// is true, the sidebar can be hidden by focus mode or too narrow window.
        #[property(get, set)]
        sidebar_open: Cell<bool>,

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
        new_folder_button: TemplateChild<MenuButton>,
        #[template_child]
        new_document_button: TemplateChild<MenuButton>,
        #[template_child]
        unfullscreen_button: TemplateChild<Button>,

        #[template_child]
        format_bar: TemplateChild<EditorFormatBar>,
        #[template_child]
        format_bar_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        editor_sidebar_toggle: TemplateChild<ToggleButton>,

        library_browser: LibraryBrowser,
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
            EditorFormatBar::ensure_type();
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
                .bind("library-show-sidebar", obj.as_ref(), "sidebar-open")
                .build();
            let editor_sidebar_toggle: &ToggleButton = self.editor_sidebar_toggle.as_ref();
            settings
                .bind("editor-show-sidebar", editor_sidebar_toggle, "active")
                .build();
            let format_bar: &EditorFormatBar = self.format_bar.as_ref();
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
            let library_browser: &LibraryBrowser = self.library_browser.as_ref();
            settings
                .bind(
                    "library-ignore-hidden-files",
                    library_browser,
                    "ignore-hidden-files",
                )
                .flags(SettingsBindFlags::GET)
                .build();
            settings.connect_changed(
                Some("library-ignore-hidden-files"),
                clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |_, _| {
                        this.library_browser.refresh_content();
                    }
                ),
            );

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
                #[weak(rename_to = this)]
                self,
                move |_controller, x, y| {
                    if this.obj().focus_mode_active() {
                        // Exit focus mode if cursor moved
                        const THRESHOLD: f64 = 100.;
                        let (start_x, start_y) = this.focus_mode_cursor_position.get();
                        let (delta_x, delta_y) = (x - start_x, y - start_y);
                        if (delta_x * delta_x + delta_y * delta_y).sqrt() > THRESHOLD {
                            this.set_focus_mode_active(false);
                        }
                    } else {
                        this.focus_mode_cursor_position.replace((x, y));
                    }
                }
            ));
            self.motion_controller.connect_enter(clone!(
                #[weak(rename_to = this)]
                self,
                move |_controller, _x, _y| this.set_focus_mode_active(false)
            ));
            self.motion_controller.connect_leave(clone!(
                #[weak(rename_to = this)]
                self,
                move |_controller| this.set_focus_mode_active(false)
            ));

            self.editor_sidebar_toggle.set_sensitive(false);

            let builder = Builder::from_resource("/org/scratchmark/Scratchmark/ui/shortcuts.ui");
            let shortcuts = builder.object("help_overlay").unwrap();
            obj.set_help_overlay(Some(&shortcuts));

            let top_split = self.top_split.get();

            self.library_browser.connect_closure(
                "document-selected",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryBrowser, path: PathBuf| {
                        this.load_sheet(path);
                    }
                ),
            );

            self.library_browser.connect_closure(
                "folder-trash-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryBrowser, folder: LibraryFolder| {
                        obj.imp().trash_folder(folder);
                    }
                ),
            );

            self.library_browser.connect_closure(
                "document-trash-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryBrowser, sheet: LibrarySheet| {
                        obj.imp().trash_sheet(sheet);
                    }
                ),
            );

            self.library_browser.connect_closure(
                "folder-delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryBrowser, folder: LibraryFolder| {
                        let heading = "Delete folder?";
                        let body = format!(
                            "Are you sure you want to permanently delete {}?",
                            folder.name()
                        );
                        let dialog = AlertDialog::new(Some(heading), Some(&body));
                        dialog.add_response("cancel", "Cancel");
                        dialog.add_response("commit-delete", "Delete");
                        dialog.set_response_appearance(
                            "commit-delete",
                            adw::ResponseAppearance::Destructive,
                        );
                        dialog.connect_closure(
                            "response",
                            false,
                            closure_local!(
                                #[weak]
                                obj,
                                #[weak]
                                folder,
                                move |_: AlertDialog, response: String| {
                                    if response == "commit-delete" {
                                        obj.imp().delete_folder(folder);
                                    }
                                }
                            ),
                        );
                        dialog.present(Some(&obj));
                    }
                ),
            );

            self.library_browser.connect_closure(
                "folder-rename-requested",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_browser: LibraryBrowser, folder: LibraryFolder, new_path: PathBuf| {
                        assert!(!folder.is_root());

                        let original_path = folder.path();
                        let new_path = util::incremented_path(new_path);
                        let selected_item_path = this.library_browser.selected_item_path();

                        let editor_opt = this.editor.borrow();
                        let open_document_affected = editor_opt
                            .as_ref()
                            .is_some_and(|e| e.path().starts_with(&original_path));
                        let selected_item_affected = selected_item_path.starts_with(&original_path);

                        if open_document_affected {
                            editor_opt.as_ref().unwrap().cancel_filemon();
                        }

                        let new_folder = File::for_path(&new_path);
                        let old_folder = File::for_path(&original_path);
                        if old_folder
                            .move_(&new_folder, FileCopyFlags::NONE, None::<&Cancellable>, None)
                            .is_err()
                            && let Err(e) = util::move_folder(&original_path, &new_path)
                        {
                            this.toast(&e.to_string());
                        }

                        if open_document_affected {
                            let open_document_path = editor_opt.as_ref().unwrap().path();
                            let relative = open_document_path.strip_prefix(folder.path()).unwrap();
                            let sheet_path = new_path.join(relative);
                            this.library_browser
                                .set_open_document_path(Some(sheet_path.clone()));
                            editor_opt.as_ref().unwrap().set_path(sheet_path);
                        }
                        if selected_item_affected {
                            let relative = selected_item_path.strip_prefix(folder.path()).unwrap();
                            let new_selected_path = new_path.join(relative);
                            this.library_browser
                                .set_selected_item_path(new_selected_path);
                        }

                        assert_eq!(
                            this.library_browser.open_document_path(),
                            this.editor.borrow().as_ref().map(|e| e.path())
                        );

                        this.library_browser.refresh_content();
                        this.update_window_title();
                    }
                ),
            );

            self.library_browser.connect_closure(
                "document-rename-requested",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_browser: LibraryBrowser, sheet: LibrarySheet, new_path: PathBuf| {
                        let original_path = sheet.path();
                        let new_path = util::incremented_path(new_path);

                        let editor_opt = this.editor.borrow();
                        let open_sheet_affected = editor_opt
                            .as_ref()
                            .is_some_and(|e| e.path() == sheet.path());
                        if open_sheet_affected {
                            editor_opt.as_ref().unwrap().cancel_filemon();
                        }
                        let new_file = File::for_path(&new_path);
                        if let Err(e) = File::for_path(&original_path).move_(
                            &new_file,
                            FileCopyFlags::NONE,
                            None::<&Cancellable>,
                            None,
                        ) {
                            println!("{e}");
                            this.toast("Couldn't move file.");
                        }
                        if open_sheet_affected {
                            this.library_browser
                                .set_open_document_path(Some(new_path.clone()));
                            editor_opt.as_ref().unwrap().set_path(new_path);
                        }

                        assert_eq!(
                            this.library_browser.open_document_path(),
                            this.editor.borrow().as_ref().map(|e| e.path())
                        );

                        this.library_browser.refresh_content();
                        this.update_window_title();
                    }
                ),
            );

            self.library_browser.connect_closure(
                "document-delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryBrowser, sheet: LibrarySheet| {
                        let heading = "Delete sheet?";
                        let body = format!(
                            "Are you sure you want to permanently delete {}?",
                            sheet.stem()
                        );
                        let dialog = AlertDialog::new(Some(heading), Some(&body));
                        dialog.add_response("cancel", "Cancel");
                        dialog.add_response("commit-delete", "Delete");
                        dialog.set_response_appearance(
                            "commit-delete",
                            adw::ResponseAppearance::Destructive,
                        );
                        dialog.connect_closure(
                            "response",
                            false,
                            closure_local!(
                                #[weak]
                                obj,
                                #[weak]
                                sheet,
                                move |_: AlertDialog, response: String| {
                                    if response == "commit-delete" {
                                        obj.imp().delete_sheet(sheet);
                                    }
                                }
                            ),
                        );
                        dialog.present(Some(&obj));
                    }
                ),
            );

            self.library_browser.connect_closure(
                "close-project-requested",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |browser: LibraryBrowser, project_path: PathBuf| {
                        let contains_edited_file = this
                            .editor
                            .borrow()
                            .as_ref()
                            .is_some_and(|editor| editor.path().starts_with(&project_path));

                        if contains_edited_file && let Err(e) = this.close_editor() {
                            this.toast(&e.to_string());
                            return;
                        }

                        browser.remove_project(&project_path);
                    }
                ),
            );

            self.library_browser.connect_closure(
                "notify-err",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_browser: LibraryBrowser, msg: String| {
                        this.toast(&msg);
                    }
                ),
            );

            let new_folder_popover = ItemCreatePopover::for_folder();
            self.new_folder_button
                .set_popover(Some(&new_folder_popover));
            new_folder_popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_popover: ItemCreatePopover, path: PathBuf| this.create_folder(path)
                ),
            );
            self.library_browser
                .bind_property(
                    "selected-item-path",
                    &new_folder_popover,
                    "selected-item-path",
                )
                .build();

            let new_document_popover = ItemCreatePopover::for_document();
            self.new_document_button
                .set_popover(Some(&new_document_popover));
            new_document_popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_popover: ItemCreatePopover, path: PathBuf| this.create_sheet(path)
                ),
            );
            self.library_browser
                .bind_property(
                    "selected-item-path",
                    &new_document_popover,
                    "selected-item-path",
                )
                .build();

            if !self.top_split.is_collapsed() {
                // Get initial state from setting.
                self.top_split.set_show_sidebar(obj.sidebar_open());
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
                    obj.set_sidebar_open(sidebar_toggle.is_active())
                }
            ));

            self.top_split.connect_collapsed_notify(clone!(
                #[weak]
                obj,
                move |top_split| {
                    if !top_split.is_collapsed() {
                        // Sidebar was uncollapsed, get uncollapsed state from setting again.
                        top_split.set_show_sidebar(obj.sidebar_open());
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
                #[weak(rename_to = this)]
                self,
                move |_| {
                    this.update_toolbar_style();
                }
            ));

            self.editor_sidebar_toggle.connect_active_notify(clone!(
                #[weak(rename_to = this)]
                self,
                move |toggle| {
                    if let Some(editor) = this.editor.borrow().as_ref() {
                        editor.set_show_sidebar(toggle.is_active());
                    }
                    this.update_toolbar_style();
                }
            ));

            self.main_toolbar_view
                .set_content(Some(&EditorPlaceholder::default()));
            self.sidebar_toolbar_view
                .set_content(Some(&self.library_browser));
            self.update_window_title();

            obj.connect_close_request(clone!(
                #[weak(rename_to = this)]
                self,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_| this.on_close_request()
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
                #[weak (rename_to = this)]
                self,
                move |_| this.update_toolbar_visibility()
            ));
            self.update_toolbar_visibility();
            self.setup_fullscreen_headerbar();

            let action = SimpleAction::new("file-new", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.new_document_button.popup();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("project-add", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.library_browser
                        .activate_action("library.project-add", None)
                        .unwrap();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("file-save", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.save_sheet();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("file-close", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    if let Err(e) = this.close_editor() {
                        this.toast(&e.to_string());
                    }
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("file-rename-selected", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.library_browser.prompt_rename_selected();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("library-refresh", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.library_browser.refresh_content();
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
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.show_about();
                }
            ));
            obj.add_action(&action);

            let action = SimpleAction::new("preferences", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.show_preferences();
                }
            ));
            obj.add_action(&action);

            let editor_actions = SimpleActionGroup::new();
            obj.insert_action_group("editor", Some(&editor_actions));

            fn forward_action_to_editor(
                this: &Window,
                name: &str,
                parameter_type: Option<&glib::VariantTy>,
                editor_actions: &SimpleActionGroup,
            ) {
                let action = SimpleAction::new(name, parameter_type);
                let name = format!("editor.{name}");
                action.connect_activate(clone!(
                    #[weak]
                    this,
                    move |_action, param| {
                        if let Some(editor) = this.editor.borrow().as_ref() {
                            editor.activate_action(&name, param).expect(&name);
                        }
                    }
                ));
                editor_actions.add_action(&action);
            }

            let pi32 = Some(VariantTy::INT32);
            forward_action_to_editor(self, "format-bold", None, &editor_actions);
            forward_action_to_editor(self, "format-italic", None, &editor_actions);
            forward_action_to_editor(self, "format-heading", pi32, &editor_actions);
            forward_action_to_editor(self, "format-code", None, &editor_actions);
            forward_action_to_editor(self, "show-search", None, &editor_actions);
            forward_action_to_editor(self, "show-search-replace", None, &editor_actions);
            forward_action_to_editor(self, "hide-search", None, &editor_actions);
            forward_action_to_editor(self, "shiftreturn", None, &editor_actions);

            obj.connect_map(|this| {
                this.imp()
                    .editor_actions_set_enabled(this.imp().editor.borrow().is_some());
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
                .set_show_sidebar(obj.sidebar_open() && !active);
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
            let settings = self.settings();

            let open_document_path = settings.string("open-document-path");
            if !open_document_path.is_empty() {
                let open_sheet_path = PathBuf::from(open_document_path);
                if !open_sheet_path.exists() {
                    self.toast("Opened sheet has been moved or deleted.");
                }
                self.load_sheet(open_sheet_path);
            }
            self.library_browser
                .set_selected_item_from_last_session(Some(PathBuf::from(
                    settings.string("selected-folder-path"),
                )));
            self.library_browser.refresh_content();
            let open_projects = settings.strv("library-project-paths");
            for path in open_projects {
                self.library_browser.add_project(PathBuf::from(path));
            }
            let library_expanded_folders = settings.strv("library-expanded-folders");
            for path in library_expanded_folders {
                self.library_browser.expand_folder(PathBuf::from(path));
            }
        }

        fn save_state(&self) -> Result<(), glib::BoolError> {
            let settings = self.settings();

            let open_document_path = self
                .editor
                .borrow()
                .as_ref()
                .map(|e| e.path())
                .unwrap_or_default();
            settings.set_string("open-document-path", open_document_path.to_str().unwrap())?;
            settings.set_string(
                "selected-folder-path",
                self.library_browser.selected_item_path().to_str().unwrap(),
            )?;
            let expanded_folders = self.library_browser.expanded_folder_paths();
            settings.set_strv("library-expanded-folders", expanded_folders)?;
            let open_projects = self.library_browser.open_project_paths();
            settings.set_strv("library-project-paths", open_projects)?;

            Ok(())
        }

        fn create_folder(&self, path: PathBuf) {
            if let Err(e) = util::create_folder(&path) {
                self.toast(&e.to_string());
                self.library_browser.refresh_content();
                return;
            }
            self.library_browser.refresh_content();
            self.library_browser
                .get_folder(&util::path_builtin_library())
                .unwrap()
                .set_expanded(true);
        }

        fn create_sheet(&self, path: PathBuf) {
            if let Err(e) = self.close_editor() {
                self.toast(&e.to_string());
                return;
            }
            if let Err(e) = util::create_sheet_file(&path) {
                self.toast(&e.to_string());
                self.library_browser.refresh_content();
                return;
            }
            self.library_browser.refresh_content();
            self.load_sheet(path);
            self.library_browser
                .get_folder(&util::path_builtin_library())
                .unwrap()
                .set_expanded(true);
        }

        fn load_sheet(&self, path: PathBuf) {
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
                "close-requested",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: Editor| {
                        if let Err(e) = this.close_editor() {
                            this.toast(&e.to_string());
                            return;
                        }
                    }
                ),
            );

            editor.connect_closure(
                "saved-as",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |editor: Editor| {
                        this.library_browser.refresh_content();
                        this.library_browser
                            .set_open_document_path(Some(editor.path()));
                        this.update_window_title();
                    }
                ),
            );

            editor.connect_closure(
                "buffer-changed",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: Editor| {
                        this.set_focus_mode_active(true);
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
            self.library_browser.set_open_document_path(Some(path));
            self.editor_actions_set_enabled(true);
            self.update_window_title();
            self.update_toolbar_style();
        }

        fn trash_folder(&self, folder: LibraryFolder) {
            assert!(!folder.is_root());

            let path = folder.path();
            let parent_of_currently_open = self
                .editor
                .borrow()
                .as_ref()
                .is_some_and(|e| e.path().starts_with(&path));
            if parent_of_currently_open && let Err(e) = self.close_editor() {
                self.toast(&e.to_string());
                return;
            }
            if let Err(e) = File::for_path(path).trash(None::<&Cancellable>) {
                println!("{e}");
                self.toast("Couldn't move to trash.");
                return;
            }
            self.toast("Moved to trash");
            self.library_browser.refresh_content();
        }

        fn trash_sheet(&self, sheet: LibrarySheet) {
            let path = sheet.path();
            let currently_open = self
                .editor
                .borrow()
                .as_ref()
                .is_some_and(|e| e.path() == path);
            if currently_open && let Err(e) = self.close_editor() {
                self.toast(&e.to_string());
                return;
            }
            if let Err(e) = File::for_path(path).trash(None::<&Cancellable>) {
                println!("{e}");
                self.toast("Couldn't move to trash.");
                return;
            }
            self.toast("Moved to trash");
            self.library_browser.refresh_content();
        }

        fn delete_folder(&self, folder: LibraryFolder) {
            assert!(!folder.is_root());

            let path = folder.path();
            let parent_of_currently_open = self
                .editor
                .borrow()
                .as_ref()
                .is_some_and(|e| e.path().starts_with(&path));
            if parent_of_currently_open && let Err(e) = self.close_editor() {
                self.toast(&e.to_string());
                return;
            }
            if let Err(e) = std::fs::remove_dir_all(path) {
                println!("{e}");
                self.toast("Couldn't delete folder.");
            }
            self.library_browser.refresh_content();
        }

        fn delete_sheet(&self, sheet: LibrarySheet) {
            let path = sheet.path();
            let currently_open = self
                .editor
                .borrow()
                .as_ref()
                .is_some_and(|e| e.path() == path);
            if currently_open && let Err(e) = self.close_editor() {
                self.toast(&e.to_string());
                return;
            }
            if let Err(e) = std::fs::remove_file(path) {
                println!("{e}");
                self.toast("Couldn't delete file.");
            }
            self.library_browser.refresh_content();
        }

        fn save_sheet(&self) {
            let mut editor_bind = self.editor.borrow_mut();
            let Some(editor) = editor_bind.as_mut() else {
                return;
            };
            if let Err(e) = editor.save() {
                self.toast(&e.to_string());
                return;
            }
            self.toast("Saved");
        }

        fn close_editor(&self) -> Result<(), ScratchmarkError> {
            if let Some(editor) = self.editor.borrow_mut().as_ref() {
                editor.save()?;
            }
            self.editor.replace(None);

            self.main_toolbar_view
                .set_content(Some(&EditorPlaceholder::default()));
            self.update_window_title();
            self.library_browser.set_open_document_path(None);
            self.format_bar.bind_editor(None);
            self.editor_sidebar_toggle.set_sensitive(false);
            self.editor_actions_set_enabled(false);
            self.set_focus_mode_active(false);
            self.update_toolbar_style();
            Ok(())
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

        fn show_about(&self) {
            let obj = self.obj();
            let builder = Builder::from_resource("/org/scratchmark/Scratchmark/ui/about_dialog.ui");
            let dialog: AboutDialog = builder.object("dialog").unwrap();
            dialog.set_version(config::VERSION);
            dialog.present(Some(&*obj));
        }

        fn show_preferences(&self) {
            let dialog = PreferencesDialog::new(self.settings().clone());
            dialog.connect_closure(
                "font-changed",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: PreferencesDialog, font: FontDescription| {
                        if let Err(e) = this.set_editor_font(font) {
                            this.toast(&e.to_string());
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
                #[weak(rename_to = this)]
                self,
                move |_controller, x, y| {
                    if !this.obj().is_fullscreen() {
                        return;
                    }

                    let root = this.obj().root().unwrap();
                    let bounds = this.main_header_bar.compute_bounds(&root).unwrap();
                    let x_start = bounds.x() as f64;
                    let x_end = (bounds.x() + bounds.width()) as f64;

                    if x < x_start || x_end < x {
                        this.main_header_revealer.set_reveal_child(false);
                        return;
                    }

                    const REVEAL_THRESHOLD: f64 = 50.0;
                    const HIDE_THRESHOLD: f64 = 120.0;
                    let revealed = this.main_header_revealer.reveals_child();

                    if revealed && y > HIDE_THRESHOLD {
                        this.main_header_revealer.set_reveal_child(false);
                    } else if !revealed && y < REVEAL_THRESHOLD {
                        this.main_header_revealer.set_reveal_child(true);
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
