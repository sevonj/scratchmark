use gtk::glib;
use std::path::PathBuf;

use crate::APP_ID;

/// User data directory
pub fn path_userdata() -> PathBuf {
    let path = glib::user_data_dir().join(APP_ID);
    std::fs::create_dir_all(&path).expect("Couldn't create dir for userdata");
    path
}

/// Library directory inside userdata
pub fn path_builtin_library() -> PathBuf {
    let path = path_userdata().join("library");
    std::fs::create_dir_all(&path).expect("Couldn't create dir for builtin_library");
    path
}
