mod imp {
    use std::cell::{Cell, OnceCell, RefCell};
    use std::fs;
    use std::path::PathBuf;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{clone, closure_local};
    use gtk::gio;
    use gtk::glib;
    use gtk::pango;

    use adw::{
        AboutDialog, AlertDialog, ApplicationWindow, HeaderBar, NavigationPage, OverlaySplitView,
        Toast, ToastOverlay, ToolbarStyle, ToolbarView,
    };
    use gio::{Cancellable, Settings, SimpleAction, SimpleActionGroup};
    use glib::VariantTy;
    use gtk::{
        Builder, Button, CompositeTemplate, EventControllerMotion, FontDialog, MenuButton,
        Revealer, ToggleButton,
    };
    use pango::FontDescription;

    use crate::APP_ID;
    use crate::error::ScratchmarkError;
    use crate::util;

    use crate::widgets::EditorFormatBar;
    use crate::widgets::EditorPlaceholder;
    use crate::widgets::ItemCreatePopover;
    use crate::widgets::LibraryBrowser;
    use crate::widgets::LibraryFolder;
    use crate::widgets::LibrarySheet;
    use crate::widgets::SheetEditor;

    #[derive(CompositeTemplate, Default)]
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
        sidebar_uncollapsed_open: Cell<bool>,

        #[template_child]
        main_page: TemplateChild<NavigationPage>,
        #[template_child]
        main_toolbar_view: TemplateChild<ToolbarView>,
        #[template_child]
        main_header_revealer: TemplateChild<Revealer>,
        #[template_child]
        main_header_bar: TemplateChild<HeaderBar>,

        #[template_child]
        toast_overlay: TemplateChild<ToastOverlay>,
        #[template_child]
        new_folder_button: TemplateChild<MenuButton>,
        #[template_child]
        new_sheet_button: TemplateChild<MenuButton>,
        #[template_child]
        unfullscreen_button: TemplateChild<Button>,

        #[template_child]
        format_bar: TemplateChild<EditorFormatBar>,
        #[template_child]
        format_bar_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        editor_sidebar_toggle: TemplateChild<ToggleButton>,

        library_browser: LibraryBrowser,
        sheet_editor: RefCell<Option<SheetEditor>>,

        settings: OnceCell<Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            EditorFormatBar::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

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
                .bind("is-maximized", obj.as_ref(), "maximized")
                .build();
            self.settings.set(settings).expect(
                "`settings` should not be set before calling `setup_settings`.
                ",
            );

            self.editor_sidebar_toggle.set_sensitive(false);

            let builder = Builder::from_resource("/org/scratchmark/Scratchmark/ui/shortcuts.ui");
            let shortcuts = builder.object("help_overlay").unwrap();
            obj.set_help_overlay(Some(&shortcuts));

            let top_split = self.top_split.get();

            self.library_browser.connect_closure(
                "sheet-selected",
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
                "sheet-trash-requested",
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

                        let sheet_editor_opt = this.sheet_editor.borrow();
                        let open_sheet_affected = sheet_editor_opt
                            .as_ref()
                            .is_some_and(|e| e.path().starts_with(&original_path));
                        if open_sheet_affected {
                            sheet_editor_opt.as_ref().unwrap().cancel_filemon();
                        }
                        fs::rename(&original_path, &new_path).expect("Folder rename failed");
                        if open_sheet_affected {
                            let selected_sheet = sheet_editor_opt.as_ref().unwrap().path();
                            let relative = selected_sheet.strip_prefix(folder.path()).unwrap();
                            let sheet_path = new_path.join(relative);
                            this.library_browser
                                .set_selected_sheet(Some(sheet_path.clone()));
                            sheet_editor_opt.as_ref().unwrap().set_path(sheet_path);
                        }

                        assert_eq!(
                            this.library_browser.selected_sheet(),
                            this.sheet_editor.borrow().as_ref().map(|e| e.path())
                        );

                        this.library_browser.refresh_content();
                        this.update_window_title();
                    }
                ),
            );

            self.library_browser.connect_closure(
                "sheet-rename-requested",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_browser: LibraryBrowser, sheet: LibrarySheet, new_path: PathBuf| {
                        let original_path = sheet.path();
                        let new_path = util::incremented_path(new_path);

                        let sheet_editor_opt = this.sheet_editor.borrow();
                        let open_sheet_affected = sheet_editor_opt
                            .as_ref()
                            .is_some_and(|e| e.path() == sheet.path());
                        if open_sheet_affected {
                            sheet_editor_opt.as_ref().unwrap().cancel_filemon();
                        }
                        fs::rename(&original_path, &new_path).expect("File rename failed");
                        if open_sheet_affected {
                            this.library_browser
                                .set_selected_sheet(Some(new_path.clone()));
                            sheet_editor_opt.as_ref().unwrap().set_path(new_path);
                        }

                        assert_eq!(
                            this.library_browser.selected_sheet(),
                            this.sheet_editor.borrow().as_ref().map(|e| e.path())
                        );

                        this.library_browser.refresh_content();
                        this.update_window_title();
                    }
                ),
            );

            self.library_browser.connect_closure(
                "sheet-delete-requested",
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

            let new_sheet_popover = ItemCreatePopover::for_sheet();
            self.new_sheet_button.set_popover(Some(&new_sheet_popover));
            new_sheet_popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_popover: ItemCreatePopover, path: PathBuf| this.create_sheet(path)
                ),
            );

            let sidebar_toggle: &ToggleButton = self.sidebar_toggle.as_ref();
            self.top_split
                .bind_property("show-sidebar", sidebar_toggle, "active")
                .bidirectional()
                .sync_create()
                .build();

            self.top_split.connect_collapsed_notify(clone!(
                #[weak (rename_to = this)]
                self,
                move |top_split| {
                    if !top_split.is_collapsed() {
                        top_split.set_show_sidebar(this.sidebar_uncollapsed_open.get());
                    }
                }
            ));

            self.sidebar_toggle.connect_active_notify(clone!(
                #[weak (rename_to = this)]
                self,
                move |sidebar_toggle| {
                    if !this.top_split.is_collapsed() {
                        this.sidebar_uncollapsed_open
                            .replace(sidebar_toggle.is_active());
                    }
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

            let action_fullscreen = gio::SimpleAction::new("fullscreen", None);
            action_fullscreen.connect_activate(clone!(
                #[weak]
                obj,
                move |_, _| obj.fullscreen()
            ));
            obj.add_action(&action_fullscreen);

            let action_unfullscreen = gio::SimpleAction::new("unfullscreen", None);
            action_unfullscreen.connect_activate(clone!(
                #[weak]
                obj,
                move |_, _| obj.unfullscreen()
            ));
            obj.add_action(&action_unfullscreen);

            obj.connect_fullscreened_notify(clone!(
                #[weak (rename_to = this)]
                self,
                #[weak]
                action_fullscreen,
                #[weak]
                action_unfullscreen,
                move |_| this.on_fullscreen_changed(action_fullscreen, action_unfullscreen)
            ));
            self.on_fullscreen_changed(action_fullscreen, action_unfullscreen);
            self.setup_fullscreen_headerbar();

            let action = gio::SimpleAction::new("file-new", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.new_sheet_button.popup();
                }
            ));
            obj.add_action(&action);

            let action = gio::SimpleAction::new("file-save", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.save_sheet();
                }
            ));
            obj.add_action(&action);

            let action = gio::SimpleAction::new("file-close", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    if let Err(e) = this.close_editor() {
                        let toast = Toast::new(&e.to_string());
                        this.toast_overlay.add_toast(toast);
                    }
                }
            ));
            obj.add_action(&action);

            let action = gio::SimpleAction::new("file-rename-open", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.library_browser.rename_selected_sheet();
                }
            ));
            obj.add_action(&action);

            let action = gio::SimpleAction::new("library-refresh", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.library_browser.refresh_content();
                }
            ));
            obj.add_action(&action);

            let action = gio::SimpleAction::new("toggle-sidebar", None);
            action.connect_activate(clone!(
                #[weak]
                top_split,
                move |_, _| {
                    let show = !top_split.shows_sidebar();
                    top_split.set_show_sidebar(show);
                }
            ));
            obj.add_action(&action);

            let action = gio::SimpleAction::new("show-about", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.show_about();
                }
            ));
            obj.add_action(&action);

            let action = gio::SimpleAction::new("show-font-dialog", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.show_font_dialog();
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
                let action = gio::SimpleAction::new(name, parameter_type);
                let name = format!("editor.{name}");
                action.connect_activate(clone!(
                    #[weak]
                    this,
                    move |_action, param| {
                        let sheet_editor_opt = this.sheet_editor.borrow();
                        if let Some(sheet_editor) = sheet_editor_opt.as_ref() {
                            sheet_editor.activate_action(&name, param).expect(&name);
                        }
                    }
                ));
                editor_actions.add_action(&action);
            }

            forward_action_to_editor(self, "format-bold", None, &editor_actions);
            forward_action_to_editor(self, "format-italic", None, &editor_actions);
            forward_action_to_editor(
                self,
                "format-heading",
                Some(VariantTy::INT32),
                &editor_actions,
            );
            forward_action_to_editor(self, "format-code", None, &editor_actions);
            forward_action_to_editor(self, "show-search", None, &editor_actions);
            forward_action_to_editor(self, "show-search-replace", None, &editor_actions);
            forward_action_to_editor(self, "hide-search", None, &editor_actions);
            forward_action_to_editor(self, "shiftreturn", None, &editor_actions);

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
            if let Some(editor) = self.sheet_editor.borrow().as_ref()
                && let Some(stem) = editor.path().file_stem()
            {
                self.main_page.set_title(&stem.to_string_lossy());
                return;
            };
            self.main_page.set_title("Scratchmark");
        }

        fn update_toolbar_style(&self) {
            let format_bar_open = self.format_bar_toggle.is_active();
            let editor_sidebar_open = self.editor_sidebar_toggle.is_active();
            let style = if format_bar_open || editor_sidebar_open {
                ToolbarStyle::Raised
            } else {
                ToolbarStyle::Flat
            };
            self.main_toolbar_view.set_top_bar_style(style);
        }

        fn load_state(&self) {
            let settings = self.settings();

            let open_sheet_path = settings.string("open-sheet-path");
            if !open_sheet_path.is_empty() {
                let open_sheet_path = PathBuf::from(open_sheet_path);
                if !open_sheet_path.exists() {
                    let toast = Toast::new("Last open sheet has been moved or deleted.");
                    self.toast_overlay.add_toast(toast);
                }
                self.load_sheet(open_sheet_path);
            }

            let show_sidebar = settings.boolean("library-sidebar-open");
            self.sidebar_uncollapsed_open.replace(show_sidebar);
            self.top_split.set_show_sidebar(show_sidebar);
            self.format_bar
                .set_visible(settings.boolean("editor-formatbar-open"));
            self.editor_sidebar_toggle
                .set_active(settings.boolean("editor-sidebar-open"));

            let library_expanded_folders = settings.strv("library-expanded-folders");
            for path in library_expanded_folders {
                if let Some(folder) = self.library_browser.get_folder(&PathBuf::from(path)) {
                    folder.set_expanded(true);
                }
            }
        }

        fn save_state(&self) -> Result<(), glib::BoolError> {
            let settings = self.settings();

            let open_sheet_path = self
                .sheet_editor
                .borrow()
                .as_ref()
                .map(|e| e.path())
                .unwrap_or_default();
            settings.set_string("open-sheet-path", open_sheet_path.to_str().unwrap())?;

            settings.set_boolean("library-sidebar-open", self.sidebar_uncollapsed_open.get())?;
            settings.set_boolean("editor-formatbar-open", self.format_bar.is_visible())?;
            settings.set_boolean(
                "editor-sidebar-open",
                self.editor_sidebar_toggle.is_active(),
            )?;

            let expanded_folders = self.library_browser.expanded_folder_paths();
            settings.set_strv("library-expanded-folders", expanded_folders)?;

            Ok(())
        }

        fn create_folder(&self, path: PathBuf) {
            util::create_folder(&path);
            self.library_browser.refresh_content();
        }

        fn create_sheet(&self, path: PathBuf) {
            if let Err(e) = self.close_editor() {
                let toast = Toast::new(&e.to_string());
                self.toast_overlay.add_toast(toast);
                return;
            }
            util::create_sheet_file(&path);
            self.library_browser.refresh_content();
            self.load_sheet(path);
        }

        fn load_sheet(&self, path: PathBuf) {
            if let Err(e) = self.close_editor() {
                let toast = Toast::new(&e.to_string());
                self.toast_overlay.add_toast(toast);
                return;
            }

            let editor = match SheetEditor::new(path.clone()) {
                Ok(editor) => editor,
                Err(e) => {
                    let toast = Toast::new(&e.to_string());
                    self.toast_overlay.add_toast(toast);
                    self.update_window_title();
                    return;
                }
            };

            let font_family = self.settings().string("editor-font-family");
            let font_size = self.settings().uint("editor-font-size");
            editor.set_font(font_family.as_str(), font_size);

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
                move |_| {
                    this.update_toolbar_style();
                }
            ));

            self.editor_sidebar_toggle
                .bind_property("active", &editor, "show_sidebar")
                .sync_create()
                .build();

            self.editor_sidebar_toggle.set_sensitive(true);

            editor.connect_closure(
                "close-requested",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: SheetEditor| {
                        if let Err(e) = this.close_editor() {
                            let toast = Toast::new(&e.to_string());
                            this.toast_overlay.add_toast(toast);
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
                    move |editor: SheetEditor| {
                        this.library_browser.refresh_content();
                        this.library_browser.set_selected_sheet(Some(editor.path()));
                        this.update_window_title();
                    }
                ),
            );

            self.main_toolbar_view.set_content(Some(&editor));
            self.format_bar.bind_editor(Some(editor.clone()));
            self.sheet_editor.replace(Some(editor));
            self.library_browser.set_selected_sheet(Some(path));
            self.obj().action_set_enabled("win.file-save", true);
            self.update_window_title();
        }

        fn trash_folder(&self, folder: LibraryFolder) {
            assert!(!folder.is_root());

            let path = folder
                .path()
                .canonicalize()
                .expect("folder trash failed to canonicalize folder");
            let parent_of_currently_open = self.sheet_editor.borrow().as_ref().is_some_and(|e| {
                e.path()
                    .canonicalize()
                    .expect("folder delet trash to canonicalize sheet")
                    .starts_with(&path)
            });
            if parent_of_currently_open && let Err(e) = self.close_editor() {
                let toast = Toast::new(&e.to_string());
                self.toast_overlay.add_toast(toast);
                return;
            }
            gio::File::for_path(path)
                .trash(None::<&Cancellable>)
                .expect("folder trash failed");
            self.toast_overlay.add_toast(Toast::new("Moved to trash"));
            self.library_browser.refresh_content();
        }

        fn trash_sheet(&self, sheet: LibrarySheet) {
            let path = sheet.path();
            let currently_open = self
                .sheet_editor
                .borrow()
                .as_ref()
                .is_some_and(|e| e.path() == path);
            if currently_open && let Err(e) = self.close_editor() {
                let toast = Toast::new(&e.to_string());
                self.toast_overlay.add_toast(toast);
                return;
            }
            gio::File::for_path(path)
                .trash(None::<&Cancellable>)
                .expect("folder trash failed");
            self.toast_overlay.add_toast(Toast::new("Moved to trash"));
            self.library_browser.refresh_content();
        }

        fn delete_folder(&self, folder: LibraryFolder) {
            assert!(!folder.is_root());

            let path = folder
                .path()
                .canonicalize()
                .expect("folder delet failed to canonicalize folder");
            let parent_of_currently_open = self.sheet_editor.borrow().as_ref().is_some_and(|e| {
                e.path()
                    .canonicalize()
                    .expect("folder delet failed to canonicalize sheet")
                    .starts_with(&path)
            });
            if parent_of_currently_open && let Err(e) = self.close_editor() {
                let toast = Toast::new(&e.to_string());
                self.toast_overlay.add_toast(toast);
                return;
            }
            std::fs::remove_dir_all(path).expect("folder delet failed");
            self.library_browser.refresh_content();
        }

        fn delete_sheet(&self, sheet: LibrarySheet) {
            let path = sheet.path();
            let currently_open = self
                .sheet_editor
                .borrow()
                .as_ref()
                .is_some_and(|e| e.path() == path);
            if currently_open && let Err(e) = self.close_editor() {
                let toast = Toast::new(&e.to_string());
                self.toast_overlay.add_toast(toast);
                return;
            }
            std::fs::remove_file(path).expect("file delet failed");
            self.library_browser.refresh_content();
        }

        fn save_sheet(&self) {
            let mut editor_bind = self.sheet_editor.borrow_mut();
            let Some(editor) = editor_bind.as_mut() else {
                return;
            };
            if let Err(e) = editor.save() {
                let toast = Toast::new(&e.to_string());
                self.toast_overlay.add_toast(toast);
                return;
            }
            self.toast_overlay.add_toast(Toast::new("Saved"));
        }

        fn close_editor(&self) -> Result<(), ScratchmarkError> {
            if let Some(editor) = self.sheet_editor.borrow_mut().as_ref() {
                editor.save()?;
            }
            self.sheet_editor.replace(None);

            self.main_toolbar_view
                .set_content(Some(&EditorPlaceholder::default()));
            self.update_window_title();
            self.library_browser.set_selected_sheet(None);
            self.format_bar.bind_editor(None);
            self.editor_sidebar_toggle.set_sensitive(false);
            self.obj().action_set_enabled("win.file-save", false);
            Ok(())
        }

        fn show_about(&self) {
            let obj = self.obj();
            let dialog = AboutDialog::new();
            dialog.set_application_icon(APP_ID);
            dialog.set_application_name("Scratchmark");
            dialog.set_developer_name("Sevonj");
            dialog.set_issue_url("https://github.com/sevonj/scratchmark/issues/");
            dialog.set_version(env!("CARGO_PKG_VERSION"));
            dialog.set_website("https://github.com/sevonj/scratchmark/");
            dialog.set_support_url("https://github.com/sevonj/scratchmark/discussions/");
            dialog.present(Some(&*obj));
        }

        fn show_font_dialog(&self) {
            let obj = self.obj();

            let font_family = self.settings().string("editor-font-family");
            let font_size = self.settings().uint("editor-font-size");
            let mut initial = FontDescription::new();
            initial.set_family(&font_family);
            initial.set_size(font_size as i32 * pango::SCALE);

            FontDialog::builder().modal(true).build().choose_font(
                Some(obj.as_ref()),
                Some(&initial),
                None::<&Cancellable>,
                clone!(
                    #[weak (rename_to = this)]
                    self,
                    move |result| {
                        let Ok(font) = result else {
                            return;
                        };

                        if let Err(e) = this.set_editor_font(font) {
                            let toast = Toast::new(&e.to_string());
                            this.toast_overlay.add_toast(toast);
                        }
                    }
                ),
            );
        }

        fn set_editor_font(&self, font: FontDescription) -> Result<(), glib::error::BoolError> {
            let family = font.family().unwrap_or_default();
            let size = (font.size() / pango::SCALE) as u32;

            self.settings().set_uint("editor-font-size", size)?;
            self.settings().set_string("editor-font-family", &family)?;

            if let Some(editor) = self.sheet_editor.borrow().as_ref() {
                editor.set_font(family.as_str(), size);
            };

            Ok(())
        }

        /// App quit
        fn on_close_request(&self) -> glib::Propagation {
            self.save_state().expect("Failed to save app state");
            if let Err(e) = self.close_editor() {
                let toast = Toast::new(&e.to_string());
                self.toast_overlay.add_toast(toast);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }

        fn on_fullscreen_changed(
            &self,
            action_fullscreen: SimpleAction,
            action_unfullscreen: SimpleAction,
        ) {
            if self.obj().is_fullscreen() {
                self.unfullscreen_button.set_visible(true);
                self.main_header_revealer.set_reveal_child(false);
                self.main_toolbar_view
                    .set_top_bar_style(adw::ToolbarStyle::Raised);
                self.main_header_bar.set_show_end_title_buttons(false);
                action_fullscreen.set_enabled(false);
                action_unfullscreen.set_enabled(true);
            } else {
                self.unfullscreen_button.set_visible(false);
                self.main_header_revealer.set_reveal_child(true);
                self.main_toolbar_view
                    .set_top_bar_style(adw::ToolbarStyle::Flat);
                self.main_header_bar.set_show_end_title_buttons(true);
                action_fullscreen.set_enabled(true);
                action_unfullscreen.set_enabled(false);
            }
        }

        fn setup_fullscreen_headerbar(&self) {
            let motion_controller = EventControllerMotion::new();
            motion_controller.connect_motion(clone!(
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
            self.obj().add_controller(motion_controller);
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
