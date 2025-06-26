mod imp {
    use std::cell::{OnceCell, RefCell};
    use std::fs;
    use std::path::PathBuf;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{clone, closure_local};
    use gtk::gio;
    use gtk::glib;

    use adw::{
        AboutDialog, AlertDialog, ApplicationWindow, HeaderBar, NavigationPage, OverlaySplitView,
        Toast, ToastOverlay, ToolbarView,
    };
    use gio::{Cancellable, Settings, SimpleActionGroup};
    use gtk::{Builder, Button, CompositeTemplate, EventControllerMotion, MenuButton, Revealer};

    use crate::APP_ID;
    use crate::util;
    use crate::widgets::ItemCreatePopover;
    use crate::widgets::LibraryFolder;
    use crate::widgets::LibrarySheet;
    use crate::widgets::SheetEditorPlaceholder;

    use super::LibraryBrowser;
    use super::SheetEditor;

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
        main_page: TemplateChild<NavigationPage>,
        #[template_child]
        pub(super) main_toolbar_view: TemplateChild<ToolbarView>,
        #[template_child]
        main_header_revealer: TemplateChild<Revealer>,
        #[template_child]
        main_header_bar: TemplateChild<HeaderBar>,

        #[template_child]
        pub(super) toast_overlay: TemplateChild<ToastOverlay>,
        #[template_child]
        new_folder_button: TemplateChild<MenuButton>,
        #[template_child]
        new_sheet_button: TemplateChild<MenuButton>,
        #[template_child]
        unfullscreen_button: TemplateChild<Button>,

        pub(super) library_browser: LibraryBrowser,
        pub(super) sheet_editor: RefCell<Option<SheetEditor>>,

        pub(super) settings: OnceCell<Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
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
            obj.setup_settings();
            #[cfg(debug_assertions)]
            {
                obj.add_css_class("devel");
            }

            let builder = Builder::from_resource("/org/scratchmark/Scratchmark/ui/shortcuts.ui");
            let shortcuts = builder.object("help_overlay").unwrap();
            obj.set_help_overlay(Some(&shortcuts));

            let top_split = self.top_split.get();

            self.library_browser.connect_closure(
                "sheet-selected",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryBrowser, path: PathBuf| {
                        obj.load_sheet(path);
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
                        let body = format!("Are you sure you want to delete {}?", folder.name());
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
                                        obj.imp().force_delete_folder(folder);
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
                        fs::rename(&original_path, &new_path).expect("Folder rename failed");

                        let sheet_editor_opt = this.sheet_editor.borrow();
                        if let Some(sheet_editor) = sheet_editor_opt.as_ref() {
                            let selected = sheet_editor.path();
                            let old_path = folder.path();
                            if selected.starts_with(&old_path) {
                                let relative = selected.strip_prefix(&old_path).unwrap();
                                let sheet_path = new_path.join(relative);
                                this.library_browser
                                    .set_selected_sheet(Some(sheet_path.clone()));
                                sheet_editor.set_path(sheet_path);
                            }
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
                        fs::rename(&original_path, &new_path).expect("File rename failed");

                        let sheet_editor_opt = this.sheet_editor.borrow();
                        if let Some(sheet_editor) = sheet_editor_opt.as_ref() {
                            if sheet_editor.path() == sheet.path() {
                                this.library_browser
                                    .set_selected_sheet(Some(new_path.clone()));
                                sheet_editor.set_path(new_path);
                            }
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
                        let body = format!("Are you sure you want to delete {}?", sheet.stem());
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
                                        obj.imp().force_delete_sheet(sheet);
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
                    #[weak]
                    obj,
                    move |_popover: ItemCreatePopover, path: PathBuf| {
                        obj.create_folder(path);
                    }
                ),
            );

            let new_sheet_popover = ItemCreatePopover::for_sheet();
            self.new_sheet_button.set_popover(Some(&new_sheet_popover));
            new_sheet_popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_popover: ItemCreatePopover, path: PathBuf| {
                        obj.create_sheet(path);
                    }
                ),
            );

            self.main_toolbar_view
                .set_content(Some(&SheetEditorPlaceholder::default()));
            self.sidebar_toolbar_view
                .set_content(Some(&self.library_browser));
            self.update_window_title();

            obj.connect_close_request(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_: &super::Window| {
                    obj.save_state().expect("Failed to save app state");
                    if let Err(e) = obj.close_editor() {
                        let toast = Toast::new(&e.to_string());
                        obj.imp().toast_overlay.add_toast(toast);
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                }
            ));

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
            obj.add_controller(motion_controller);

            let action_fullscreen = gio::SimpleAction::new("fullscreen", None);
            let action_unfullscreen = gio::SimpleAction::new("unfullscreen", None);
            obj.connect_fullscreened_notify(clone!(
                #[weak]
                action_fullscreen,
                #[weak]
                action_unfullscreen,
                move |window| {
                    let imp = window.imp();
                    if window.is_fullscreen() {
                        imp.unfullscreen_button.set_visible(true);
                        imp.main_header_revealer.set_reveal_child(false);
                        imp.main_toolbar_view
                            .set_top_bar_style(adw::ToolbarStyle::Raised);
                        imp.main_header_bar.set_show_end_title_buttons(false);
                        action_fullscreen.set_enabled(false);
                        action_unfullscreen.set_enabled(true);
                    } else {
                        imp.unfullscreen_button.set_visible(false);
                        imp.main_header_revealer.set_reveal_child(true);
                        imp.main_toolbar_view
                            .set_top_bar_style(adw::ToolbarStyle::Flat);
                        imp.main_header_bar.set_show_end_title_buttons(true);
                        action_fullscreen.set_enabled(true);
                        action_unfullscreen.set_enabled(false);
                    }
                }
            ));
            action_fullscreen.connect_activate(clone!(
                #[weak]
                obj,
                move |_, _| {
                    obj.fullscreen();
                }
            ));
            action_unfullscreen.connect_activate(clone!(
                #[weak]
                obj,
                move |_, _| {
                    obj.unfullscreen();
                }
            ));
            obj.add_action(&action_fullscreen);
            obj.add_action(&action_unfullscreen);

            let action = gio::SimpleAction::new("file-new", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    this.new_sheet_button.popup();
                }
            ));
            obj.add_action(&action);
            let action = gio::SimpleAction::new("file-close", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_, _| {
                    if let Err(e) = obj.close_editor() {
                        let toast = Toast::new(&e.to_string());
                        obj.imp().toast_overlay.add_toast(toast);
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
                    let collapsed = !top_split.is_collapsed();
                    top_split.set_collapsed(collapsed);
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

            let editor_actions = SimpleActionGroup::new();
            obj.insert_action_group("editor", Some(&editor_actions));
            let action = gio::SimpleAction::new("search-toggle", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    let sheet_editor_opt = this.sheet_editor.borrow();
                    if let Some(sheet_editor) = sheet_editor_opt.as_ref() {
                        sheet_editor
                            .activate_action("editor.search-toggle", Some(&false.to_variant()))
                            .unwrap();
                    }
                }
            ));
            editor_actions.add_action(&action);

            obj.load_state();
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}

    impl Window {
        pub(super) fn update_window_title(&self) {
            if let Some(editor) = self.sheet_editor.borrow().as_ref() {
                if let Some(stem) = editor.path().file_stem() {
                    self.main_page.set_title(&stem.to_string_lossy());
                    return;
                };
            };
            self.main_page.set_title("Scratchmark");
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
            if parent_of_currently_open {
                if let Err(e) = self.obj().close_editor() {
                    let toast = Toast::new(&e.to_string());
                    self.toast_overlay.add_toast(toast);
                    return;
                }
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
            if currently_open {
                if let Err(e) = self.obj().close_editor() {
                    let toast = Toast::new(&e.to_string());
                    self.toast_overlay.add_toast(toast);
                    return;
                }
            }
            gio::File::for_path(path)
                .trash(None::<&Cancellable>)
                .expect("folder trash failed");
            self.toast_overlay.add_toast(Toast::new("Moved to trash"));
            self.library_browser.refresh_content();
        }

        fn force_delete_folder(&self, folder: LibraryFolder) {
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
            if parent_of_currently_open {
                if let Err(e) = self.obj().close_editor() {
                    let toast = Toast::new(&e.to_string());
                    self.toast_overlay.add_toast(toast);
                    return;
                }
            }
            std::fs::remove_dir_all(path).expect("folder delet failed");
            self.library_browser.refresh_content();
        }

        fn force_delete_sheet(&self, sheet: LibrarySheet) {
            let path = sheet.path();
            let currently_open = self
                .sheet_editor
                .borrow()
                .as_ref()
                .is_some_and(|e| e.path() == path);
            if currently_open {
                if let Err(e) = self.obj().close_editor() {
                    let toast = Toast::new(&e.to_string());
                    self.toast_overlay.add_toast(toast);
                    return;
                }
            }
            std::fs::remove_file(path).expect("file delet failed");
            self.library_browser.refresh_content();
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
    }
}

use std::path::PathBuf;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::closure_local;
use gtk::gio;
use gtk::glib;

use adw::Toast;
use gio::Settings;
use glib::Object;

use crate::APP_ID;
use crate::error::ScratchmarkError;
use crate::util;

use super::LibraryBrowser;
use super::SheetEditor;
use super::SheetEditorPlaceholder;

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

    fn settings(&self) -> &Settings {
        self.imp().settings.get().expect("Settings uninitialized.")
    }

    fn setup_settings(&self) {
        let settings = Settings::new(APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("`settings` should not be set before calling `setup_settings`.");
    }

    fn load_state(&self) {
        let open_sheet_path = self.settings().string("open-sheet-path");
        if !open_sheet_path.is_empty() {
            let open_sheet_path = PathBuf::from(open_sheet_path);
            if !open_sheet_path.exists() {
                let toast = Toast::new("Last open sheet has been moved or deleted.");
                self.imp().toast_overlay.add_toast(toast);
            }
            self.load_sheet(open_sheet_path);
        }

        let library_expanded_folders = self.settings().strv("library-expanded-folders");
        for path in library_expanded_folders {
            if let Some(folder) = self.imp().library_browser.get_folder(&PathBuf::from(path)) {
                folder.set_expanded(true);
            }
        }
    }

    fn save_state(&self) -> Result<(), glib::BoolError> {
        let open_sheet_path = self
            .imp()
            .sheet_editor
            .borrow()
            .as_ref()
            .map(|e| e.path())
            .unwrap_or_default();
        self.settings()
            .set_string("open-sheet-path", open_sheet_path.to_str().unwrap())?;

        let expanded_folders = self.imp().library_browser.expanded_folder_paths();
        self.settings()
            .set_strv("library-expanded-folders", expanded_folders)?;

        Ok(())
    }

    fn load_sheet(&self, path: PathBuf) {
        let imp = self.imp();
        if let Err(e) = self.close_editor() {
            let toast = Toast::new(&e.to_string());
            imp.toast_overlay.add_toast(toast);
            return;
        }

        let editor = match SheetEditor::new(path.clone()) {
            Ok(editor) => editor,
            Err(e) => {
                let toast = Toast::new(&e.to_string());
                imp.toast_overlay.add_toast(toast);
                imp.update_window_title();
                return;
            }
        };

        editor.connect_closure(
            "close-requested",
            false,
            closure_local!(
                #[weak(rename_to = obj)]
                self,
                move |_: SheetEditor| {
                    if let Err(e) = obj.close_editor() {
                        let toast = Toast::new(&e.to_string());
                        obj.imp().toast_overlay.add_toast(toast);
                        return;
                    }
                }
            ),
        );

        editor.connect_closure(
            "saved-as",
            false,
            closure_local!(
                #[weak]
                imp,
                move |editor: SheetEditor| {
                    imp.library_browser.refresh_content();
                    imp.library_browser.set_selected_sheet(Some(editor.path()));
                    imp.update_window_title();
                }
            ),
        );

        imp.main_toolbar_view.set_content(Some(&editor));
        imp.sheet_editor.replace(Some(editor));
        imp.library_browser.set_selected_sheet(Some(path));
        imp.update_window_title();
    }

    fn create_folder(&self, path: PathBuf) {
        util::create_folder(&path);
        self.imp().library_browser.refresh_content();
    }

    fn create_sheet(&self, path: PathBuf) {
        if let Err(e) = self.close_editor() {
            let toast = Toast::new(&e.to_string());
            self.imp().toast_overlay.add_toast(toast);
            return;
        }
        util::create_sheet_file(&path);
        self.imp().library_browser.refresh_content();
        self.load_sheet(path);
    }

    fn close_editor(&self) -> Result<(), ScratchmarkError> {
        let imp = self.imp();
        if let Some(editor) = imp.sheet_editor.borrow_mut().as_ref() {
            editor.save()?;
        }
        imp.sheet_editor.replace(None);

        imp.main_toolbar_view
            .set_content(Some(&SheetEditorPlaceholder::default()));
        self.imp().update_window_title();
        self.imp().library_browser.set_selected_sheet(None);
        Ok(())
    }
}
