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
    use gtk::gdk::Key;
    use gtk::gdk::ModifierType;
    use gtk::gdk::RGBA;
    use gtk::gdk::pango::Underline;
    use gtk::gio::Cancellable;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::glib::clone;
    use gtk::prelude::*;
    use sourceview5::subclass::prelude::*;
    use std::cell::Cell;

    const TYPEWRITER_DIM_TAG: &str = "typewriter-dim";
    const CLICKABLE_HOVER_TAG: &str = "link-hover";

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
                obj.imp().setup_tag_table();
                obj.imp().setup_typewriter_dimming();
            });
            obj.connect_typewriter_mode_notify(move |obj| {
                obj.imp().update_typewriter_dimming();
            });

            self.setup_clickables();
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
        fn link_at_pos(&self, x: f64, y: f64) -> Option<Clickable> {
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
            let clickable = find_clickable(&line_text, byte_index)?;

            let mut link_start = line_start;
            link_start.set_line_index(clickable.start as i32);
            let mut link_end = line_start;
            link_end.set_line_index(clickable.end as i32);

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

            let line_off = buffer.iter_at_line(line).unwrap().offset() as usize;
            let clickable_absolute = Clickable {
                start: clickable.start + line_off,
                end: clickable.end + line_off,
                data: clickable.data,
            };
            Some(clickable_absolute)
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

        fn update_clickable_hover(&self) {
            let view = self.obj();
            let buffer = TextViewExt::buffer(self.obj().upcast_ref::<TextView>());
            let (start, end) = buffer.bounds();
            buffer.remove_tag_by_name(CLICKABLE_HOVER_TAG, &start, &end);

            let Some((x, y)) = self.last_mouse_pos.get() else {
                view.set_cursor_from_name(None);
                return;
            };
            let Some(link) = self.link_at_pos(x, y) else {
                view.set_cursor_from_name(None);
                return;
            };

            let link_start = buffer.iter_at_offset(link.start as i32);
            let link_end = buffer.iter_at_offset(link.end as i32);
            buffer.apply_tag_by_name(CLICKABLE_HOVER_TAG, &link_start, &link_end);

            if self.is_ctrl_held_l.get() || self.is_ctrl_held_r.get() {
                view.set_cursor_from_name(Some("pointer"));
            } else {
                view.set_cursor_from_name(None);
            }
        }

        fn setup_tag_table(&self) {
            let buffer = self.obj().upcast_ref::<TextView>().buffer();
            let tag_table = buffer.tag_table();

            tag_table.add(
                &gtk::TextTag::builder()
                    .name(TYPEWRITER_DIM_TAG)
                    .foreground_rgba(&RGBA::new(0.57, 0.57, 0.57, 1.0))
                    .build(),
            );

            tag_table.add(
                &gtk::TextTag::builder()
                    .name(CLICKABLE_HOVER_TAG)
                    .underline(Underline::Single)
                    .build(),
            );
        }

        fn setup_typewriter_dimming(&self) {
            let buffer = TextViewExt::buffer(self.obj().upcast_ref::<TextView>());
            buffer.connect_cursor_position_notify(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.update_typewriter_dimming();
                }
            ));
            self.update_typewriter_dimming();
        }

        fn setup_clickables(&self) {
            let obj = self.obj();

            let motion = EventControllerMotion::new();
            motion.connect_motion(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_controller, x, y| {
                    imp.last_mouse_pos.set(Some((x, y)));
                    imp.update_clickable_hover();
                }
            ));
            motion.connect_leave(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.last_mouse_pos.set(None);
                    imp.update_clickable_hover();
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
                    imp.update_clickable_hover();
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
                    imp.update_clickable_hover();
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
                    if n_press != 1
                        || !gesture
                            .current_event_state()
                            .contains(ModifierType::CONTROL_MASK)
                    {
                        return;
                    }
                    let Some(clickable) = imp.link_at_pos(x, y) else {
                        return;
                    };
                    match clickable.data {
                        ClickableData::Link { url } => imp.on_link_clicked(&url),
                        ClickableData::TaskBox { checked } => {
                            imp.on_taskbox_clicked(checked, clickable.start as i32 + 1)
                        }
                    }
                    gesture.set_state(gtk::EventSequenceState::Claimed);
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
                    let Some(clickable) = imp.link_at_pos(x as f64, y as f64) else {
                        return false;
                    };
                    let key_string = gettext("Ctrl+Click");
                    match clickable.data {
                        ClickableData::Link { url } => {
                            tooltip.set_text(Some(&format!("{}\n({key_string})", url)));
                            tooltip.set_icon_from_icon_name(Some("smk-external-link-symbolic"));
                        }
                        ClickableData::TaskBox { checked, .. } => {
                            let check_str = if checked {
                                gettext("Uncheck")
                            } else {
                                gettext("Check")
                            };
                            tooltip.set_text(Some(&format!("{check_str} ({key_string})")));
                        }
                    }
                    true
                }
            ));
        }

        fn on_link_clicked(&self, url: &str) {
            let launcher = gtk::UriLauncher::new(&process_url(url));
            let root = self.obj().root();
            let window = root.and_downcast_ref::<gtk::Window>();
            launcher.launch(window, None::<&Cancellable>, move |result| {
                if let Err(e) = result {
                    eprintln!("Couldn't open url. {e}");
                }
            });
        }

        fn on_taskbox_clicked(&self, checked: bool, check_off: i32) {
            let buf = self.obj().buffer();
            let new_char = if checked { " " } else { "x" };

            buf.begin_user_action();
            buf.delete(
                &mut buf.iter_at_offset(check_off),
                &mut buf.iter_at_offset(check_off + 1),
            );
            buf.insert(&mut buf.iter_at_offset(check_off), new_char);
            let mut end_of_line = buf.iter_at_offset(check_off);
            end_of_line.forward_to_line_end();
            buf.place_cursor(&end_of_line);
            buf.end_user_action();
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    struct Clickable {
        pub start: usize,
        pub end: usize,
        pub data: ClickableData,
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    enum ClickableData {
        Link { url: String },
        TaskBox { checked: bool },
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

    fn find_clickable(line: &str, byte_offset: usize) -> Option<Clickable> {
        if let Some(task) = find_task_box(line, byte_offset) {
            return Some(task);
        }
        find_link(line, byte_offset)
    }

    fn find_task_box(line: &str, byte_offset: usize) -> Option<Clickable> {
        let caps = regex::TASK_LIST.captures(line)?;
        let box_match = caps
            .name("task_box")
            .expect("task exists so task_box must too");
        let box_start = box_match.start();
        let box_end = box_match.end();
        if !(box_start..=box_end).contains(&byte_offset) {
            return None;
        }
        let check = caps
            .name("task_check")
            .expect("task exists so task_check must too")
            .as_str();
        Some(Clickable {
            start: box_start,
            end: box_end,
            data: ClickableData::TaskBox {
                checked: check != " ",
            },
        })
    }

    fn find_link(line: &str, byte_offset: usize) -> Option<Clickable> {
        let mut links = Vec::new();

        for caps in regex::INLINE_LINK.captures_iter(line) {
            let whole_match = caps.get(0).unwrap();

            let Some(url) = caps.name("url").map(|m| m.as_str().to_string()) else {
                continue;
            };

            links.push(Clickable {
                start: whole_match.start(),
                end: whole_match.end(),
                data: ClickableData::Link { url },
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
