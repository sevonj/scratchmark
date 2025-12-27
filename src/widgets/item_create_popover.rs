mod imp {

    use std::cell::Cell;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::Button;
    use gtk::CompositeTemplate;
    use gtk::Entry;
    use gtk::Label;
    use gtk::TemplateChild;
    use gtk::glib::Properties;

    use crate::util::FilenameStatus;

    #[derive(Debug, Default, Clone, Copy)]
    pub(super) enum Kind {
        #[default]
        Folder,
        Document,
    }

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::ItemCreatePopover)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/item_create_popover.ui")]
    pub struct ItemCreatePopover {
        #[template_child]
        pub(super) name_field: TemplateChild<Entry>,
        #[template_child]
        pub(super) commit_button: TemplateChild<Button>,
        #[template_child]
        pub(super) name_error_label: TemplateChild<Label>,

        pub(super) kind: Cell<Kind>,

        /// Selection in library - can be file or dir
        #[property(get, set)]
        selected_item_path: RefCell<PathBuf>,

        can_commit: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ItemCreatePopover {
        const NAME: &'static str = "ItemCreatePopover";
        type Type = super::ItemCreatePopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ItemCreatePopover {
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
            self.name_field.connect_activate(clone!(
                #[weak]
                this,
                move |_| {
                    this.commit();
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

    impl WidgetImpl for ItemCreatePopover {}
    impl PopoverImpl for ItemCreatePopover {}

    impl ItemCreatePopover {
        fn clear(&self) {
            self.name_field.set_text("");
        }

        fn refresh(&self) {
            let stem = self.name_field.text();
            let new_path = self.filepath();
            let file_exists = new_path.exists();

            let name_status = FilenameStatus::from(stem.as_str());
            self.can_commit.replace(name_status.is_ok() && !file_exists);
            self.commit_button.set_sensitive(self.can_commit.get());

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
            if !self.can_commit.get() {
                return;
            }
            let filepath = self.filepath();
            self.obj().emit_by_name::<()>("committed", &[&filepath]);
            self.obj().popdown();
        }

        fn filepath(&self) -> PathBuf {
            let filename = match self.kind.get() {
                Kind::Folder => self.name_field.text().to_string(),
                Kind::Document => self.name_field.text().to_string() + ".md",
            };

            let selected_path = self.obj().selected_item_path();
            let parent_path = if selected_path.is_dir() {
                // Is dir -- don't change path
                selected_path
            } else if selected_path.is_file() {
                // Is file -- use parent path
                selected_path.parent().unwrap().to_path_buf()
            } else {
                PathBuf::default()
            };
            parent_path.join(&filename)
        }
    }
}

use adw::subclass::prelude::*;
use gtk::glib;

use glib::Object;

glib::wrapper! {
    pub struct ItemCreatePopover(ObjectSubclass<imp::ItemCreatePopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native ,gtk::ShortcutManager;
}

impl ItemCreatePopover {
    pub fn for_folder() -> Self {
        let this: Self = Object::builder().build();
        this.imp().kind.replace(imp::Kind::Folder);
        this
    }

    pub fn for_document() -> Self {
        let this: Self = Object::builder().build();
        this.imp().kind.replace(imp::Kind::Document);
        this
    }
}
