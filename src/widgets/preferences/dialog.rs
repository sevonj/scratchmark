mod imp {
    use std::cell::OnceCell;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::OnceLock;

    use adw::ActionRow;
    use adw::SpinRow;
    use adw::SwitchRow;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::FlowBox;
    use gtk::FontDialog;
    use gtk::MenuButton;
    use gtk::gio::Cancellable;
    use gtk::gio::Settings;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::glib::closure_local;

    use crate::util;
    use crate::widgets::PreferencesFileExtItem;
    use crate::widgets::preferences::file_ext_add_popover::FileExtAddPopover;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/preferences/dialog.ui")]
    pub struct PreferencesDialog {
        settings: OnceCell<Settings>,

        #[template_child]
        editor_font_button: TemplateChild<ActionRow>,
        #[template_child]
        editor_font_reset_button: TemplateChild<ActionRow>,
        #[template_child]
        editor_minimap_toggle: TemplateChild<SwitchRow>,
        #[template_child]
        editor_tabs_as_spaces_toggle: TemplateChild<SwitchRow>,
        #[template_child]
        editor_limit_width_toggle: TemplateChild<SwitchRow>,
        #[template_child]
        editor_max_width_spin: TemplateChild<SpinRow>,
        #[template_child]
        editor_spellcheck_toggle: TemplateChild<SwitchRow>,

        #[template_child]
        library_ignore_hidden_files_toggle: TemplateChild<SwitchRow>,
        #[template_child]
        library_show_file_extensions_toggle: TemplateChild<SwitchRow>,
        #[template_child]
        library_extensions_flowbox: TemplateChild<FlowBox>,
        #[template_child]
        library_add_ext_menubutton: TemplateChild<MenuButton>,
        library_ext_items: RefCell<HashMap<String, PreferencesFileExtItem>>,
        library_file_ext_add_popover: OnceLock<FileExtAddPopover>,
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
            let obj: glib::BorrowedObject<'_, super::PreferencesDialog> = self.obj();
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
            let editor_tabs_as_spaces_toggle: &SwitchRow = &self.editor_tabs_as_spaces_toggle;
            settings
                .bind(
                    "editor-tabs-as-spaces",
                    editor_tabs_as_spaces_toggle,
                    "active",
                )
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
            let editor_spellcheck_toggle: &SwitchRow = &self.editor_spellcheck_toggle;
            settings
                .bind("editor-use-spellcheck", editor_spellcheck_toggle, "active")
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
            let library_show_file_extensions_toggle: &SwitchRow =
                &self.library_show_file_extensions_toggle;
            settings
                .bind(
                    "library-show-file-extensions",
                    library_show_file_extensions_toggle,
                    "active",
                )
                .build();

            self.library_extensions_flowbox.set_sort_func(|a, b| {
                let a = a
                    .child()
                    .and_then(|c| c.downcast::<PreferencesFileExtItem>().ok());
                let b = b
                    .child()
                    .and_then(|c| c.downcast::<PreferencesFileExtItem>().ok());

                let Some(a) = a else {
                    if b.is_some() {
                        return gtk::Ordering::Larger;
                    } else {
                        return gtk::Ordering::Equal;
                    }
                };

                let Some(b) = b else {
                    return gtk::Ordering::Smaller;
                };

                a.collation_key().cmp(b.collation_key()).into()
            });

            let library_custom_file_extensions = settings.strv("library-custom-file-extensions");
            for ext in library_custom_file_extensions {
                self.add_custom_file_ext_no_save(ext.into());
            }

            let popover = FileExtAddPopover::default();
            self.library_add_ext_menubutton.set_popover(Some(&popover));
            popover.connect_closure(
                "changed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |popover: FileExtAddPopover, ext: String| {
                        popover.set_extension_ok(imp.can_add_ext(&ext));
                    }
                ),
            );
            popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: FileExtAddPopover, ext: String| imp.add_custom_file_ext(ext)
                ),
            );
            self.library_file_ext_add_popover.set(popover).unwrap();
            obj.connect_destroy(move |obj| {
                obj.imp()
                    .library_file_ext_add_popover
                    .get()
                    .unwrap()
                    .unparent();
            });
        }

        fn has_file_ext_already(&self, ext: &str) -> bool {
            let ext = util::process_file_ext_text(ext);
            ext == "md" || self.library_ext_items.borrow().contains_key(&ext)
        }

        fn can_add_ext(&self, ext: &str) -> bool {
            !self.has_file_ext_already(ext) && !ext.is_empty()
        }

        fn add_custom_file_ext(&self, ext: String) {
            self.add_custom_file_ext_no_save(ext);

            let binding = self.library_ext_items.borrow();
            let extensions: Vec<&String> = binding.keys().collect();
            self.settings
                .get()
                .unwrap()
                .set_strv("library-custom-file-extensions", extensions)
                .unwrap();
        }

        fn add_custom_file_ext_no_save(&self, ext: String) {
            let obj = self.obj();
            if !self.can_add_ext(&ext) {
                return;
            }
            let item = PreferencesFileExtItem::new(ext.clone());
            item.connect_closure(
                "remove",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |item: PreferencesFileExtItem| {
                        obj.imp().remove_custom_file_ext(item.ext());
                    }
                ),
            );
            self.library_extensions_flowbox.append(&item);
            self.library_ext_items.borrow_mut().insert(ext, item);
        }

        fn remove_custom_file_ext(&self, ext: &str) {
            let Some(item) = self.library_ext_items.borrow_mut().remove(ext) else {
                return;
            };
            self.library_extensions_flowbox.remove(&item);
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
