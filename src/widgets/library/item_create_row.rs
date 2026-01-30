mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::EventControllerKey;
    use gtk::GestureClick;
    use gtk::gdk::Key;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;
    use gtk::Entry;
    use gtk::EventControllerFocus;
    use gtk::Image;
    use gtk::ListBoxRow;
    use gtk::TemplateChild;
    use gtk::glib::property::PropertySet;
    use gtk::glib::subclass::Signal;

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
        pub(super) filename: RefCell<Option<PathBuf>>,
        pub(super) is_dir: Cell<bool>,
        pub(super) committed: Cell<bool>,
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

            let focus_controller = EventControllerFocus::new();
            let click_controller = GestureClick::new();
            let esc_controller = EventControllerKey::new();

            focus_controller.connect_leave(clone!(
                #[weak (rename_to = imp)]
                self,
                move |_| {
                    imp.commit();
                }
            ));
            obj.add_controller(focus_controller);
            click_controller.connect_pressed(clone!(
                #[weak (rename_to = imp)]
                self,
                move |_, _, x, y| {
                    let widget = imp.obj().clone().upcast::<gtk::Widget>();
                    if !widget.contains(x, y) {
                        imp.commit();
                    }
                }
            ));
            esc_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
            esc_controller.connect_key_pressed(clone!(
                #[weak (rename_to = imp)]
                self,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_, key, _, _| {
                    if key == Key::Escape {
                        imp.cancel();
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                }
            ));

            obj.connect_map(clone!(move |obj| {
                let root = obj.root().unwrap();
                root.add_controller(click_controller.clone());
                root.add_controller(esc_controller.clone());
                obj.connect_destroy(clone!(
                    #[weak]
                    click_controller,
                    #[weak]
                    esc_controller,
                    move |_| {
                        root.remove_controller(&click_controller);
                        root.remove_controller(&esc_controller);
                    }
                ));

                glib::idle_add_local_once(clone!(
                    #[weak]
                    obj,
                    move || {
                        obj.imp().name_entry.grab_focus();
                    }
                ));
            }));
            obj.set_focusable(false);

            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("cancelled").build(),
                    Signal::builder("committed")
                        .param_types([String::static_type()])
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

            self.refresh();
        }

        fn refresh(&self) {
            if self.committed.get() {
                return;
            }

            let entry_text = self.name_entry.text();

            if entry_text.contains("/") {
                self.filename.set(None);
                self.name_entry.add_css_class("error");
                return;
            }

            let Ok((mut filename, _)) = glib::filename_from_utf8(entry_text) else {
                self.filename.set(None);
                self.name_entry.add_css_class("error");
                return;
            };

            if !self.is_dir.get() {
                filename.set_extension("md");
            }

            if self.parent_path.get().unwrap().join(&filename).exists() {
                self.filename.set(None);
                self.name_entry.add_css_class("error");
            } else {
                self.filename.set(Some(filename));
                self.name_entry.remove_css_class("error");
            }
        }

        fn commit(&self) {
            let Some(filename) = self.filename.borrow().clone() else {
                self.cancel();
                return;
            };

            if self.committed.replace(true) {
                return;
            }

            self.obj().emit_by_name("committed", &[&filename])
        }

        fn cancel(&self) {
            if self.committed.replace(true) {
                return;
            }

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
    pub fn for_document(parent: &Folder) -> Self {
        let obj: Self = Object::builder().build();
        obj.imp().bind(parent.path(), false, parent.depth() + 1);
        obj
    }

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
}
