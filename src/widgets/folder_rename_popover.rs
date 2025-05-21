//! Sheet rename menu
//!

mod imp {

    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::prelude::*;

    use glib::GString;
    use gtk::Button;
    use gtk::Entry;
    use gtk::Label;
    use gtk::{CompositeTemplate, TemplateChild};

    use crate::util::FilenameStatus;
    use crate::util::path_builtin_library;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/folder_rename_popover.ui")]
    pub struct FolderRenamePopover {
        #[template_child]
        name_field: TemplateChild<Entry>,
        #[template_child]
        commit_button: TemplateChild<Button>,
        #[template_child]
        name_error_label: TemplateChild<Label>,

        pub(super) original_path: RefCell<PathBuf>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FolderRenamePopover {
        const NAME: &'static str = "FolderRenamePopover";
        type Type = super::FolderRenamePopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FolderRenamePopover {
        fn constructed(&self) {
            self.parent_constructed();
            let this = self;
            let obj = self.obj();

            obj.connect_closed(clone!(
                #[weak]
                this,
                move |_| {
                    this.reset();
                }
            ));

            self.name_field.connect_changed(clone!(
                #[weak]
                this,
                move |name_field| {
                    this.refresh(name_field.text());
                }
            ));

            self.commit_button.connect_clicked(clone!(
                #[weak]
                this,
                move |_| {
                    this.commit();
                }
            ));
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

    impl WidgetImpl for FolderRenamePopover {}
    impl PopoverImpl for FolderRenamePopover {}

    impl FolderRenamePopover {
        pub(super) fn reset(&self) {
            let stem = self
                .original_path
                .borrow()
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            self.name_field.set_text(&stem);
        }

        fn refresh(&self, stem: GString) {
            let original_path = self.original_path.borrow();
            let parent_path = original_path.parent().expect("Failed to get path parent.");
            let name = stem.to_string();
            let new_path = parent_path.join(&name);
            let file_exists = new_path.exists();

            let name_status = FilenameStatus::from(stem.as_str());
            self.commit_button
                .set_sensitive(name_status.is_ok() && !file_exists);

            let label = &self.name_error_label;

            match name_status {
                FilenameStatus::Ok => {
                    if file_exists && new_path != *original_path {
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
            let original_path = self.original_path.borrow();
            fs::rename(&*original_path, &filepath).expect("Folder rename failed");
            self.obj().emit_by_name::<()>("committed", &[&filepath]);
            self.obj().popdown();
        }

        fn filepath(&self) -> PathBuf {
            let filename = self.name_field.text().to_string();
            path_builtin_library().join(&filename)
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;

use glib::Object;

glib::wrapper! {
    pub struct FolderRenamePopover(ObjectSubclass<imp::FolderRenamePopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native ,gtk::ShortcutManager;
}

impl Default for FolderRenamePopover {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl FolderRenamePopover {
    pub fn set_path(&self, path: PathBuf) {
        let _ = self.imp().original_path.replace(path);
        self.imp().reset();
    }
}
