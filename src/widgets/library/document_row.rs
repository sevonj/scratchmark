mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::closure_local;
    use gtk::ListBoxRow;
    use gtk::gdk;
    use gtk::gio;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::Builder;
    use gtk::CompositeTemplate;
    use gtk::DragSource;
    use gtk::FileLauncher;
    use gtk::Label;
    use gtk::PopoverMenu;
    use gtk::TemplateChild;
    use gtk::gdk::Rectangle;
    use gtk::gio::MenuModel;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib::Binding;

    use super::super::item_rename_popover::ItemRenamePopover;
    use crate::data::Document;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/document_row.ui")]
    pub struct DocumentRow {
        #[template_child]
        pub(super) open_in_editor_indicator: TemplateChild<Label>,
        #[template_child]
        pub(super) document_name_label: TemplateChild<Label>,
        #[template_child]
        pub(super) title_row: TemplateChild<gtk::Box>,

        pub(super) document: OnceLock<Document>,
        pub(super) bindings: RefCell<Vec<Binding>>,

        context_menu_popover: RefCell<Option<PopoverMenu>>,
        pub(super) rename_popover: RefCell<Option<ItemRenamePopover>>,
        pub(super) drag_source: RefCell<Option<DragSource>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DocumentRow {
        const NAME: &'static str = "DocumentRow";
        type Type = super::DocumentRow;
        type ParentType = ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DocumentRow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            self.setup_context_menu();
            self.setup_drag();

            let actions = SimpleActionGroup::new();
            obj.insert_action_group("document", Some(&actions));

            let action = gio::SimpleAction::new("filemanager", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    let file = gio::File::for_path(obj.path());
                    FileLauncher::new(Some(&file)).open_containing_folder(
                        None::<&adw::ApplicationWindow>,
                        None::<&gio::Cancellable>,
                        |_| {},
                    );
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("rename-begin", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| this.prompt_rename()
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("duplicate", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    this.document().duplicate();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("trash", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    this.document().trash();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("delete", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    this.document().delete();
                }
            ));
            actions.add_action(&action);
        }
    }

    impl WidgetImpl for DocumentRow {}
    impl ListBoxRowImpl for DocumentRow {}

    impl DocumentRow {
        pub(super) fn prompt_rename(&self) {
            self.rename_popover.borrow().as_ref().unwrap().popup();
        }

        pub(super) fn document(&self) -> &Document {
            self.document.get().unwrap()
        }

        fn setup_context_menu(&self) {
            let obj = self.obj();

            let builder = Builder::from_resource(
                "/org/scratchmark/Scratchmark/ui/library/document_context_menu.ui",
            );
            let popover = builder.object::<MenuModel>("context-menu").unwrap();
            let menu = PopoverMenu::builder()
                .menu_model(&popover)
                .has_arrow(false)
                .build();
            menu.set_parent(&*obj);
            let _ = self.context_menu_popover.replace(Some(menu));

            let gesture = gtk::GestureClick::new();
            gesture.set_button(gdk::ffi::GDK_BUTTON_SECONDARY as u32);
            gesture.connect_released(clone!(
                #[weak(rename_to = this)]
                self,
                move |gesture, _n, x, y| {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    if let Some(popover) = this.context_menu_popover.borrow().as_ref() {
                        popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
                        popover.popup();
                    };
                }
            ));
            obj.add_controller(gesture);

            obj.connect_destroy(move |obj| {
                if let Some(popover) = obj.imp().context_menu_popover.take() {
                    popover.unparent();
                }
            });
        }

        fn setup_rename_menu(&self) {
            let obj = self.obj();

            let rename_popover = ItemRenamePopover::for_document();
            rename_popover.set_parent(&*obj);
            rename_popover.set_path(self.document.get().unwrap().path());

            rename_popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_popover: ItemRenamePopover, path: PathBuf| {
                        if let Err(e) = this.document().rename(path) {
                            this.document().notify(&e.to_string())
                        }
                    }
                ),
            );

            let _ = self.rename_popover.replace(Some(rename_popover));

            obj.connect_destroy(move |obj| {
                if let Some(popover) = obj.imp().rename_popover.take() {
                    popover.unparent();
                }
            });
        }

        fn setup_drag(&self) {
            let obj = self.obj();

            let drag_source = DragSource::new();
            drag_source.set_actions(gdk::DragAction::COPY);
            drag_source.set_content(Some(&gdk::ContentProvider::for_value(&obj.to_value())));

            obj.add_controller(drag_source.clone());
            let _ = self.drag_source.replace(Some(drag_source));
        }

        pub(super) fn bind(&self, document: &Document) {
            self.document.get_or_init(|| document.clone());

            let open_in_editor_indicator: &Label = self.open_in_editor_indicator.as_ref();
            document
                .bind_property("is_open_in_editor", open_in_editor_indicator, "visible")
                .sync_create()
                .build();

            self.setup_rename_menu();

            self.title_row
                .set_margin_start(12 * document.depth() as i32);
            let title_label = self.document_name_label.get();
            let mut bindings = self.bindings.borrow_mut();

            let title_binding = document
                .bind_property("stem", &title_label, "label")
                .sync_create()
                .build();
            bindings.push(title_binding);
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::ListBoxRow;
use gtk::glib;
use gtk::prelude::*;

use glib::Object;

use crate::data::Document;

glib::wrapper! {
pub struct DocumentRow(ObjectSubclass<imp::DocumentRow>)
    @extends ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl DocumentRow {
    pub fn new(data: &Document) -> Self {
        let obj: Self = Object::builder().build();
        obj.imp().bind(data);
        obj
    }

    pub fn document(&self) -> &Document {
        self.imp().document()
    }

    pub fn path(&self) -> PathBuf {
        self.document().path()
    }

    pub fn stem(&self) -> String {
        self.document().stem()
    }

    pub fn on_click(&self) {
        self.document().open();
    }

    pub fn prompt_rename(&self) {
        self.imp().prompt_rename();
    }

    pub fn rename(&self, path: PathBuf) {
        if let Err(e) = self.document().rename(path) {
            self.document().notify(&e.to_string())
        }
    }
}
