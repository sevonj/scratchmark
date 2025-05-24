mod imp {
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::closure_local;
    use gtk::glib;

    use adw::{
        ApplicationWindow, HeaderBar, NavigationPage, OverlaySplitView, Toast, ToastOverlay,
        ToolbarView,
    };
    use gtk::{Button, CompositeTemplate, MenuButton};

    use crate::util;
    use crate::widgets::LibraryFolder;
    use crate::widgets::LibrarySheet;
    use crate::widgets::NewFolderPopover;
    use crate::widgets::NewSheetPopover;
    use crate::widgets::SheetEditorPlaceholder;

    use super::LibraryBrowser;
    use super::SheetEditor;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/window.ui")]
    pub struct Window {
        #[template_child]
        pub(super) top_split: TemplateChild<OverlaySplitView>,

        #[template_child]
        pub(super) sidebar_page: TemplateChild<NavigationPage>,
        #[template_child]
        pub(super) sidebar_header_bar: TemplateChild<HeaderBar>,
        #[template_child]
        pub(super) sidebar_toggle: TemplateChild<Button>,
        #[template_child]
        pub(super) sidebar_toolbar_view: TemplateChild<ToolbarView>,

        #[template_child]
        pub(super) main_page: TemplateChild<NavigationPage>,
        #[template_child]
        pub(super) main_toolbar_view: TemplateChild<ToolbarView>,

        #[template_child]
        pub(super) toast_overlay: TemplateChild<ToastOverlay>,
        #[template_child]
        pub(super) new_folder_button: TemplateChild<MenuButton>,
        #[template_child]
        pub(super) new_sheet_button: TemplateChild<MenuButton>,

        pub(super) library_browser: LibraryBrowser,
        pub(super) sheet_editor: RefCell<Option<SheetEditor>>,
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

            #[cfg(debug_assertions)]
            {
                obj.add_css_class("devel");
            }

            let top_split = self.top_split.get();
            self.sidebar_toggle.connect_clicked(clone!(move |_| {
                let collapsed = !top_split.is_collapsed();
                top_split.set_collapsed(collapsed);
            }));

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
                "folder-delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |browser: LibraryBrowser, folder: LibraryFolder| {
                        assert!(!folder.is_root());

                        let path = folder
                            .path()
                            .canonicalize()
                            .expect("folder delet failed to canonicalize folder");
                        let parent_of_currently_open =
                            obj.imp().sheet_editor.borrow().as_ref().is_some_and(|e| {
                                e.path()
                                    .canonicalize()
                                    .expect("folder delet failed to canonicalize sheet")
                                    .starts_with(&path)
                            });
                        if parent_of_currently_open {
                            if let Err(e) = obj.close_sheet() {
                                let toast = Toast::new(&e.to_string());
                                obj.imp().toast_overlay.add_toast(toast);
                                return;
                            }
                        }
                        std::fs::remove_dir_all(path).expect("folder delet failed");
                        browser.refresh_content();
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
                    move |browser: LibraryBrowser, sheet: LibrarySheet| {
                        let path = sheet.path();
                        let currently_open = obj
                            .imp()
                            .sheet_editor
                            .borrow()
                            .as_ref()
                            .is_some_and(|e| e.path() == path);
                        if currently_open {
                            if let Err(e) = obj.close_sheet() {
                                let toast = Toast::new(&e.to_string());
                                obj.imp().toast_overlay.add_toast(toast);
                                return;
                            }
                        }
                        std::fs::remove_file(path).expect("file delet failed");
                        browser.refresh_content();
                    }
                ),
            );

            let new_folder_popover = NewFolderPopover::default();
            self.new_folder_button
                .set_popover(Some(&new_folder_popover));
            new_folder_popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_popover: NewFolderPopover, path: PathBuf| {
                        obj.create_folder(path);
                    }
                ),
            );

            let new_sheet_popover = NewSheetPopover::default();
            self.new_sheet_button.set_popover(Some(&new_sheet_popover));
            new_sheet_popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_popover: NewSheetPopover, path: PathBuf| {
                        obj.create_sheet(path);
                    }
                ),
            );

            self.main_toolbar_view
                .set_content(Some(&SheetEditorPlaceholder::default()));
            self.sidebar_toolbar_view
                .set_content(Some(&self.library_browser));
            self.update_window_title();
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
            self.main_page.set_title("TheftMD");
        }
    }
}

use std::path::PathBuf;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::gio;
use gtk::glib;
use gtk::glib::closure_local;

use adw::Toast;
use glib::Object;

use crate::error::TheftMDError;
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

    fn load_sheet(&self, path: PathBuf) {
        let imp = self.imp();
        if let Err(e) = self.close_sheet() {
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
                    if let Err(e) = obj.close_sheet() {
                        let toast = Toast::new(&e.to_string());
                        obj.imp().toast_overlay.add_toast(toast);
                        return;
                    }
                    obj.imp().library_browser.set_selected_sheet(None);
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
        if let Err(e) = self.close_sheet() {
            let toast = Toast::new(&e.to_string());
            self.imp().toast_overlay.add_toast(toast);
            return;
        }
        util::create_sheet_file(&path);
        self.imp().library_browser.refresh_content();
        self.load_sheet(path);
    }

    fn close_sheet(&self) -> Result<(), TheftMDError> {
        let imp = self.imp();
        if let Some(editor) = imp.sheet_editor.borrow_mut().as_ref() {
            editor.save()?;
        }
        imp.sheet_editor.replace(None);

        imp.main_toolbar_view
            .set_content(Some(&SheetEditorPlaceholder::default()));
        self.imp().update_window_title();
        Ok(())
    }
}
