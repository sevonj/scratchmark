mod imp {
    use adw::subclass::prelude::*;
    use gtk::CssProvider;
    use gtk::TextIter;
    use gtk::TextView;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::glib::clone;
    use gtk::prelude::*;
    use sourceview5::subclass::prelude::*;
    use std::cell::Cell;

    const TYPEWRITER_DIM_TAG: &str = "typewriter-dim";

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::EditorTextView)]
    pub struct EditorTextView {
        #[property(get, set)]
        typewriter_mode: Cell<bool>,

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

    #[glib::derived_properties]
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

            obj.connect_buffer_notify(move |obj| {
                obj.imp().setup_typewriter_dimming();
            });
            obj.connect_typewriter_mode_notify(move |obj| {
                obj.imp().update_typewriter_dimming();
            });
            obj.imp().setup_typewriter_dimming();
        }
    }

    impl WidgetImpl for EditorTextView {}
    impl TextViewImpl for EditorTextView {
        fn copy_clipboard(&self) {
            let buf = self.obj().buffer();
            let cursor = buf.iter_at_offset(buf.cursor_position());
            if !buf.has_selection()
                && let Some((mut start, end)) = line_bounds(&buf, cursor.line())
            {
                let mut before = start;
                before.backward_cursor_position();
                if buf.text(&before, &start, true) == "\n" {
                    start = before;
                }

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
                && let Some((mut start, end)) = line_bounds(&buf, cursor.line())
            {
                let mut before = start;
                before.backward_cursor_position();
                if buf.text(&before, &start, true) == "\n" {
                    start = before;
                }

                buf.select_range(&start, &end);
                self.parent_cut_clipboard();
                return;
            }
            self.parent_cut_clipboard();
        }

        fn paste_clipboard(&self) {
            glib::idle_add_local_once(clone!(
                #[weak(rename_to = imp)]
                self,
                move || imp.update_typewriter_dimming()
            ));
            self.parent_paste_clipboard();
        }
    }

    impl ViewImpl for EditorTextView {}

    impl EditorTextView {
        fn update_typewriter_dimming(&self) {
            let buffer = TextViewExt::buffer(self.obj().upcast_ref::<TextView>());

            if !self.obj().typewriter_mode() {
                let (start, end) = buffer.bounds();
                buffer.remove_tag_by_name(TYPEWRITER_DIM_TAG, &start, &end);
                return;
            }

            let insert = buffer.get_insert();
            let iter = buffer.iter_at_mark(&insert);
            let line = iter.line();

            let (start, end) = buffer.bounds();
            buffer.apply_tag_by_name(TYPEWRITER_DIM_TAG, &start, &end);

            if let Some(line_start) = buffer.iter_at_line(line) {
                let mut line_end = line_start;
                line_end.forward_to_line_end();
                buffer.remove_tag_by_name(TYPEWRITER_DIM_TAG, &line_start, &line_end);
            }
        }

        fn setup_typewriter_dimming(&self) {
            let buffer = TextViewExt::buffer(self.obj().upcast_ref::<TextView>());
            let tag_table = buffer.tag_table();

            let dim_color = gtk::gdk::RGBA::new(0.57, 0.57, 0.57, 1.0);

            let dim_tag = gtk::TextTag::builder()
                .name(TYPEWRITER_DIM_TAG)
                .foreground_rgba(&dim_color)
                .build();
            tag_table.add(&dim_tag);

            let (start, end) = buffer.bounds();
            buffer.apply_tag(&dim_tag, &start, &end);

            buffer.connect_cursor_position_notify(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.update_typewriter_dimming();
                }
            ));
            self.update_typewriter_dimming();
        }
    }

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
use gtk::glib::Object;
use gtk::subclass::prelude::*;

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
