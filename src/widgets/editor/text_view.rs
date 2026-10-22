mod imp {
    use crate::util::regex;
    use adw::subclass::prelude::*;
    use gettextrs::gettext;
    use gtk::CssProvider;
    use gtk::EventControllerKey;
    use gtk::EventControllerMotion;
    use gtk::GestureClick;
    use gtk::TextIter;
    use gtk::TextView;
    use gtk::gdk::{Key, ModifierType};
    use gtk::gio::Cancellable;
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
        last_mouse_pos: Cell<Option<(f64, f64)>>,
        is_ctrl_held_l: Cell<bool>,
        is_ctrl_held_r: Cell<bool>,
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

            self.setup_clickable_links();
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
        fn link_at_pos(&self, x: f64, y: f64) -> Option<MarkdownLink> {
            let obj = self.obj();

            let (x, y) =
                obj.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);

            let iter = obj.iter_at_location(x, y)?;
            let buffer = obj.buffer();
            let line = iter.line();

            let line_start = buffer.iter_at_line(line)?;
            let mut line_end = line_start;
            line_end.forward_to_line_end();

            let line_text = buffer.text(&line_start, &line_end, false);
            let byte_index = iter.line_index() as usize;
            let link = find_link(&line_text, byte_index)?;

            let mut link_start = line_start;
            link_start.set_line_index(link.start as i32);
            let mut link_end = line_start;
            link_end.set_line_index(link.end as i32);

            let start_rect = obj.iter_location(&link_start);
            let end_rect = obj.iter_location(&link_end);

            if y < start_rect.y() || y > end_rect.y() + end_rect.height() {
                return None;
            }
            if start_rect.y() == end_rect.y()
                && (x < start_rect.x() || x > end_rect.x() + end_rect.width())
            {
                return None;
            }

            Some(link)
        }

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

        fn update_pointer(&self) {
            let view = self.obj();
            let in_control = self.is_ctrl_held_l.get() || self.is_ctrl_held_r.get();
            if in_control
                && let Some((x, y)) = self.last_mouse_pos.get()
                && self.link_at_pos(x, y).is_some()
            {
                view.set_cursor_from_name(Some("pointer"));
            } else {
                view.set_cursor_from_name(None);
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

        fn setup_clickable_links(&self) {
            let obj = self.obj();

            let motion = EventControllerMotion::new();
            motion.connect_motion(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_controller, x, y| {
                    imp.last_mouse_pos.set(Some((x, y)));
                    imp.update_pointer();
                }
            ));
            motion.connect_leave(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.last_mouse_pos.set(None);
                    imp.update_pointer();
                }
            ));
            obj.add_controller(motion);

            let key_controller = EventControllerKey::new();
            /* FIXME: Find out why this seems to return the previous event and then get rid of
                separate key states
            key_controller.connect_modifiers(clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_controller, state| {
                    print!("modified");
                    print!(" before={} ", imp.is_ctrl_held.get());
                    imp.is_ctrl_held
                        .set(state.contains(ModifierType::CONTROL_MASK ));
                    print!(" after={} ", imp.is_ctrl_held.get());
                    imp.update_pointer();
                    println!("");
                    glib::Propagation::Proceed
                }
            ));
            */
            key_controller.connect_key_pressed(clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_, keyval, _, _| {
                    if keyval == Key::Control_L {
                        imp.is_ctrl_held_l.set(true);
                    } else if keyval == Key::Control_R {
                        imp.is_ctrl_held_r.set(true);
                    }
                    imp.update_pointer();
                    glib::Propagation::Proceed
                }
            ));
            key_controller.connect_key_released(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, keyval, _, _| {
                    if keyval == Key::Control_L {
                        imp.is_ctrl_held_l.set(false);
                    } else if keyval == Key::Control_R {
                        imp.is_ctrl_held_r.set(false);
                    }
                    imp.update_pointer();
                }
            ));
            obj.add_controller(key_controller);

            let click_gesture = GestureClick::new();
            click_gesture.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
            click_gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
            click_gesture.connect_pressed(clone!(
                #[weak(rename_to = imp)]
                self,
                move |gesture, n_press, x, y| {
                    if n_press == 1
                        && gesture
                            .current_event_state()
                            .contains(ModifierType::CONTROL_MASK)
                        && let Some(link) = imp.link_at_pos(x, y)
                    {
                        imp.on_link_clicked(&link.url);
                        gesture.set_state(gtk::EventSequenceState::Claimed);
                    }
                }
            ));
            obj.add_controller(click_gesture);

            obj.set_has_tooltip(true);
            obj.connect_query_tooltip(clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                false,
                move |_view, x, y, _keyboard_mode, tooltip| {
                    if let Some(link) = imp.link_at_pos(x as f64, y as f64) {
                        tooltip.set_text(Some(&format!(
                            "{} ➜ {}",
                            gettext("Ctrl+Click"),
                            link.url
                        )));
                        true
                    } else {
                        false
                    }
                }
            ));
        }

        fn on_link_clicked(&self, url: &str) {
            let launcher = gtk::UriLauncher::new(&process_url(&url));
            let root = self.obj().root();
            let window = root.and_downcast_ref::<gtk::Window>();
            launcher.launch(window, None::<&Cancellable>, move |result| {
                if let Err(e) = result {
                    eprintln!("Couldn't open url. {e}");
                }
            });
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    struct MarkdownLink {
        pub start: usize,
        pub end: usize,
        pub url: String,
    }

    fn line_bounds(buf: &gtk::TextBuffer, line: i32) -> Option<(TextIter, TextIter)> {
        let start = buf.iter_at_line(line)?;
        let mut end = buf.iter_at_line(line + 1).unwrap_or_else(|| buf.end_iter());
        if end.line() != line {
            end.backward_char();
        }
        Some((start, end))
    }

    fn process_url(url: &str) -> String {
        let trimmed = url.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return trimmed.to_string();
        }
        format!("http://{trimmed}")
    }

    fn find_link(line: &str, byte_offset: usize) -> Option<MarkdownLink> {
        let mut links = Vec::new();

        for caps in regex::INLINE_LINK.captures_iter(line) {
            let whole_match = caps.get(0).unwrap();

            let url = if let Some(url_bare) = caps.name("url") {
                url_bare.as_str().to_string()
            } else {
                continue;
            };

            links.push(MarkdownLink {
                start: whole_match.start(),
                end: whole_match.end(),
                url,
            });
        }

        links
            .into_iter()
            .find(|link| byte_offset >= link.start && byte_offset < link.end)
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
