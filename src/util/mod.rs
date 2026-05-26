pub mod file_actions;
pub mod regex;

pub fn process_file_ext_text(text: &str) -> String {
    text.to_lowercase().trim_start_matches(".").to_string()
}
