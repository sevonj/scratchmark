mod formatting;

mod imp {
    use super::MarkdownHeading;
    use adw::glib::subclass::Signal;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::TextIter;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::glib::clone;
    use sourceview5::prelude::*;
    use sourceview5::subclass::prelude::*;
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::sync::OnceLock;
    use std::time::Duration;

    #[derive(Debug, Properties, Default)]
    #[properties(wrapper_type = super::MarkdownBuffer)]
    pub struct MarkdownBuffer {
        #[property(get, set)]
        pub(super) paste_in_progress: Cell<bool>,
        pub(super) formatting_action_in_progress: Cell<bool>,
        pub(super) table_of_contents: RefCell<Vec<MarkdownHeading>>,
        pub(super) toc_refresh_source_id: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MarkdownBuffer {
        const NAME: &'static str = "MarkdownBuffer";
        type Type = super::MarkdownBuffer;
        type ParentType = sourceview5::Buffer;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MarkdownBuffer {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("toc-changed").build()])
        }

        fn constructed(&self) {
            let obj = self.obj();
            obj.set_highlight_matching_brackets(false);

            obj.connect_changed(move |obj| obj.imp().schedule_refresh_toc());

            self.parent_constructed();
        }

        fn dispose(&self) {
            if let Some(source_id) = self.toc_refresh_source_id.take() {
                source_id.remove();
            }
        }
    }

    impl TextBufferImpl for MarkdownBuffer {
        // Inserted text passes through here
        // We can modify or cancel it here, and do other things, like move the cursor
        fn insert_text(&self, iter: &mut TextIter, new_text: &str) {
            let obj = self.obj();

            if self.paste_in_progress.get() {
                /* FIXME: Paste delivery may be split into multiple calls, at least when there's syntax highlighting
                text: ' '
                nah
                text: '-'
                nah
                text: ' '
                nah
                text: '['
                nah
                text: 'x'
                nah
                text: ']'
                nah
                text: ' Click detect'
                nah
                text: '
                '
                yes
                */
                println!("text: '{new_text}'");

                if !obj.has_selection()
                    && new_text.ends_with('\n')
                    && let Some(line_start) = obj.iter_at_line(iter.line())
                {
                    obj.place_cursor(&line_start);
                    *iter = line_start;
                    println!("yes");
                } else {
                    println!("nah");
                }

                self.parent_insert_text(iter, new_text);
                return;
            }

            if !self.formatting_action_in_progress.get()
                && let (Some(auto_formatted_text), cursor_move) =
                    self.process_auto_close_formatting(iter, new_text.to_owned())
            {
                self.parent_insert_text(iter, &auto_formatted_text);
                let mut cursor = obj.iter_at_offset(obj.cursor_position());
                cursor.forward_cursor_positions(cursor_move);
                obj.place_cursor(&cursor);
                return;
            }

            self.parent_insert_text(iter, new_text);
        }

        fn paste_done(&self, clipboard: &gtk::gdk::Clipboard) {
            self.paste_in_progress.replace(false);
            self.parent_paste_done(clipboard)
        }
    }
    impl BufferImpl for MarkdownBuffer {}

    impl MarkdownBuffer {
        /// If you type an opening formatting char, for example an asterisk,
        /// this will place a closing one after the cursor
        pub(super) fn process_auto_close_formatting(
            &self,
            iter: &mut TextIter,
            process_text: String,
        ) -> (Option<String>, i32) {
            let obj = self.obj();

            let lookahead = obj
                .text(iter, &obj.iter_at_offset(iter.offset() + 1), false)
                .chars()
                .next();

            fn has_whitespace(text: &str) -> bool {
                for c in text.chars() {
                    if c.is_whitespace() {
                        return true;
                    }
                }
                false
            }

            match process_text.as_str() {
                "*" => match lookahead {
                    Some('*') => {
                        let lookback = self.lookback2(iter);
                        if lookback == "**" || !has_whitespace(&lookback) {
                            return (None, 1);
                        }
                        return (Some(process_text + "*"), -1);
                    }
                    Some(c) if !c.is_whitespace() => {
                        return (Some(process_text), 0);
                    }
                    _ => {
                        return (Some(process_text + "*"), -1);
                    }
                },
                "~" => match lookahead {
                    Some('~') => {
                        let lookback = self.lookback2(iter);
                        if lookback == "~~" || !has_whitespace(&lookback) {
                            return (None, 1);
                        }
                        return (Some(process_text + "~"), -1);
                    }
                    Some(c) if !c.is_whitespace() => {
                        return (Some(process_text), 0);
                    }
                    _ => {
                        if self.lookback(iter) == Some('~') {
                            return (Some(process_text + "~~"), -2);
                        }
                        return (Some(process_text), 0);
                    }
                },
                "=" => match lookahead {
                    Some('=') => {
                        let lookback = self.lookback2(iter);
                        if lookback == "==" || !has_whitespace(&lookback) {
                            return (None, 1);
                        }
                        return (Some(process_text + "="), -1);
                    }
                    Some(c) if !c.is_whitespace() => {
                        return (Some(process_text), 0);
                    }
                    _ => {
                        if self.lookback(iter) == Some('=') {
                            return (Some(process_text + "=="), -2);
                        }
                        return (Some(process_text), 0);
                    }
                },
                "`" => match lookahead {
                    Some('`') => {
                        return (None, 1);
                    }
                    Some(c) if !c.is_whitespace() => {
                        return (Some(process_text), 0);
                    }
                    _ => {
                        if self.lookback(iter) == Some('`') {
                            return (Some(process_text), 0);
                        }
                        return (Some(process_text + "`"), -1);
                    }
                },
                _ => (),
            }

            (Some(process_text), 0)
        }

        fn lookback(&self, iter: &TextIter) -> Option<char> {
            let obj = self.obj();
            obj.text(&obj.iter_at_offset(iter.offset() - 1), iter, false)
                .chars()
                .next()
        }

        fn lookback2(&self, iter: &TextIter) -> glib::GString {
            let obj = self.obj();
            obj.text(&obj.iter_at_offset(iter.offset() - 2), iter, false)
        }

        fn schedule_refresh_toc(&self) {
            let obj = self.obj();
            if let Some(source_id) = self.toc_refresh_source_id.take() {
                source_id.remove();
            }

            let source_id = glib::timeout_add_local_once(
                Duration::from_millis(250),
                clone!(
                    #[weak]
                    obj,
                    move || {
                        let imp = obj.imp();
                        imp.toc_refresh_source_id.take();
                        imp.refresh_toc();
                    }
                ),
            );
            self.toc_refresh_source_id.replace(Some(source_id));
        }

        fn refresh_toc(&self) {
            if let Some(source_id) = self.toc_refresh_source_id.take() {
                source_id.remove();
            }

            let obj = self.obj();
            let mut new_toc = Vec::new();

            let mut line_start = obj.start_iter();
            let mut fenced_backtick = false;
            let mut fenced_tilde = false;
            loop {
                let first_char = line_start.char();
                if !&['#', ' ', '`', '~'].contains(&first_char) {
                    if !line_start.forward_line() {
                        break;
                    }
                    continue;
                }

                let mut line_end = line_start;
                line_end.forward_to_line_end();
                let line_text = obj.text(&line_start, &line_end, false);

                let indent = line_text.chars().take_while(|&c| c == ' ').count();
                if indent > 3 {
                    if !line_start.forward_line() {
                        break;
                    }
                    continue;
                }

                let trimmed = line_text.trim_start();

                if !fenced_tilde && trimmed.starts_with("```") {
                    fenced_backtick = !fenced_backtick;
                }
                if !fenced_backtick && trimmed.starts_with("~~~") {
                    fenced_tilde = !fenced_tilde;
                }
                if fenced_backtick || fenced_tilde {
                    if !line_start.forward_line() {
                        break;
                    }
                    continue;
                }

                if let Some((level, title)) = parse_atx_heading(trimmed) {
                    let mark = obj.create_mark(None, &line_start, true);
                    let line = obj.iter_at_mark(&mark).line();
                    new_toc.push(MarkdownHeading {
                        level,
                        line,
                        text: title.to_owned(),
                        mark,
                    });
                }

                if !line_start.forward_line() {
                    break;
                }
            }

            let mut toc = self.table_of_contents.borrow_mut();
            let toc_changed = toc.len() != new_toc.len()
                || toc.iter().zip(new_toc.iter()).any(|(old, new)| old != new);

            for h in toc.drain(..) {
                obj.delete_mark(&h.mark);
            }

            *toc = new_toc;
            drop(toc);

            if toc_changed {
                obj.emit_by_name::<()>("toc-changed", &[]);
            }
        }
    }

    /// Start must be trimmed
    fn parse_atx_heading(line: &str) -> Option<(usize, &str)> {
        let level = line.chars().take_while(|&c| c == '#').count();
        if !(1..=6).contains(&level) {
            return None;
        }

        let after_level = &line[level..];
        if after_level.is_empty() // No title is a valid heading
            || after_level.starts_with(' ') // else must have space or tab
            || after_level.starts_with('\t')
        {
            let title = after_level.trim().trim_end_matches('#').trim();
            Some((level, title))
        } else {
            None
        }
    }
}

#[cfg(feature = "installed")]
use crate::config::PKGDATADIR;
use crate::data::DocumentStats;
use adw::subclass::prelude::*;
use gtk::TextMark;
use gtk::glib;
use gtk::glib::Object;
use sourceview5::LanguageManager;
use sourceview5::StyleSchemeManager;
use sourceview5::prelude::*;

#[derive(Debug, Clone)]
pub struct MarkdownHeading {
    level: usize,
    line: i32,
    text: String,
    mark: TextMark,
}

impl MarkdownHeading {
    pub fn level(&self) -> usize {
        self.level
    }

    pub fn line(&self) -> i32 {
        self.line
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn mark(&self) -> &TextMark {
        &self.mark
    }
}

impl PartialEq for MarkdownHeading {
    fn eq(&self, other: &Self) -> bool {
        self.level == other.level && self.line == other.line && self.text == other.text
    }
}

glib::wrapper! {
    pub struct MarkdownBuffer(ObjectSubclass<imp::MarkdownBuffer>)
        @extends sourceview5::Buffer, gtk::TextBuffer;
}

impl Default for MarkdownBuffer {
    fn default() -> Self {
        let obj: Self = Object::builder().build();
        let lm = LanguageManager::default();
        obj.set_language(Some(&lm.language("markdown").unwrap()));
        obj
    }
}

impl MarkdownBuffer {
    pub fn with_style_scheme(self, scheme_id: &str) -> Self {
        if let Some(style_scheme) = StyleSchemeManager::default().scheme(scheme_id) {
            self.set_style_scheme(Some(&style_scheme));
        } else {
            println!("MarkdownBuffer: Failed to load scheme with id '{scheme_id}'.")
        }
        self
    }

    pub fn set_theme(&self, scheme_id: &str) {
        if let Some(style_scheme) = StyleSchemeManager::default().scheme(scheme_id) {
            self.set_style_scheme(Some(&style_scheme));
        } else {
            println!("MarkdownBuffer: Failed to load scheme with id '{scheme_id}'.")
        }
    }

    pub fn stats(&self) -> DocumentStats {
        let num_lines = self.line_count();
        let num_chars = self.char_count();
        let mut num_spaces = 0;
        let mut num_words = 0;
        let mut prev_whitespace = true;
        for i in 0..num_lines {
            let start = self.iter_at_line(i).unwrap();
            let end = self.iter_at_line(i + 1).unwrap_or_else(|| self.end_iter());
            let text = self.text(&start, &end, true);
            for char in text.chars() {
                let is_whitespace = char.is_whitespace();
                if is_whitespace {
                    num_spaces += 1;
                } else if prev_whitespace {
                    num_words += 1;
                }
                prev_whitespace = is_whitespace;
            }
        }
        DocumentStats {
            num_lines,
            num_chars,
            num_spaces,
            num_words,
        }
    }

    pub fn table_of_contents(&self) -> std::cell::Ref<'_, Vec<MarkdownHeading>> {
        self.imp().table_of_contents.borrow()
    }

    /// Tell the buffer that a paste has been started
    pub fn open_paste(&self) {
        self.imp().paste_in_progress.replace(true);
    }

    pub fn format_bold(&self) {
        self.imp().formatting_action_in_progress.set(true);
        formatting::format_bold(self);
        self.imp().formatting_action_in_progress.set(false);
    }

    pub fn format_italic(&self) {
        self.imp().formatting_action_in_progress.set(true);
        formatting::format_italic(self);
        self.imp().formatting_action_in_progress.set(false);
    }

    pub fn format_strikethrough(&self) {
        self.imp().formatting_action_in_progress.set(true);
        formatting::format_strikethrough(self);
        self.imp().formatting_action_in_progress.set(false);
    }

    pub fn format_highlight(&self) {
        self.imp().formatting_action_in_progress.set(true);
        formatting::format_highlight(self);
        self.imp().formatting_action_in_progress.set(false);
    }

    pub fn format_heading(&self, heading_level: i32) {
        self.imp().formatting_action_in_progress.set(true);
        formatting::format_heading(self, heading_level);
        self.imp().formatting_action_in_progress.set(false);
    }

    pub fn format_blockquote(&self) {
        self.imp().formatting_action_in_progress.set(true);
        formatting::format_blockquote(self);
        self.imp().formatting_action_in_progress.set(false);
    }

    pub fn format_insert_link(&self) {
        self.imp().formatting_action_in_progress.set(true);
        formatting::format_insert_link(self);
        self.imp().formatting_action_in_progress.set(false);
    }

    pub fn format_code(&self) {
        self.imp().formatting_action_in_progress.set(true);
        formatting::format_code(self);
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! init {
        () => {
            if !gtk::is_initialized() {
                gtk::init().unwrap();
            }
        };
    }

    /// Create buffer with text
    macro_rules! buf {
        ( $t:expr ) => {{
            let buffer = MarkdownBuffer::default();
            buffer.set_text($t);
            buffer
        }};
    }

    #[test]
    fn test_autoclose_italic() {
        if !gtk::is_initialized() {
            gtk::init().unwrap();
        }
        let buf = buf!("");
        buf.insert_at_cursor("*");
        assert_eq!(buf.text(&buf.start_iter(), &buf.end_iter(), false), "**");
        assert_eq!(buf.cursor_position(), 1);
    }

    #[test]
    fn test_autoclose_bold() {
        if !gtk::is_initialized() {
            gtk::init().unwrap();
        }

        let buf = buf!("**");
        buf.place_cursor(&buf.iter_at_offset(1));
        buf.insert_at_cursor("*");
        assert_eq!(buf.text(&buf.start_iter(), &buf.end_iter(), false), "****");
        assert_eq!(buf.cursor_position(), 2);

        buf.insert_at_cursor("*");
        assert_eq!(buf.text(&buf.start_iter(), &buf.end_iter(), false), "****");
        assert_eq!(buf.cursor_position(), 3);

        buf.insert_at_cursor("*");
        assert_eq!(buf.text(&buf.start_iter(), &buf.end_iter(), false), "****");
        assert_eq!(buf.cursor_position(), 4);
    }
}
*/
