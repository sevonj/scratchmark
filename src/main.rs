mod data;
mod error;
mod util;
mod widgets;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

use widgets::Window;

const APP_ID: &str = "org.scratchmark.Scratchmark";

fn main() -> glib::ExitCode {
    gio::resources_register_include!("gresources.gresource")
        .expect("Failed to register resources.");

    let app = adw::Application::builder().application_id(APP_ID).build();
    setup_accels(&app);

    app.connect_activate(|app| {
        let window = Window::new(app);
        window.set_title(Some("Scratchmark"));
        window.present();
    });

    app.run()
}

fn setup_accels(app: &adw::Application) {
    app.set_accels_for_action("win.file-close", &["<Ctrl>W"]);
    app.set_accels_for_action("win.file-new", &["<Ctrl>N"]);
    app.set_accels_for_action("win.file-rename-open", &["F2"]);
    app.set_accels_for_action("editor.format-bold", &["<Ctrl>B"]);
    app.set_accels_for_action("editor.format-italic", &["<Ctrl>I"]);
    app.set_accels_for_action("editor.show-search", &["<Ctrl>F"]);
    app.set_accels_for_action("editor.show-search-replace", &["<Ctrl>R"]);
    app.set_accels_for_action("editor.hide-search", &["Escape"]);
    app.set_accels_for_action("editor.shiftreturn", &["<Shift>Return"]);
    app.set_accels_for_action("win.library-refresh", &["F5"]);

    app.set_accels_for_action("win.toggle-sidebar", &["F9"]);
    app.set_accels_for_action("win.fullscreen", &["F11"]);
    app.set_accels_for_action("win.unfullscreen", &["F11"]);
    app.set_accels_for_action("win.show-help-overlay", &["<Control>question"]);
}
