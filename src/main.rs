mod config;
mod data;
mod error;
mod util;
mod widgets;

use gtk::glib;
use gtk::prelude::*;

use gtk::glib::GString;
use sourceview5::LanguageManager;
use sourceview5::StyleSchemeManager;

use config::PKGDATADIR;
use util::file_actions;
use widgets::Window;

const APP_ID: &str = "org.scratchmark.Scratchmark";

fn main() -> glib::ExitCode {
    file_actions::create_builtin_library();

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
        setup_buffer_styles();
        setup_language_manager();

        let window = Window::new(app);
        window.set_title(Some("Scratchmark"));
        window.present();
    });

    app.run()
}

fn setup_accels(app: &adw::Application) {
    app.set_accels_for_action("win.file-new", &["<Ctrl>N"]);
    app.set_accels_for_action("win.folder-new", &["<Shift><Ctrl>N"]);
    app.set_accels_for_action("win.project-add", &["<Ctrl><Shift>O"]);
    app.set_accels_for_action("win.file-save", &["<Ctrl>S"]);
    app.set_accels_for_action("win.file-rename-selected", &["F2"]);
    app.set_accels_for_action("win.file-close", &["<Ctrl>W"]);
    app.set_accels_for_action("editor.format-bold", &["<Ctrl>B"]);
    app.set_accels_for_action("editor.format-italic", &["<Ctrl>I"]);
    app.set_accels_for_action("editor.format-h1", &["<Ctrl>1"]);
    app.set_accels_for_action("editor.format-h2", &["<Ctrl>2"]);
    app.set_accels_for_action("editor.format-h3", &["<Ctrl>3"]);
    app.set_accels_for_action("editor.format-h4", &["<Ctrl>4"]);
    app.set_accels_for_action("editor.format-h5", &["<Ctrl>5"]);
    app.set_accels_for_action("editor.format-h6", &["<Ctrl>6"]);
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

fn setup_buffer_styles() {
    StyleSchemeManager::default().append_search_path(&format!("{PKGDATADIR}/editor_schemes"));
    StyleSchemeManager::default().append_search_path(&format!("{PKGDATADIR}/document_preview"));

    #[cfg(not(feature = "installed"))]
    {
        const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
        StyleSchemeManager::default()
            .append_search_path(format!("{MANIFEST_DIR}/data/editor_schemes").as_str());
        StyleSchemeManager::default()
            .append_search_path(format!("{MANIFEST_DIR}/data/document_preview").as_str());
    }
}

fn setup_language_manager() {
    let lm = LanguageManager::default();
    let mut search_path = lm.search_path();

    #[cfg(feature = "installed")]
    {
        let lang_spec_dir = &format!("{PKGDATADIR}/language_specs");
        search_path.insert(0, lang_spec_dir.into());
    }
    #[cfg(not(feature = "installed"))]
    {
        const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
        let lang_spec_dir = format!("{MANIFEST_DIR}/data/language_specs");
        search_path.insert(0, lang_spec_dir.into());
    }

    let dirs: Vec<&str> = search_path.iter().map(GString::as_str).collect();
    lm.set_search_path(&dirs);
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
