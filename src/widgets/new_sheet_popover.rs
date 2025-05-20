//! Sheet creation menu
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

    use crate::util::path_builtin_library;

    enum FilenameStatus {
        Ok,
        AlreadyExists,
        IsEmpty,
        HasIllegalChars,
    }

    impl FilenameStatus {
        pub fn is_ok(&self) -> bool {
            match self {
                Self::Ok => true,
                Self::AlreadyExists => false,
                Self::IsEmpty => false,
                Self::HasIllegalChars => false,
            }
        }
    }

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/new_sheet_popover.ui")]
    pub struct NewSheetPopover {
        #[template_child]
        pub(super) name_field: TemplateChild<Entry>,
        #[template_child]
        pub(super) commit_button: TemplateChild<Button>,
        #[template_child]
        pub(super) name_error_label: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NewSheetPopover {
        const NAME: &'static str = "NewSheetPopover";
        type Type = super::NewSheetPopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NewSheetPopover {
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

    impl WidgetImpl for NewSheetPopover {}
    impl PopoverImpl for NewSheetPopover {}

    impl NewSheetPopover {
        fn clear(&self) {
            self.name_field.set_text("");
        }

        fn refresh(&self) {
            let name_status = self.filename_status();
            self.commit_button.set_sensitive(name_status.is_ok());

            let label = &self.name_error_label;

            match name_status {
                FilenameStatus::Ok => label.set_visible(false),
                FilenameStatus::AlreadyExists => {
                    label.set_text("File exists");
                    label.set_visible(true);
                }
                FilenameStatus::IsEmpty => label.set_visible(false),
                FilenameStatus::HasIllegalChars => {
                    label.set_text("Invalid filename");
                    label.set_visible(true);
                }
            }
        }

        fn filename_status(&self) -> FilenameStatus {
            let text = self.name_field.text();

            if text.is_empty() {
                return FilenameStatus::IsEmpty;
            }

            if self.filepath().exists() {
                return FilenameStatus::AlreadyExists;
            }

            if text.contains("/") {
                return FilenameStatus::HasIllegalChars;
            }

            FilenameStatus::Ok
        }

        fn commit(&self) {
            let filepath = self.filepath();
            self.obj().emit_by_name::<()>("committed", &[&filepath]);
            self.obj().popdown();
        }

        fn filepath(&self) -> PathBuf {
            let filename = self.name_field.text().to_string() + ".md";
            path_builtin_library().join(&filename)
        }
    }
}

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct NewSheetPopover(ObjectSubclass<imp::NewSheetPopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native ,gtk::ShortcutManager;
}

impl Default for NewSheetPopover {
    fn default() -> Self {
        Object::builder().build()
    }
}
