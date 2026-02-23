mod formatting;

mod imp {
    use std::cell::Cell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::glib;
    use sourceview5::prelude::*;
    use sourceview5::subclass::prelude::*;

    use glib::Properties;
    use gtk::TextIter;

    #[derive(Debug, Properties, Default)]
    #[properties(wrapper_type = super::MarkdownBuffer)]
    pub struct MarkdownBuffer {
        #[property(get, set)]
        pub(super) paste_in_progress: Cell<bool>,
        pub(super) formatting_action_in_progress: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MarkdownBuffer {
        const NAME: &'static str = "MarkdownBuffer";
        type Type = super::MarkdownBuffer;
        type ParentType = sourceview5::Buffer;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MarkdownBuffer {
        fn constructed(&self) {
            let obj = self.obj();
            obj.set_highlight_matching_brackets(false);
            self.parent_constructed();
        }
    }

    impl TextBufferImpl for MarkdownBuffer {
        // Inserted text passes through here
        // We can modify or cancel it here, and do other things, like move the cursor
        fn insert_text(&self, iter: &mut TextIter, new_text: &str) {
            let mut process_text = Some(new_text.to_owned());
            let cursor_move;

            (process_text, cursor_move) = self.process_auto_close_formatting(iter, process_text);

            if let Some(new_text) = process_text {
                self.parent_insert_text(iter, &new_text)
            }

            let obj = self.obj();
            let mut cursor = obj.iter_at_offset(obj.cursor_position());
            cursor.forward_cursor_positions(cursor_move);
            obj.place_cursor(&cursor);
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
            process_text: Option<String>,
        ) -> (Option<String>, i32) {
            if self.paste_in_progress.get() || self.formatting_action_in_progress.get() {
                return (process_text, 0);
            }
            let Some(process_text) = process_text else {
                return (process_text, 0);
            };

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
    }
}

use adw::subclass::prelude::*;
use gtk::glib;
use sourceview5::{StyleSchemeManager, prelude::*};

use gtk::glib::Object;
use sourceview5::LanguageManager;

#[cfg(feature = "installed")]
use crate::config::PKGDATADIR;
use crate::data::DocumentStats;

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
