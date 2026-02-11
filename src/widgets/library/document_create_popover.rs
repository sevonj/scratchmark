mod imp {
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
    use gtk::TemplateChild;
    use gtk::glib::property::PropertySet;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/document_create_popover.ui")]
    pub struct DocumentCreatePopover {
        #[template_child]
        name_entry: TemplateChild<Entry>,
        #[template_child]
        commit_button: TemplateChild<Button>,

        pub(super) parent_path: OnceLock<PathBuf>,
        pub(super) filename: RefCell<Option<PathBuf>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DocumentCreatePopover {
        const NAME: &'static str = "DocumentCreatePopover";
        type Type = super::DocumentCreatePopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DocumentCreatePopover {
        fn constructed(&self) {
            self.name_entry.connect_changed(clone!(
                #[weak (rename_to = imp)]
                self,
                move |_| {
                    imp.refresh();
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

            self.parent_constructed();
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

    impl WidgetImpl for DocumentCreatePopover {}
    impl PopoverImpl for DocumentCreatePopover {}

    impl DocumentCreatePopover {
        fn refresh(&self) {
            let entry_text = self.name_entry.text();

            if entry_text.contains("/") {
                self.filename.set(None);
                self.name_entry.add_css_class("error");
                return;
            }

            let Ok(filename) = glib::filename_from_utf8(entry_text)
                .map(|(filename, _)| filename.with_extension("md"))
            else {
                self.filename.set(None);
                self.name_entry.add_css_class("error");
                return;
            };

            if self.parent_path.get().unwrap().join(&filename).exists() {
                self.filename.set(None);
                self.name_entry.add_css_class("error");
                self.commit_button.set_sensitive(false);
            } else {
                self.filename.set(Some(filename));
                self.name_entry.remove_css_class("error");
                self.commit_button.set_sensitive(true);
            }
        }

        fn commit(&self) {
            let Some(filename) = self.filename.borrow().clone() else {
                self.cancel();
                return;
            };
            self.obj().emit_by_name::<()>("committed", &[&filename]);
            self.obj().popdown();
        }

        fn cancel(&self) {
            self.obj().emit_by_name::<()>("cancelled", &[]);
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;

use glib::Object;

glib::wrapper! {
    pub struct DocumentCreatePopover(ObjectSubclass<imp::DocumentCreatePopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native ,gtk::ShortcutManager;
}

impl DocumentCreatePopover {
    pub fn new(parent_path: PathBuf) -> Self {
        let obj: DocumentCreatePopover = Object::builder().build();
        obj.imp().parent_path.set(parent_path).unwrap();
        obj
    }
}
