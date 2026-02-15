mod imp {
    use std::cell::OnceCell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::pango;

    use adw::ActionRow;
    use adw::SpinRow;
    use adw::SwitchRow;
    use gtk::CompositeTemplate;
    use gtk::FontDialog;
    use gtk::gio::Cancellable;
    use gtk::gio::Settings;
    use gtk::pango::FontDescription;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/preferences_dialog.ui")]
    pub struct PreferencesDialog {
        settings: OnceCell<Settings>,

        #[template_child]
        editor_font_button: TemplateChild<ActionRow>,
        #[template_child]
        editor_font_reset_button: TemplateChild<ActionRow>,
        #[template_child]
        editor_minimap_toggle: TemplateChild<SwitchRow>,
        #[template_child]
        editor_limit_width_toggle: TemplateChild<SwitchRow>,
        #[template_child]
        editor_max_width_spin: TemplateChild<SpinRow>,

        #[template_child]
        library_ignore_hidden_files_toggle: TemplateChild<SwitchRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesDialog {
        const NAME: &'static str = "PreferencesDialog";
        type Type = super::PreferencesDialog;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesDialog {}

    impl WidgetImpl for PreferencesDialog {}
    impl AdwDialogImpl for PreferencesDialog {}
    impl PreferencesDialogImpl for PreferencesDialog {}

    impl PreferencesDialog {
        pub(super) fn bind_settings(&self, settings: Settings) {
            self.settings.set(settings.clone()).unwrap();

            self.editor_font_button.connect_activated(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| imp.show_font_dialog()
            ));

            self.editor_font_reset_button.connect_activated(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| imp.reset_font()
            ));

            let editor_minimap_toggle: &SwitchRow = &self.editor_minimap_toggle;
            settings
                .bind("editor-show-minimap", editor_minimap_toggle, "active")
                .build();
            let editor_limit_width_toggle: &SwitchRow = &self.editor_limit_width_toggle;
            settings
                .bind("editor-limit-width", editor_limit_width_toggle, "active")
                .build();
            let editor_max_width_spin: &SpinRow = &self.editor_max_width_spin;
            settings
                .bind("editor-max-width", editor_max_width_spin, "value")
                .build();
            settings
                .bind("editor-limit-width", editor_max_width_spin, "sensitive")
                .get()
                .build();

            let library_ignore_hidden_files_toggle: &SwitchRow =
                &self.library_ignore_hidden_files_toggle;
            settings
                .bind(
                    "library-ignore-hidden-files",
                    library_ignore_hidden_files_toggle,
                    "active",
                )
                .build();
        }

        fn show_font_dialog(&self) {
            let obj = self.obj();
            let settings = self.settings.get().expect("settings not set!");

            let font_family = settings.string("editor-font-family");
            let font_size = settings.uint("editor-font-size");
            let mut initial = FontDescription::new();
            initial.set_family(&font_family);
            initial.set_size(font_size as i32 * pango::SCALE);

            FontDialog::builder().modal(true).build().choose_font(
                obj.root().and_downcast_ref::<gtk::Window>(),
                Some(&initial),
                None::<&Cancellable>,
                clone!(
                    #[weak]
                    settings,
                    move |result| {
                        let Ok(font) = result else {
                            return;
                        };
                        settings
                            .set_uint("editor-font-size", (font.size() / pango::SCALE) as u32)
                            .unwrap();
                        settings
                            .set_string("editor-font-family", &font.family().unwrap_or_default())
                            .unwrap();
                    }
                ),
            );
        }

        fn reset_font(&self) {
            let settings = self.settings.get().unwrap();
            settings.reset("editor-font-family");
            settings.reset("editor-font-size");
        }
    }
}

use adw::subclass::prelude::*;
use gtk::glib;

use glib::Object;
use gtk::gio::Settings;

glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
       @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog,
       @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl PreferencesDialog {
    pub fn new(settings: Settings) -> Self {
        let obj: PreferencesDialog = Object::builder().build();
        obj.imp().bind_settings(settings);
        obj
    }
}
