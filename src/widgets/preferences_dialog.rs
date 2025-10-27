mod imp {
    use std::cell::OnceCell;
    use std::sync::OnceLock;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::glib::subclass::Signal;
    use gtk::pango;

    use adw::ActionRow;
    use adw::SwitchRow;
    use gtk::CompositeTemplate;
    use gtk::FontDialog;
    use gtk::gio::Cancellable;
    use gtk::gio::Settings;
    use gtk::gio::SettingsBindFlags;
    use gtk::pango::FontDescription;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/preferences_dialog.ui")]
    pub struct PreferencesDialog {
        settings: OnceCell<Settings>,

        #[template_child]
        editor_font_button: TemplateChild<ActionRow>,

        #[template_child]
        editor_minimap_toggle: TemplateChild<SwitchRow>,

        #[template_child]
        general_autosave_toggle: TemplateChild<SwitchRow>,

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

    impl ObjectImpl for PreferencesDialog {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("font-changed")
                        .param_types([FontDescription::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for PreferencesDialog {}
    impl AdwDialogImpl for PreferencesDialog {}
    impl PreferencesDialogImpl for PreferencesDialog {}

    impl PreferencesDialog {
        pub(super) fn bind_settings(&self, settings: Settings) {
            self.settings
                .set(settings)
                .expect("Settings set multiple times");
            let settings = self.settings.get().unwrap();

            self.editor_font_button.connect_activated(clone!(
                #[weak(rename_to = this)]
                self,
                move |_| this.show_font_dialog()
            ));

            let editor_minimap_toggle: &SwitchRow = self.editor_minimap_toggle.as_ref();
            settings
                .bind("editor-show-minimap", editor_minimap_toggle, "active")
                .flags(SettingsBindFlags::DEFAULT)
                .build();

            let general_autosave_toggle: &SwitchRow = self.general_autosave_toggle.as_ref();
            settings
                .bind("autosave", general_autosave_toggle, "active")
                .flags(SettingsBindFlags::DEFAULT)
                .build();

            let library_ignore_hidden_files_toggle: &SwitchRow =
                self.library_ignore_hidden_files_toggle.as_ref();
            settings
                .bind(
                    "library-ignore-hidden-files",
                    library_ignore_hidden_files_toggle,
                    "active",
                )
                .flags(SettingsBindFlags::DEFAULT)
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
                    obj,
                    move |result| {
                        let Ok(font) = result else {
                            return;
                        };
                        obj.emit_by_name("font-changed", &[&font])
                    }
                ),
            );
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
        let this: PreferencesDialog = Object::builder().build();
        this.imp().bind_settings(settings);
        this
    }
}
