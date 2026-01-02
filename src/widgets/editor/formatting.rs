use adw::prelude::*;

use gtk::TextBuffer;
use gtk::TextIter;

use super::regex;

pub fn format_bold(buffer: &TextBuffer) {
    if let Some((start, mut end)) = find_delim_range(buffer, "**") {
        // Already bold, remove it
        let start_off = start.offset();
        let end_off = end.offset();

        buffer.begin_user_action();
        buffer.delete(&mut buffer.iter_at_offset(end_off - 2), &mut end);
        buffer.delete(
            &mut buffer.iter_at_offset(start_off),
            &mut buffer.iter_at_offset(start_off + 2),
        );
        buffer.end_user_action();
        return;
    }

    if let Some((start, mut end)) = buffer
        .selection_bounds()
        .or_else(|| word_around_cursor(buffer))
    {
        let start_off = start.offset();

        buffer.begin_user_action();
        buffer.insert(&mut end, "**");
        buffer.insert(&mut buffer.iter_at_offset(start_off), "**");
        buffer.end_user_action();
        return;
    }

    buffer.begin_user_action();
    buffer.insert_at_cursor("****");
    buffer.place_cursor(&buffer.iter_at_offset(buffer.cursor_position() - 2));
    buffer.end_user_action();
}

pub fn format_italic(buffer: &TextBuffer) {
    let is_bold = find_delim_range(buffer, "**").is_some();
    let is_both = find_delim_range(buffer, "***").is_some();

    if (!is_bold || is_both)
        && let Some((start, mut end)) = find_delim_range(buffer, "*")
    {
        // Already italic, remove it
        let start_off = start.offset();
        let end_off = end.offset();

        buffer.begin_user_action();
        buffer.delete(&mut buffer.iter_at_offset(end_off - 1), &mut end);
        buffer.delete(
            &mut buffer.iter_at_offset(start_off),
            &mut buffer.iter_at_offset(start_off + 1),
        );
        buffer.end_user_action();
        return;
    }

    if let Some((start, mut end)) = buffer
        .selection_bounds()
        .or_else(|| word_around_cursor(buffer))
    {
        let start_off = start.offset();

        buffer.begin_user_action();
        buffer.insert(&mut end, "*");
        buffer.insert(&mut buffer.iter_at_offset(start_off), "*");
        buffer.end_user_action();
        return;
    }

    buffer.begin_user_action();
    buffer.insert_at_cursor("**");
    buffer.place_cursor(&buffer.iter_at_offset(buffer.cursor_position() - 1));
    buffer.end_user_action();
}

pub fn format_strikethrough(buffer: &TextBuffer) {
    if let Some((start, mut end)) = find_delim_range(buffer, "~~") {
        let start_off = start.offset();
        let end_off = end.offset();

        buffer.begin_user_action();
        buffer.delete(&mut buffer.iter_at_offset(end_off - 2), &mut end);
        buffer.delete(
            &mut buffer.iter_at_offset(start_off),
            &mut buffer.iter_at_offset(start_off + 2),
        );
        buffer.end_user_action();
        return;
    }

    if let Some((start, mut end)) = buffer
        .selection_bounds()
        .or_else(|| word_around_cursor(buffer))
    {
        let start_off = start.offset();

        buffer.begin_user_action();
        buffer.insert(&mut end, "~~");
        buffer.insert(&mut buffer.iter_at_offset(start_off), "~~");
        buffer.end_user_action();
        return;
    }

    buffer.begin_user_action();
    buffer.insert_at_cursor("~~~~");
    buffer.place_cursor(&buffer.iter_at_offset(buffer.cursor_position() - 2));
    buffer.end_user_action();
}

pub fn format_highlight(buffer: &TextBuffer) {
    if let Some((start, mut end)) = find_delim_range(buffer, "==") {
        let start_off = start.offset();
        let end_off = end.offset();

        buffer.begin_user_action();
        buffer.delete(&mut buffer.iter_at_offset(end_off - 2), &mut end);
        buffer.delete(
            &mut buffer.iter_at_offset(start_off),
            &mut buffer.iter_at_offset(start_off + 2),
        );
        buffer.end_user_action();
        return;
    }

    if let Some((start, mut end)) = buffer
        .selection_bounds()
        .or_else(|| word_around_cursor(buffer))
    {
        let start_off = start.offset();

        buffer.begin_user_action();
        buffer.insert(&mut end, "==");
        buffer.insert(&mut buffer.iter_at_offset(start_off), "==");
        buffer.end_user_action();
        return;
    }

    buffer.begin_user_action();
    buffer.insert_at_cursor("====");
    buffer.place_cursor(&buffer.iter_at_offset(buffer.cursor_position() - 2));
    buffer.end_user_action();
}

pub fn format_heading(buffer: &TextBuffer, heading_level: i32) {
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
    let regex_match = regex::ATX_H_OPENING.find(&old_line);
    let old_heading_level = regex_match
        .map(|m| m.as_str().chars().filter(|c| *c == '#').count())
        .unwrap_or(0) as i32;

    let stripped_line = match regex_match {
        Some(regex_match) => old_line[regex_match.len()..].to_owned(),
        None => old_line.to_string(),
    };

    buffer.begin_user_action();
    buffer.delete(&mut start, &mut end);
    if old_heading_level != heading_level {
        buffer.insert_at_cursor(&(String::from("#").repeat(heading_level as usize) + " "));
    }
    buffer.insert_at_cursor(&stripped_line);
    buffer.end_user_action();
}

pub fn format_blockquote(buffer: &TextBuffer) {
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

    if is_selection_blockquote(first_line, last_line, buffer) {
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

    buffer.begin_user_action();

    buffer.delete(&mut start_iter, &mut end_iter);

    let mut start_iter = buffer.iter_at_line(first_line).unwrap();
    buffer.insert(&mut start_iter, &new_contents);

    let start_iter = buffer.iter_at_line(first_line).unwrap();

    let end_iter = buffer
        .iter_at_line(last_line + 1)
        .map(|i| buffer.iter_at_offset(i.offset() - 1))
        .unwrap_or_else(|| buffer.end_iter());

    buffer.select_range(&start_iter, &end_iter);

    buffer.end_user_action();
}

pub fn format_code(buffer: &TextBuffer) {
    if let Some((start, mut end)) = find_delim_range(buffer, "`") {
        let start_off = start.offset();
        let end_off = end.offset();

        buffer.begin_user_action();
        buffer.delete(&mut buffer.iter_at_offset(end_off - 1), &mut end);
        buffer.delete(
            &mut buffer.iter_at_offset(start_off),
            &mut buffer.iter_at_offset(start_off + 1),
        );
        buffer.end_user_action();
        return;
    }

    if let Some((start, mut end)) = buffer
        .selection_bounds()
        .or_else(|| word_around_cursor(buffer))
    {
        let start_off = start.offset();

        buffer.begin_user_action();
        buffer.insert(&mut end, "`");
        buffer.insert(&mut buffer.iter_at_offset(start_off), "`");
        buffer.end_user_action();
        return;
    }

    buffer.begin_user_action();
    buffer.insert_at_cursor("``");
    buffer.place_cursor(&buffer.iter_at_offset(buffer.cursor_position() - 1));
    buffer.end_user_action();
}

fn range_around_cursor(buffer: &TextBuffer, distance: i32) -> Option<(TextIter, TextIter)> {
    let cursor_pos = buffer.iter_at_mark(&buffer.get_insert());
    let start_off = cursor_pos.offset() - distance;
    let end_off = cursor_pos.offset() + distance;
    if start_off < 0 || end_off > buffer.end_iter().offset() {
        return None;
    }
    let start_iter = buffer.iter_at_offset(start_off);
    let end_iter = buffer.iter_at_offset(end_off);
    Some((start_iter, end_iter))
}

fn word_around_cursor(buffer: &TextBuffer) -> Option<(TextIter, TextIter)> {
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let mut start = cursor;
    let mut end = cursor;

    loop {
        let mut before_start = start;
        if !before_start.backward_char() {
            break; // Can't go further back
        }
        if buffer
            .text(&before_start, &start, false)
            .chars()
            .next()
            .unwrap()
            .is_whitespace()
        {
            break;
        }
        start = before_start;
    }

    loop {
        let mut after_end = end;
        if !after_end.forward_char() {
            // Can't go further forward
            after_end = buffer.end_iter();
        }
        if buffer
            .text(&end, &after_end, false)
            .chars()
            .next()
            .is_none_or(|c| c.is_whitespace())
        {
            break;
        }
        end = after_end;
    }

    if start.offset() == end.offset() {
        return None;
    }
    Some((start, end))
}

/// Tries to find a range immediately around the cursor or selection starts and ends with delimiter
fn find_delim_range(buffer: &TextBuffer, delimiter: &str) -> Option<(TextIter, TextIter)> {
    if let Some((mut start, mut end)) = buffer
        .selection_bounds()
        .or_else(|| word_around_cursor(buffer))
    {
        // Move start back to find delimiter
        for _ in 0..(delimiter.len() + 1) {
            if buffer.text(&start, &end, false).starts_with(delimiter) {
                break;
            }
            if !start.backward_char() {
                return None;
            };
        }
        if !buffer.text(&start, &end, false).starts_with(delimiter) {
            return None;
        }
        // Move end forward to find delimiter
        for _ in 0..(delimiter.len() + 1) {
            if buffer.text(&start, &end, false).ends_with(delimiter) {
                break;
            }
            if !end.forward_char() {
                end = buffer.end_iter();
                break;
            };
        }
        let final_text = buffer.text(&start, &end, false);
        if !final_text.ends_with(delimiter) || final_text.len() < delimiter.len() * 2 {
            return None;
        }
        return Some((start, end));
    } else if let Some((start, end)) = range_around_cursor(buffer, 2)
        && {
            let text = buffer.text(&start, &end, false);
            text.starts_with(delimiter) && text.ends_with(delimiter)
        }
    {
        return Some((start, end));
    }
    None
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
    macro_rules! buf {
        ( $t:expr ) => {{
            let buffer = TextBuffer::default();
            buffer.set_text($t);
            buffer
        }};
    }

    macro_rules! select_all {
        ( $buf:expr ) => {{ $buf.select_range(&$buf.start_iter(), &$buf.end_iter()) }};
    }

    /// Get full contents of buffer
    macro_rules! contents {
        ( $buf:expr ) => {{ $buf.text(&$buf.start_iter(), &$buf.end_iter(), true) }};
    }

    #[test]
    fn test_format_bold_empty() {
        let buffer = buf!("");
        select_all!(&buffer);
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "****");
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "");
    }

    #[test]
    fn test_format_bold() {
        let buffer = buf!("text");
        select_all!(&buffer);
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "**text**");
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_bold_cursor_placement() {
        let buffer = buf!("text  text");
        buffer.place_cursor(&buffer.iter_at_offset(5));
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "text **** text");
        buffer.place_cursor(&buffer.iter_at_offset(7));
        format_bold(&buffer);
        let buffer = buf!("text  text");
        buffer.place_cursor(&buffer.iter_at_offset(5));
    }

    #[test]
    fn test_format_bold_no_selection_within_word() {
        let buffer = buf!("text");
        buffer.place_cursor(&buffer.iter_at_offset(2));
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "**text**");
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_bold_surrounded_by_whitespace() {
        let buffer = buf!("  text\ntext\ntext  \n");
        select_all!(&buffer);
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "**  text\ntext\ntext  \n**");
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "  text\ntext\ntext  \n");
    }

    #[test]
    fn test_format_bold_from_italic() {
        let buffer = buf!("*text*");
        select_all!(&buffer);
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "***text***");
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "*text*");
    }

    #[test]
    fn test_format_bold_from_italic_within_word() {
        let buffer = buf!("*text*");
        buffer.place_cursor(&buffer.iter_at_offset(3));
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "***text***");
        format_bold(&buffer);
        assert_eq!(contents!(buffer), "*text*");
    }

    #[test]
    fn test_format_italic_empty() {
        let buffer = buf!("");
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "**");
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "");
    }

    #[test]
    fn test_format_italic() {
        let buffer = buf!("text");
        select_all!(&buffer);
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "*text*");
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_italic_cursor_placement() {
        let buffer = buf!("text  text");
        buffer.place_cursor(&buffer.iter_at_offset(5));
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "text ** text");
        buffer.place_cursor(&buffer.iter_at_offset(6));
        format_italic(&buffer);
        let buffer = buf!("text  text");
        buffer.place_cursor(&buffer.iter_at_offset(5));
    }

    #[test]
    fn test_format_italic_no_selection_within_word() {
        let buffer = buf!("text");
        buffer.place_cursor(&buffer.iter_at_offset(2));
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "*text*");
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_italic_from_bold() {
        let buffer = buf!("**text**");
        select_all!(&buffer);
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "***text***");
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "**text**");
    }

    #[test]
    fn test_format_italic_from_bold_within_word() {
        let buffer = buf!("**text**");
        buffer.place_cursor(&buffer.iter_at_offset(4));
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "***text***");
        format_italic(&buffer);
        assert_eq!(contents!(buffer), "**text**");
    }

    #[test]
    fn test_format_strikethrough() {
        let buffer = buf!("text");
        select_all!(&buffer);
        format_strikethrough(&buffer);
        assert_eq!(contents!(buffer), "~~text~~");
        format_strikethrough(&buffer);
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_strikethrough_multiline() {
        let buffer = buf!("text\n");
        select_all!(&buffer);
        format_strikethrough(&buffer);
        assert_eq!(contents!(buffer), "~~text\n~~");
        format_strikethrough(&buffer);
        assert_eq!(contents!(buffer), "text\n");
    }

    #[test]
    fn test_format_strikethrough_mid_multiline() {
        let buffer = buf!("text\n");
        buffer.select_range(&buffer.start_iter(), &buffer.iter_at_offset(4));
        format_strikethrough(&buffer);
        assert_eq!(contents!(buffer), "~~text~~\n");
        format_strikethrough(&buffer);
        assert_eq!(contents!(buffer), "text\n");
    }

    #[test]
    fn test_format_highlight() {
        let buffer = buf!("text");
        select_all!(&buffer);
        format_highlight(&buffer);
        assert_eq!(contents!(buffer), "==text==");
        format_highlight(&buffer);
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_highlight_multiline() {
        let buffer = buf!("text\n");
        select_all!(&buffer);
        format_highlight(&buffer);
        assert_eq!(contents!(buffer), "==text\n==");
        format_highlight(&buffer);
        assert_eq!(contents!(buffer), "text\n");
    }

    #[test]
    fn test_format_highlight_mid_multiline() {
        let buffer = buf!("text\n");
        buffer.select_range(&buffer.start_iter(), &buffer.iter_at_offset(4));
        format_highlight(&buffer);
        assert_eq!(contents!(buffer), "==text==\n");
        format_highlight(&buffer);
        assert_eq!(contents!(buffer), "text\n");
    }

    #[test]
    fn test_format_heading() {
        let buffer = buf!("text");
        select_all!(&buffer);
        format_heading(&buffer, 1);
        assert_eq!(contents!(buffer), "# text");
        format_heading(&buffer, 2);
        assert_eq!(contents!(buffer), "## text");
        format_heading(&buffer, 2);
        assert_eq!(contents!(buffer), "text");
        format_heading(&buffer, 6);
        assert_eq!(contents!(buffer), "###### text");
        format_heading(&buffer, 5);
        assert_eq!(contents!(buffer), "##### text");
        format_heading(&buffer, 5);
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_heading_indent() {
        let buffer = buf!(" # text");
        format_heading(&buffer, 2);
        assert_eq!(contents!(buffer), "## text");
        let buffer = buf!("   # text");
        format_heading(&buffer, 1);
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_heading_nospace() {
        let buffer = buf!("#text");
        format_heading(&buffer, 2);
        assert_eq!(contents!(buffer), "## #text");
        format_heading(&buffer, 1);
        assert_eq!(contents!(buffer), "# #text");
        format_heading(&buffer, 1);
        assert_eq!(contents!(buffer), "#text");
    }

    #[test]
    fn test_format_blockquote_empty() {
        let buffer = buf!("");
        select_all!(&buffer);
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "> ");
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "");
    }

    #[test]
    fn test_format_blockquote_word() {
        let buffer = buf!("text");
        select_all!(&buffer);
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "> text");
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "text");
    }

    #[test]
    fn test_format_blockquote_word_newline() {
        let buffer = buf!("text\n");
        select_all!(&buffer);
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "> text\n> ");
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "text\n");
    }

    #[test]
    fn test_format_blockquote_word_manylines() {
        let buffer = buf!("text\ntext\ntext");
        select_all!(&buffer);
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "> text\n> text\n> text");
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "text\ntext\ntext");
    }

    #[test]
    fn test_format_blockquote_word_manylines_trail() {
        let buffer = buf!("text\ntext\ntext\n");
        select_all!(&buffer);
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "> text\n> text\n> text\n> ");
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "text\ntext\ntext\n");
    }

    #[test]
    fn test_format_blockquote_word_line_between() {
        let buffer = buf!("first\nsecond\nlast");
        buffer.select_range(
            &buffer.iter_at_line(1).unwrap(),
            &buffer.iter_at_line_offset(1, 2).unwrap(),
        );
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "first\n> second\nlast");
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "first\nsecond\nlast");
    }

    #[test]
    fn test_format_blockquote_word_lines_between() {
        let buffer = buf!("first\nsecond\n\n\nlast");
        buffer.select_range(
            &buffer.iter_at_line(1).unwrap(),
            &buffer.iter_at_line(2).unwrap(),
        );
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "first\n> second\n> \n\nlast");
        format_blockquote(&buffer);
        assert_eq!(contents!(buffer), "first\nsecond\n\n\nlast");
    }

    #[test]
    fn test_find_delim_range() {
        let buffer = buf!("");
        select_all!(&buffer);
        assert!(find_delim_range(&buffer, "**").is_none());

        let buffer = buf!("*");
        select_all!(&buffer);
        assert!(find_delim_range(&buffer, "**").is_none());

        let buffer = buf!("**");
        select_all!(&buffer);
        assert!(find_delim_range(&buffer, "**").is_none());

        let buffer = buf!("***");
        select_all!(&buffer);
        assert!(find_delim_range(&buffer, "**").is_none());

        let buffer = buf!("****");
        select_all!(&buffer);
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "****");

        let buffer = buf!("****");
        buffer.place_cursor(&buffer.iter_at_offset(0));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "****");
        buffer.place_cursor(&buffer.iter_at_offset(1));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "****");
        buffer.place_cursor(&buffer.end_iter());
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "****");

        let buffer = buf!("Never **enough**");
        select_all!(&buffer);
        assert!(find_delim_range(&buffer, "**").is_none());

        let buffer = buf!("Never **enough** ");
        buffer.place_cursor(&buffer.iter_at_offset(0));
        assert!(find_delim_range(&buffer, "**").is_none());
        buffer.place_cursor(&buffer.iter_at_offset(4));
        assert!(find_delim_range(&buffer, "**").is_none());
        buffer.place_cursor(&buffer.iter_at_offset(5));
        assert!(find_delim_range(&buffer, "**").is_none());

        buffer.place_cursor(&buffer.iter_at_offset(6));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "**enough**");
        buffer.place_cursor(&buffer.iter_at_offset(9));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "**enough**");
        buffer.place_cursor(&buffer.iter_at_offset(16));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "**enough**");
        buffer.place_cursor(&buffer.end_iter());
        assert!(find_delim_range(&buffer, "**").is_none());

        // '**enough**'
        buffer.select_range(&buffer.iter_at_offset(6), &buffer.iter_at_offset(16));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "**enough**");
        // '*enough*'
        buffer.select_range(&buffer.iter_at_offset(7), &buffer.iter_at_offset(15));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "**enough**");
        // 'enough'
        buffer.select_range(&buffer.iter_at_offset(8), &buffer.iter_at_offset(14));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "**enough**");
        // '*enough'
        buffer.select_range(&buffer.iter_at_offset(7), &buffer.iter_at_offset(14));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "**enough**");
        // '*enough**'
        buffer.select_range(&buffer.iter_at_offset(7), &buffer.iter_at_offset(16));
        let (start, end) = find_delim_range(&buffer, "**").unwrap();
        assert_eq!(buffer.text(&start, &end, false), "**enough**");
        // '*enough** '
        buffer.select_range(&buffer.iter_at_offset(7), &buffer.iter_at_offset(17));
        assert!(find_delim_range(&buffer, "**").is_none());
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

    #[test]
    fn test_range_around_cursor() {
        let buffer = buf!("");
        assert_eq!(range_around_cursor(&buffer, 1), None);

        let buffer = buf!("******");
        buffer.place_cursor(&buffer.iter_at_offset(0));
        assert_eq!(range_around_cursor(&buffer, 1), None);

        let buffer = buf!("******");
        buffer.place_cursor(&buffer.iter_at_offset(6));
        assert_eq!(range_around_cursor(&buffer, 1), None);

        let buffer = buf!("******");
        buffer.place_cursor(&buffer.iter_at_offset(3));
        let (start_iter, end_iter) = range_around_cursor(&buffer, 2).unwrap();
        assert_eq!(start_iter.offset(), 1);
        assert_eq!(end_iter.offset(), 5);

        let buffer = buf!("******");
        buffer.place_cursor(&buffer.iter_at_offset(3));
        let (start_iter, end_iter) = range_around_cursor(&buffer, 3).unwrap();
        assert_eq!(start_iter.offset(), 0);
        assert_eq!(end_iter.offset(), 6);

        let buffer = buf!("******");
        buffer.place_cursor(&buffer.iter_at_offset(3));
        assert_eq!(range_around_cursor(&buffer, 4), None);

        let buffer = buf!("abcdefg");
        buffer.place_cursor(&buffer.iter_at_offset(3));
        let (start_iter, end_iter) = range_around_cursor(&buffer, 2).unwrap();
        assert_eq!(&buffer.text(&start_iter, &end_iter, true), "bcde");
    }

    #[test]
    fn test_word_around_cursor() {
        let buffer = buf!("spagublio");
        buffer.place_cursor(&buffer.iter_at_offset(0));
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(start.offset(), 0);
        assert_eq!(end.offset(), 9);
        buffer.place_cursor(&buffer.iter_at_offset(1));
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(start.offset(), 0);
        assert_eq!(end.offset(), 9);
        buffer.place_cursor(&buffer.iter_at_offset(8));
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(start.offset(), 0);
        assert_eq!(end.offset(), 9);
        buffer.place_cursor(&buffer.iter_at_offset(9));
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(start.offset(), 0);
        assert_eq!(end.offset(), 9);

        let buffer = buf!("Why don't we just move on to\nthe cutting of the monitors");
        buffer.place_cursor(&buffer.iter_at_offset(5)); // d_on't
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(start.offset(), 4);
        assert_eq!(end.offset(), 9);
        buffer.place_cursor(&buffer.iter_at_offset(26)); // _to
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(start.offset(), 26);
        assert_eq!(end.offset(), 28);
        buffer.place_cursor(&buffer.iter_at_offset(29)); // _the
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(start.offset(), 29);
        assert_eq!(end.offset(), 32);
        buffer.place_cursor(&buffer.end_iter());
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(start.offset(), buffer.end_iter().offset() - 8);
        assert_eq!(end.offset(), buffer.end_iter().offset());

        let buffer = buf!(" space around ");
        buffer.place_cursor(&buffer.iter_at_offset(0));
        assert!(word_around_cursor(&buffer).is_none());
        buffer.place_cursor(&buffer.end_iter());
        assert!(word_around_cursor(&buffer).is_none());
        buffer.place_cursor(&buffer.iter_at_offset(3));
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(buffer.text(&start, &end, false), "space");
        buffer.place_cursor(&buffer.iter_at_offset(12));
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(buffer.text(&start, &end, false), "around");
        buffer.place_cursor(&buffer.iter_at_offset(13));
        let (start, end) = word_around_cursor(&buffer).unwrap();
        assert_eq!(buffer.text(&start, &end, false), "around");
    }
}
