mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::Entry;
    use gtk::EventControllerFocus;
    use gtk::Image;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::glib::subclass::Signal;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;
    use gtk::ListBoxRow;
    use gtk::TemplateChild;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/item_create_row.ui")]
    pub struct ItemCreateRow {
        #[template_child]
        pub(super) title_row: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) name_entry: TemplateChild<Entry>,
        #[template_child]
        pub(super) item_icon: TemplateChild<Image>,

        pub(super) parent_path: OnceLock<PathBuf>,
        pub(super) path: RefCell<PathBuf>,
        pub(super) is_dir: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ItemCreateRow {
        const NAME: &'static str = "ItemCreateRow";
        type Type = super::ItemCreateRow;
        type ParentType = ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ItemCreateRow {
        fn constructed(&self) {
            let obj = self.obj();

            self.name_entry.connect_changed(clone!(
                #[weak (rename_to = imp)]
                self,
                move |_| {
                    imp.validate_entry();
                }
            ));

            self.name_entry.connect_activate(clone!(
                #[weak (rename_to = imp)]
                self,
                move |_| {
                    println!("activated");
                    imp.commit();
                }
            ));

            let focus_controller = EventControllerFocus::new();
            focus_controller.connect_leave(clone!(
                #[weak (rename_to = imp)]
                self,
                move |_| {
                    println!("left focus");
                    imp.commit();
                }
            ));
            self.name_entry.add_controller(focus_controller);

            obj.connect_map(move |obj| {
                obj.imp().name_entry.grab_focus();
            });

            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("cancelled").build(),
                    Signal::builder("committed-document")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("committed-folder")
                        .param_types([PathBuf::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for ItemCreateRow {}
    impl ListBoxRowImpl for ItemCreateRow {}

    impl ItemCreateRow {
        pub(super) fn bind(&self, parent_path: PathBuf, is_dir: bool, depth: u32) {
            self.parent_path.set(parent_path).unwrap();
            self.is_dir.replace(is_dir);

            self.title_row
                .set_margin_start(std::cmp::max(20 + 12 * depth as i32, 0));
            if is_dir {
                self.item_icon.set_icon_name(Some("folder-symbolic"));
            }

            self.validate_entry();
        }

        fn validate_entry(&self) {
            let new_path = if self.is_dir.get() {
                self.parent_path.get().unwrap().join(self.name_entry.text())
            } else {
                self.parent_path
                    .get()
                    .unwrap()
                    .join(self.name_entry.text())
                    .with_added_extension("md")
            };

            if new_path.exists() {
                self.name_entry.add_css_class("error");
            } else {
                self.name_entry.remove_css_class("error");
            }

            self.path.replace(new_path);
        }

        fn commit(&self) {
            let path = self.path.borrow().clone();
            if path.exists() || self.name_entry.text().is_empty() {
                self.cancel();
                return;
            }
            if self.is_dir.get() {
                self.obj().emit_by_name("committed-folder", &[&path])
            } else {
                self.obj().emit_by_name("committed-document", &[&path])
            }
            println!("committed");
        }

        fn cancel(&self) {
            println!("cancelled");
            self.obj().emit_by_name("cancelled", &[])
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::ListBoxRow;
use gtk::glib;

use glib::Object;

use crate::data::Folder;

glib::wrapper! {
pub struct ItemCreateRow(ObjectSubclass<imp::ItemCreateRow>)
    @extends ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl ItemCreateRow {
    pub fn for_folder(parent: &Folder) -> Self {
        let obj: Self = Object::builder().build();
        obj.imp().bind(parent.path(), true, parent.depth() + 1);
        obj
    }

    pub fn is_dir(&self) -> bool {
        self.imp().is_dir.get()
    }

    pub fn parent_path(&self) -> PathBuf {
        self.imp().parent_path.get().unwrap().clone()
    }

    pub fn path(&self) -> PathBuf {
        self.imp().path.borrow().clone()
    }
}
