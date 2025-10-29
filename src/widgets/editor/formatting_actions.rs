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
