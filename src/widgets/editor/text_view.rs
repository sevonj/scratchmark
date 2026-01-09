mod imp {
    use adw::subclass::prelude::*;
    use gtk::TextIter;
    use gtk::glib;
    use gtk::prelude::*;
    use sourceview5::subclass::prelude::*;

    use gtk::CssProvider;

    #[derive(Debug, Default)]
    pub struct EditorTextView {
        pub(super) source_view_css_provider: CssProvider,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditorTextView {
        const NAME: &'static str = "EditorTextView";
        type Type = super::EditorTextView;
        type ParentType = sourceview5::View;

        fn class_init(_klass: &mut Self::Class) {}

        fn instance_init(_obj: &glib::subclass::InitializingObject<Self>) {}
    }

    impl ObjectImpl for EditorTextView {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Deprecated, but the only way to do this at the moment?
            // https://gnome.pages.gitlab.gnome.org/gtksourceview/gtksourceview5/class.View.html#changing-the-font
            #[allow(deprecated)]
            obj.style_context().add_provider(
                &self.source_view_css_provider,
                gtk::ffi::GTK_STYLE_PROVIDER_PRIORITY_USER as u32,
            );
        }
    }

    impl WidgetImpl for EditorTextView {}
    impl TextViewImpl for EditorTextView {
        fn copy_clipboard(&self) {
            let buf = self.obj().buffer();
            let cursor = buf.iter_at_offset(buf.cursor_position());
            if !buf.has_selection()
                && let Some((start, end)) = line_bounds(&buf, cursor.line())
            {
                buf.select_range(&start, &end);
                self.parent_copy_clipboard();
                buf.place_cursor(&cursor);
                return;
            }
            self.parent_copy_clipboard();
        }

        fn cut_clipboard(&self) {
            let buf = self.obj().buffer();
            let cursor = buf.iter_at_offset(buf.cursor_position());
            if !buf.has_selection()
                && let Some((start, end)) = line_bounds(&buf, cursor.line())
            {
                buf.select_range(&start, &end);
                self.parent_cut_clipboard();
                return;
            }
            self.parent_cut_clipboard();
        }
    }

    impl ViewImpl for EditorTextView {}

    impl EditorTextView {}

    fn line_bounds(buf: &gtk::TextBuffer, line: i32) -> Option<(TextIter, TextIter)> {
        let start = buf.iter_at_line(line)?;
        let mut end = buf.iter_at_line(line + 1).unwrap_or_else(|| buf.end_iter());
        if end.line() != line {
            end.backward_char();
        }
        Some((start, end))
    }
}

use gtk::glib;
use gtk::subclass::prelude::*;

use gtk::glib::Object;

glib::wrapper! {
    pub struct EditorTextView(ObjectSubclass<imp::EditorTextView>)
        @extends sourceview5::View, gtk::TextView, gtk::Widget,
        @implements gtk::Accessible, gtk::AccessibleText, gtk::Buildable, gtk::ConstraintTarget, gtk::Scrollable;
}

impl Default for EditorTextView {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl EditorTextView {
    pub fn set_font(&self, family: &str, size: u32) {
        let formatted = format!("textview {{font-family: {family}; font-size: {size}pt;}}");
        self.imp()
            .source_view_css_provider
            .load_from_string(&formatted);
    }
}
