mod imp {
    use std::cell::Cell;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::Button;
    use gtk::CompositeTemplate;
    use gtk::Entry;
    use gtk::TemplateChild;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::glib::clone;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;

    use crate::util;

    #[derive(Properties, CompositeTemplate, Default)]
    #[properties(wrapper_type = super::FileExtAddPopover)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/preferences/file_ext_add_popover.ui")]
    pub struct FileExtAddPopover {
        #[property(set, get)]
        extension_ok: Cell<bool>,

        #[template_child]
        name_entry: TemplateChild<Entry>,
        #[template_child]
        commit_button: TemplateChild<Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileExtAddPopover {
        const NAME: &'static str = "FileExtAddPopover";
        type Type = super::FileExtAddPopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FileExtAddPopover {
        fn constructed(&self) {
            let obj = self.obj();

            self.name_entry.connect_changed(clone!(
                #[weak]
                obj,
                move |entry| {
                    let entry_text = entry.text();
                    obj.emit_by_name::<()>("changed", &[&entry_text]);
                }
            ));

            self.name_entry.connect_activate(clone!(
                #[weak (rename_to = imp)]
                self,
                move |_| {
                    imp.commit();
                }
            ));

            self.commit_button.connect_clicked(clone!(
                #[weak (rename_to = imp)]
                self,
                move |_| {
                    imp.commit();
                }
            ));

            obj.connect_closed(move |obj| {
                obj.imp().name_entry.set_text("");
            });

            let commit_button: &Button = self.commit_button.as_ref();
            obj.bind_property("extension_ok", commit_button, "sensitive")
                .sync_create()
                .build();

            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("committed")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("changed")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for FileExtAddPopover {}
    impl PopoverImpl for FileExtAddPopover {}

    impl FileExtAddPopover {
        fn commit(&self) {
            if !self.extension_ok.get() {
                return;
            }
            let entry_text = util::process_file_ext_text(&self.name_entry.text());
            self.obj().emit_by_name::<()>("committed", &[&entry_text]);
            self.obj().popdown();
        }
    }
}

use gtk::glib;
use gtk::glib::Object;

glib::wrapper! {
    pub struct FileExtAddPopover(ObjectSubclass<imp::FileExtAddPopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native ,gtk::ShortcutManager;
}

impl Default for FileExtAddPopover {
    fn default() -> Self {
        Object::builder().build()
    }
}
