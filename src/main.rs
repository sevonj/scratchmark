mod data;
mod error;
mod util;
mod widgets;

use gtk::prelude::*;
use gtk::{gio, glib};

use widgets::Window;

const APP_ID: &str = "org.scratchmark.Scratchmark";

fn main() -> glib::ExitCode {
    gio::resources_register_include!("gresources.gresource")
        .expect("Failed to register resources.");

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        let window = Window::new(app);
        window.set_title(Some("Scratchmark"));
        window.present();
    });

    app.run()
}
