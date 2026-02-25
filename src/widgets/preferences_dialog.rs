mod imp {
    use std::cell::OnceCell;

    use adw::ActionRow;
    use adw::SpinRow;
    use adw::SwitchRow;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::FontDialog;
    use gtk::gio::Cancellable;
    use gtk::gio::Settings;
    use gtk::glib;
    use gtk::glib::clone;

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
            editor_max_width_spin
                .adjustment()
                .set_upper(crate::settings::EDITOR_WIDTH_LIMIT_MAX as f64);
            editor_max_width_spin
                .adjustment()
                .set_lower(crate::settings::EDITOR_WIDTH_LIMIT_MIN as f64);
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

            let context = obj.pango_context();
            let family_name = settings.string("editor-font-family");
            let family = context
                .list_families()
                .into_iter()
                .find(|f| f.name() == family_name);

            FontDialog::builder().modal(true).build().choose_family(
                obj.root().and_downcast_ref::<gtk::Window>(),
                family.as_ref(),
                None::<&Cancellable>,
                clone!(
                    #[weak]
                    settings,
                    move |result| {
                        let Ok(family) = result else {
                            return;
                        };
                        settings
                            .set_string("editor-font-family", &family.to_string())
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
use gtk::gio::Settings;
use gtk::glib;
use gtk::glib::Object;

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
