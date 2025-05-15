mod imp {
    use std::path::PathBuf;

    use adw::prelude::NavigationPageExt;
    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::closure_local;
    use gtk::glib;
    use gtk::prelude::*;

    use adw::{ApplicationWindow, HeaderBar, NavigationPage, OverlaySplitView, ToolbarView};
    use gtk::{Button, CompositeTemplate};

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
        pub(super) sheet_editor: SheetEditor,
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

            let sheet_editor = &self.sheet_editor;
            self.new_sheet_button.connect_clicked(clone!(
                #[weak]
                sheet_editor,
                move |_| {
                    sheet_editor.new_sheet();
                }
            ));

            let this = self;
            self.library_browser.connect_closure(
                "sheet-selected",
                false,
                closure_local!(
                    #[weak]
                    this,
                    move |_browser: LibraryBrowser, path: PathBuf| {
                        let stem = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("TheftMD");
                        this.main_page.set_title(stem);
                        this.sheet_editor.load_sheet(path);
                    }
                ),
            );

            self.sidebar_toolbar_view
                .set_content(Some(&self.library_browser));
            self.main_toolbar_view.set_content(Some(&self.sheet_editor));
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

use glib::Object;
use gtk::{gio, glib};

use super::LibraryBrowser;
use super::SheetEditor;

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
