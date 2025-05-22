//! Sheet rename menu
//!

mod imp {

    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::GString;
    use glib::clone;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::Button;
    use gtk::Entry;
    use gtk::Label;
    use gtk::{CompositeTemplate, TemplateChild};

    use crate::util::FilenameStatus;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/sheet_rename_popover.ui")]
    pub struct SheetRenamePopover {
        #[template_child]
        name_field: TemplateChild<Entry>,
        #[template_child]
        commit_button: TemplateChild<Button>,
        #[template_child]
        name_error_label: TemplateChild<Label>,

        pub(super) original_path: RefCell<PathBuf>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SheetRenamePopover {
        const NAME: &'static str = "SheetRenamePopover";
        type Type = super::SheetRenamePopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SheetRenamePopover {
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

    impl WidgetImpl for SheetRenamePopover {}
    impl PopoverImpl for SheetRenamePopover {}

    impl SheetRenamePopover {
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
            let name = stem.to_string() + ".md";
            let new_path = self.parent_path().join(&name);
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
            self.obj().emit_by_name::<()>("committed", &[&filepath]);
            self.obj().popdown();
        }

        fn parent_path(&self) -> PathBuf {
            let original_path = self.original_path.borrow();
            let parent_path = original_path.parent().expect("Failed to get path parent.");
            parent_path.to_path_buf()
        }

        fn filepath(&self) -> PathBuf {
            let filename = self.name_field.text().to_string() + ".md";
            self.parent_path().join(&filename)
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;

use glib::Object;

glib::wrapper! {
    pub struct SheetRenamePopover(ObjectSubclass<imp::SheetRenamePopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native ,gtk::ShortcutManager;
}

impl Default for SheetRenamePopover {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl SheetRenamePopover {
    pub fn set_path(&self, path: PathBuf) {
        let _ = self.imp().original_path.replace(path);
        self.imp().reset();
    }
}
