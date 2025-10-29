use adw::prelude::*;

use gtk::TextBuffer;
use gtk::glib::Regex;
use gtk::glib::RegexCompileFlags;
use gtk::glib::RegexMatchFlags;

pub fn format_bold(buffer: TextBuffer) {
    let Some((start, end)) = buffer.selection_bounds() else {
        let mut iter = buffer.iter_at_mark(&buffer.get_insert());
        let start_off = iter.offset();
        buffer.insert(&mut iter, "****");
        let end_off = iter.offset();
        let start = buffer.iter_at_offset(start_off);
        let end = buffer.iter_at_offset(end_off);
        buffer.select_range(&start, &end);
        return;
    };
    let offset = start.offset();
    let selection = buffer.text(&start, &end, false);

    let is_bold = selection.len() >= 4 && selection.starts_with("**") && selection.ends_with("**");
    let is_italic =
        !is_bold && selection.len() >= 2 && selection.starts_with("*") && selection.ends_with("*");

    let replacement = if is_bold {
        selection[2..(selection.len() - 2)].to_owned()
    } else if is_italic {
        format!("*{selection}*")
    } else {
        format!("**{selection}**")
    };

    buffer.delete_selection(true, true);
    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
    buffer.insert(&mut iter, &replacement);

    let ins = buffer.iter_at_offset(offset);
    let bound = buffer.iter_at_offset(offset + replacement.len() as i32);
    buffer.select_range(&ins, &bound);
}

pub fn format_italic(buffer: TextBuffer) {
    let Some((start, end)) = buffer.selection_bounds() else {
        let mut iter = buffer.iter_at_mark(&buffer.get_insert());
        let start_off = iter.offset();
        buffer.insert(&mut iter, "**");
        let end_off = iter.offset();
        let start = buffer.iter_at_offset(start_off);
        let end = buffer.iter_at_offset(end_off);
        buffer.select_range(&start, &end);
        return;
    };
    let offset = start.offset();
    let selection = buffer.text(&start, &end, false);

    let is_bold = selection.len() >= 4 && selection.starts_with("**") && selection.ends_with("**");
    let is_italic =
        !is_bold && selection.len() >= 2 && selection.starts_with("*") && selection.ends_with("*");

    let replacement = if is_bold || is_italic {
        selection[1..(selection.len() - 1)].to_owned()
    } else {
        format!("*{selection}*")
    };

    buffer.delete_selection(true, true);
    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
    buffer.insert(&mut iter, &replacement);

    let ins = buffer.iter_at_offset(offset);
    let bound = buffer.iter_at_offset(offset + replacement.len() as i32);
    buffer.select_range(&ins, &bound);
}

pub fn format_strikethrough(buffer: TextBuffer) {
    let Some((start, end)) = buffer.selection_bounds() else {
        let mut iter = buffer.iter_at_mark(&buffer.get_insert());
        let start_off = iter.offset();
        buffer.insert(&mut iter, "~~~~");
        let end_off = iter.offset();
        let start = buffer.iter_at_offset(start_off);
        let end = buffer.iter_at_offset(end_off);
        buffer.select_range(&start, &end);
        return;
    };
    let offset = start.offset();
    let selection = buffer.text(&start, &end, false);

    let is_strikethrough =
        selection.len() >= 4 && selection.starts_with("~~") && selection.ends_with("~~");

    let replacement = if is_strikethrough {
        selection[2..(selection.len() - 2)].to_owned()
    } else {
        format!("~~{selection}~~")
    };

    buffer.delete_selection(true, true);
    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
    buffer.insert(&mut iter, &replacement);

    let ins = buffer.iter_at_offset(offset);
    let bound = buffer.iter_at_offset(offset + replacement.len() as i32);
    buffer.select_range(&ins, &bound);
}

pub fn format_heading(buffer: TextBuffer, heading_size: i32) {
    let insert = buffer.get_insert();
    let insert_iter = buffer.iter_at_mark(&insert);
    let current_line = insert_iter.line();
    let Some(mut start) = buffer.iter_at_line(current_line) else {
        return;
    };
    let mut end = buffer
        .iter_at_line(current_line + 1)
        .unwrap_or_else(|| buffer.end_iter());
    if end.line() != current_line {
        end.backward_char();
    }

    let old_line = buffer.text(&start, &end, false);

    let new_header = String::from("#").repeat(heading_size as usize) + " ";

    let any_size_heading = Regex::new(
        "^##* ",
        RegexCompileFlags::DEFAULT,
        RegexMatchFlags::DEFAULT,
    )
    .unwrap()
    .unwrap();
    let any_size_match = any_size_heading.match_(old_line.as_gstr(), RegexMatchFlags::DEFAULT);

    let replacement = if old_line.starts_with(&new_header) {
        old_line[(new_header.len())..].to_owned()
    } else if any_size_match
        .as_ref()
        .map(|m| m.matches())
        .unwrap_or(false)
    {
        let old_header_len = any_size_match.unwrap().fetch(0).unwrap().len();
        let without_header = &old_line[old_header_len..];
        format!("{new_header}{without_header}")
    } else {
        format!("{new_header}{old_line}")
    };

    buffer.delete(&mut start, &mut end);
    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
    buffer.insert(&mut iter, &replacement);
}

pub fn format_blockquote(buffer: TextBuffer) {
    let (selection_start, selection_end) = buffer.selection_bounds().unwrap_or_else(|| {
        let iter = buffer.iter_at_mark(&buffer.get_insert());
        (iter, iter)
    });
    let first_line = selection_start.line();
    let last_line = selection_end.line();

    let mut start_iter = buffer.iter_at_line(first_line).unwrap();
    let mut end_iter = buffer
        .iter_at_line(last_line + 1)
        .map(|i| buffer.iter_at_offset(i.offset() - 1))
        .unwrap_or_else(|| buffer.end_iter());

    let old_contents = buffer.text(&start_iter, &end_iter, true);
    let mut new_contents = String::new();

    if is_selection_blockquote(first_line, last_line, &buffer) {
        for (i, line) in old_contents.split('\n').enumerate() {
            let ln = first_line as usize + i;
            if ln > last_line as usize {
                break;
            }
            if line.chars().nth(1).is_some_and(|c| c.is_whitespace()) {
                new_contents += &line[2..];
            } else {
                new_contents += &line[1..];
            }
            if ln < last_line as usize {
                new_contents += "\n";
            }
        }
    } else {
        for (i, line) in old_contents.split('\n').enumerate() {
            let ln = first_line as usize + i;
            if ln > last_line as usize {
                break;
            }
            new_contents += &format!("> {line}");
            if ln < last_line as usize {
                new_contents += "\n";
            }
        }
    }

    buffer.delete(&mut start_iter, &mut end_iter);

    let mut start_iter = buffer.iter_at_line(first_line).unwrap();
    buffer.insert(&mut start_iter, &new_contents);

    let start_iter = buffer.iter_at_line(first_line).unwrap();

    let end_iter = buffer
        .iter_at_line(last_line + 1)
        .map(|i| buffer.iter_at_offset(i.offset() - 1))
        .unwrap_or_else(|| buffer.end_iter());

    buffer.select_range(&start_iter, &end_iter);
}

pub fn format_code(buffer: TextBuffer) {
    let Some((start, end)) = buffer.selection_bounds() else {
        return;
    };
    let offset = start.offset();
    let selection = buffer.text(&start, &end, false);

    let is_code = selection.len() >= 2 && selection.starts_with("`") && selection.ends_with("`");

    let replacement = if is_code {
        selection[1..(selection.len() - 1)].to_owned()
    } else {
        format!("`{selection}`")
    };

    buffer.delete_selection(true, true);
    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
    buffer.insert(&mut iter, &replacement);

    let ins = buffer.iter_at_offset(offset);
    let bound = buffer.iter_at_offset(offset + replacement.len() as i32);
    buffer.select_range(&ins, &bound);
}

/// True if **every** line in selection matches.
fn is_selection_blockquote(first_line: i32, last_line: i32, buffer: &TextBuffer) -> bool {
    for line in first_line..=last_line {
        let start_iter = buffer.iter_at_line(line).unwrap();
        let end_iter = buffer
            .iter_at_line(line + 1)
            .unwrap_or_else(|| buffer.end_iter());
        let line_contents = buffer.text(&start_iter, &end_iter, true);
        if !line_contents.starts_with(">") {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create buffer with text
    #[macro_export]
    macro_rules! buf {
        ( $t:expr ) => {{
            let buffer = TextBuffer::default();
            buffer.set_text($t);
            buffer
        }};
    }

    #[macro_export]
    macro_rules! select_all {
        ( $buf:expr ) => {{ $buf.select_range(&$buf.start_iter(), &$buf.end_iter()) }};
    }

    /// Get full contents of buffer
    #[macro_export]
    macro_rules! contents {
        ( $buf:expr ) => {{ $buf.text(&$buf.start_iter(), &$buf.end_iter(), true) }};
    }

    /// Get full contents of buffer
    #[macro_export]
    macro_rules! selection {
        ( $buf:expr ) => {{
            let (start_iter, end_iter) = $buf.selection_bounds().unwrap();
            $buf.text(&start_iter, &end_iter, true)
        }};
    }

    #[test]
    fn test_format_bold_empty() {
        let buffer = buf!("");
        select_all!(&buffer);
        format_bold(buffer.clone());
        assert_eq!(contents!(buffer), "****");
        format_bold(buffer.clone());
        assert_eq!(contents!(buffer), "");
    }

    #[test]
    fn test_format_bold() {
        let buffer = buf!("text");
        select_all!(&buffer);
        format_bold(buffer.clone());
        assert_eq!(contents!(buffer), "**text**");
        format_bold(buffer.clone());
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_no_selection_within_word() {
        let buffer = buf!("text");
        buffer.place_cursor(&buffer.iter_at_offset(2));
        format_bold(buffer.clone());
        assert_eq!(contents!(buffer), "te****xt");
        format_bold(buffer.clone());
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_surrounded_by_whitespace() {
        let buffer = buf!("  text\ntext\ntext  \n");
        select_all!(&buffer);
        format_bold(buffer.clone());
        assert_eq!(contents!(buffer), "**  text\ntext\ntext  \n**");
        format_bold(buffer.clone());
        assert_eq!(contents!(buffer), "  text\ntext\ntext  \n");
    }

    #[test]
    fn test_format_blockquote_empty() {
        let buffer = buf!("");
        select_all!(&buffer);
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "> ");
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "");
    }

    #[test]
    fn test_format_blockquote_word() {
        let buffer = buf!("text");
        select_all!(&buffer);
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "> text");
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_blockquote_word_newline() {
        let buffer = buf!("text\n");
        select_all!(&buffer);
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "> text\n> ");
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "text\n");
    }

    #[test]
    fn test_format_blockquote_word_manylines() {
        let buffer = buf!("text\ntext\ntext");
        select_all!(&buffer);
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "> text\n> text\n> text");
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "text\ntext\ntext");
    }

    #[test]
    fn test_format_blockquote_word_manylines_trail() {
        let buffer = buf!("text\ntext\ntext\n");
        select_all!(&buffer);
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "> text\n> text\n> text\n> ");
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "text\ntext\ntext\n");
    }

    #[test]
    fn test_format_blockquote_word_line_between() {
        let buffer = buf!("first\nsecond\nlast");
        buffer.select_range(
            &buffer.iter_at_line(1).unwrap(),
            &buffer.iter_at_line_offset(1, 2).unwrap(),
        );
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "first\n> second\nlast");
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "first\nsecond\nlast");
    }

    #[test]
    fn test_format_blockquote_word_lines_between() {
        let buffer = buf!("first\nsecond\n\n\nlast");
        buffer.select_range(
            &buffer.iter_at_line(1).unwrap(),
            &buffer.iter_at_line(2).unwrap(),
        );
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "first\n> second\n> \n\nlast");
        format_blockquote(buffer.clone());
        assert_eq!(contents!(buffer), "first\nsecond\n\n\nlast");
    }

    #[test]
    fn test_is_selection_blockquote() {
        let buffer = buf!("");
        select_all!(&buffer);
        let endln = buffer.end_iter().line();
        assert_eq!(is_selection_blockquote(0, endln, &buffer), false);

        let buffer = buf!(">");
        select_all!(&buffer);
        let endln = buffer.end_iter().line();
        assert_eq!(is_selection_blockquote(0, endln, &buffer), true);

        let buffer = buf!("> ");
        select_all!(&buffer);
        let endln = buffer.end_iter().line();
        assert_eq!(is_selection_blockquote(0, endln, &buffer), true);

        let buffer = buf!("> text text text");
        select_all!(&buffer);
        let endln = buffer.end_iter().line();
        assert_eq!(is_selection_blockquote(0, endln, &buffer), true);

        let buffer = buf!("> text text text\n");
        select_all!(&buffer);
        let endln = buffer.end_iter().line();
        assert_eq!(is_selection_blockquote(0, endln, &buffer), false);

        let buffer = buf!("> text text text\ntext");
        select_all!(&buffer);
        let endln = buffer.end_iter().line();
        assert_eq!(is_selection_blockquote(0, endln, &buffer), false);

        let buffer = buf!("> text text text\n>");
        select_all!(&buffer);
        let endln = buffer.end_iter().line();
        assert_eq!(is_selection_blockquote(0, endln, &buffer), true);

        let buffer = buf!("> text\n>\n> text");
        select_all!(&buffer);
        let endln = buffer.end_iter().line();
        assert_eq!(is_selection_blockquote(0, endln, &buffer), true);

        let buffer = buf!("> text\n\n> text");
        select_all!(&buffer);
        let endln = buffer.end_iter().line();
        assert_eq!(is_selection_blockquote(0, endln, &buffer), false);
    }
}
