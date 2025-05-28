//! Folder creation menu
//!

mod imp {

    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::Button;
    use gtk::Entry;
    use gtk::Label;
    use gtk::{CompositeTemplate, TemplateChild};

    use crate::util::FilenameStatus;
    use crate::util::path_builtin_library;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/new_folder_popover.ui")]
    pub struct NewFolderPopover {
        #[template_child]
        pub(super) name_field: TemplateChild<Entry>,
        #[template_child]
        pub(super) commit_button: TemplateChild<Button>,
        #[template_child]
        pub(super) name_error_label: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NewFolderPopover {
        const NAME: &'static str = "NewFolderPopover";
        type Type = super::NewFolderPopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NewFolderPopover {
        fn constructed(&self) {
            self.parent_constructed();
            let this = self;
            let obj = self.obj();

            obj.connect_closed(clone!(
                #[weak]
                this,
                move |_| {
                    this.clear();
                }
            ));

            self.name_field.connect_changed(clone!(
                #[weak]
                this,
                move |_| {
                    this.refresh();
                }
            ));

            self.commit_button.connect_clicked(clone!(
                #[weak]
                this,
                move |_| {
                    this.commit();
                }
            ));

            self.refresh();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("committed")
                        .param_types([PathBuf::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for NewFolderPopover {}
    impl PopoverImpl for NewFolderPopover {}

    impl NewFolderPopover {
        fn clear(&self) {
            self.name_field.set_text("");
        }

        fn refresh(&self) {
            let stem = self.name_field.text();
            let new_path = self.filepath();
            let file_exists = new_path.exists();

            let name_status = FilenameStatus::from(stem.as_str());
            self.commit_button
                .set_sensitive(name_status.is_ok() && !file_exists);

            let label = &self.name_error_label;

            match name_status {
                FilenameStatus::Ok => {
                    if file_exists {
                        label.set_text("Already exists");
                        label.set_visible(true);
                    } else {
                        label.set_visible(false);
                    }
                }
                _ => {
                    if let Some(msg) = name_status.complaint_message() {
                        label.set_visible(true);
                        label.set_text(msg);
                    } else {
                        label.set_visible(false);
                    }
                }
            }
        }

        fn commit(&self) {
            let filepath = self.filepath();
            self.obj().emit_by_name::<()>("committed", &[&filepath]);
            self.obj().popdown();
        }

        fn filepath(&self) -> PathBuf {
            let filename = self.name_field.text().to_string();
            path_builtin_library().join(&filename)
        }
    }
}

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct NewFolderPopover(ObjectSubclass<imp::NewFolderPopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native ,gtk::ShortcutManager;
}

impl Default for NewFolderPopover {
    fn default() -> Self {
        Object::builder().build()
    }
}
