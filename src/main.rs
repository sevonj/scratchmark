mod config;
mod data;
mod error;
mod util;
mod widgets;

use gtk::glib;
use gtk::prelude::*;

use widgets::Window;

const APP_ID: &str = "org.scratchmark.Scratchmark";

fn main() -> glib::ExitCode {
    util::create_builtin_library();

    gettextrs::bindtextdomain(config::GETTEXT_PACKAGE, config::LOCALEDIR)
        .expect("Unable to bind the text domain");
    gettextrs::bind_textdomain_codeset(config::GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    gettextrs::textdomain(config::GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    #[cfg(feature = "installed")]
    {
        let resources =
            gtk::gio::Resource::load(config::PKGDATADIR.to_owned() + "/scratchmark.gresource")
                .expect("Could not load resources");
        gtk::gio::resources_register(&resources);
    }

    #[cfg(not(feature = "installed"))]
    {
        // Running from repository
        gtk::gio::resources_register_include!("gresources.gresource")
            .expect("Failed to register resources.");
    }

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
    app.set_accels_for_action("win.file-new", &["<Ctrl>N"]);
    app.set_accels_for_action("win.project-add", &["<Ctrl><Shift>O"]);
    app.set_accels_for_action("win.file-save", &["<Ctrl>S"]);
    app.set_accels_for_action("win.file-rename-open", &["F2"]);
    app.set_accels_for_action("win.file-close", &["<Ctrl>W"]);
    app.set_accels_for_action("editor.format-bold", &["<Ctrl>B"]);
    app.set_accels_for_action("editor.format-italic", &["<Ctrl>I"]);
    app.set_accels_for_action("editor.show-search", &["<Ctrl>F"]);
    app.set_accels_for_action("editor.show-search-replace", &["<Ctrl>R"]);
    app.set_accels_for_action("editor.hide-search", &["Escape"]);
    app.set_accels_for_action("editor.shiftreturn", &["<Shift>Return"]);
    app.set_accels_for_action("win.library-refresh", &["F5"]);

    app.set_accels_for_action("win.toggle-sidebar", &["F9"]);
    app.set_accels_for_action("win.toggle-fullscreen", &["F11"]);
    app.set_accels_for_action("win.toggle-focus", &["F8"]);
    app.set_accels_for_action("win.show-help-overlay", &["<Control>question"]);
    app.set_accels_for_action("win.preferences", &["<ctrl>comma"]);
}

#[cfg(test)]
mod tests {
    use crate::config;

    #[test]
    fn test_meson_cargo_equal_version() {
        // Top level meson.build and Cargo.toml
        assert_eq!(config::VERSION, env!("CARGO_PKG_VERSION"));
    }
}
