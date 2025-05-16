mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::closure_local;
    use gtk::glib;

    use adw::{ApplicationWindow, HeaderBar, NavigationPage, OverlaySplitView, ToolbarView};
    use gtk::{Button, CompositeTemplate};

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
        pub(super) new_sheet_button: TemplateChild<Button>,

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

            let top_split = self.top_split.get();
            self.sidebar_toggle.connect_clicked(clone!(move |_| {
                let collapsed = !top_split.is_collapsed();
                top_split.set_collapsed(collapsed);
            }));

            let obj = self.obj();
            self.library_browser.connect_closure(
                "sheet-selected",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_browser: LibraryBrowser, path: PathBuf| {
                        obj.load_sheet(path);
                    }
                ),
            );

            self.main_toolbar_view
                .set_content(Some(&SheetEditorPlaceholder::default()));
            self.main_page.set_title("TheftMD");
            self.sidebar_toolbar_view
                .set_content(Some(&self.library_browser));
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

use std::path::PathBuf;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::gio;
use gtk::glib;
use gtk::glib::closure_local;

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

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("TheftMD");
        imp.main_page.get().set_title(stem);

        let editor = SheetEditor::new(path);

        let this = self;
        editor.connect_closure(
            "close-requested",
            false,
            closure_local!(
                #[weak]
                this,
                move |_: SheetEditor| { this.close_sheet() }
            ),
        );

        imp.main_toolbar_view.set_content(Some(&editor));
        imp.sheet_editor.replace(Some(editor));
    }

    fn close_sheet(&self) {
        let imp = self.imp();
        imp.sheet_editor.replace(None);

        imp.main_toolbar_view
            .set_content(Some(&SheetEditorPlaceholder::default()));
        imp.main_page.get().set_title("TheftMD");

        imp.library_browser.clear_selected_sheet();
    }
}
