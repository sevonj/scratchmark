mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use glib::closure_local;
    use gtk::gdk;
    use gtk::gio;
    use gtk::glib;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;

    use gtk::Builder;
    use gtk::CompositeTemplate;
    use gtk::DragSource;
    use gtk::DropTarget;
    use gtk::FileLauncher;
    use gtk::Label;
    use gtk::ListBoxRow;
    use gtk::PopoverMenu;
    use gtk::TemplateChild;
    use gtk::gdk::DragAction;
    use gtk::gdk::Rectangle;
    use gtk::gio::MenuModel;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib::Binding;

    use crate::data::Document;
    use crate::widgets::library::folder_row::FolderRow;
    use crate::widgets::library::item_rename_popover::ItemRenamePopover;

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

        context_menu_popover: OnceLock<PopoverMenu>,
        pub(super) rename_popover: OnceLock<ItemRenamePopover>,
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
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("needs-attention").build()])
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            self.setup_context_menu();
            self.setup_drag();
            self.setup_drop();

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
                #[weak]
                obj,
                move |_action, _parameter| obj.prompt_rename()
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("duplicate", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    imp.document().duplicate();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("trash", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    imp.document().trash();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("delete", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    imp.document().delete();
                }
            ));
            actions.add_action(&action);
        }
    }

    impl WidgetImpl for DocumentRow {}
    impl ListBoxRowImpl for DocumentRow {}

    impl DocumentRow {
        pub(super) fn document(&self) -> &Document {
            self.document.get().unwrap()
        }

        fn setup_context_menu(&self) {
            let obj = self.obj();
            let builder = Builder::from_resource(
                "/org/scratchmark/Scratchmark/ui/library/document_context_menu.ui",
            );
            let model = builder.object::<MenuModel>("context-menu").unwrap();
            let popover = PopoverMenu::builder()
                .menu_model(&model)
                .has_arrow(false)
                .build();
            popover.set_halign(gtk::Align::Start);
            popover.set_parent(&*obj);
            self.context_menu_popover.set(popover).unwrap();
            let gesture = gtk::GestureClick::new();
            gesture.set_button(gdk::ffi::GDK_BUTTON_SECONDARY as u32);
            gesture.connect_released(clone!(
                #[weak(rename_to = imp)]
                self,
                move |gesture, _n, x, y| {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    let popover = imp.context_menu_popover.get().unwrap();
                    popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
                    popover.popup();
                }
            ));
            obj.add_controller(gesture);

            obj.connect_destroy(move |obj| {
                obj.imp().context_menu_popover.get().unwrap().unparent();
            });
        }

        fn setup_rename_menu(&self) {
            let obj = self.obj();
            let popover = ItemRenamePopover::for_document();
            popover.set_parent(&*obj);
            popover.set_path(self.document.get().unwrap().path());
            popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_popover: ItemRenamePopover, path: PathBuf| {
                        if let Err(e) = imp.document().rename(path) {
                            imp.document().notify(&e.to_string())
                        }
                    }
                ),
            );
            self.rename_popover.set(popover).unwrap();
            obj.connect_destroy(move |obj| {
                obj.imp().rename_popover.get().unwrap().unparent();
            });
        }

        fn setup_drag(&self) {
            let obj = self.obj();

            let drag_source = DragSource::new();
            drag_source.set_actions(DragAction::MOVE);
            drag_source.set_content(Some(&gdk::ContentProvider::for_value(&obj.to_value())));

            obj.add_controller(drag_source.clone());
            let _ = self.drag_source.replace(Some(drag_source));
        }

        pub(super) fn setup_drop(&self) {
            let obj = self.obj();

            let drop_target = DropTarget::new(glib::types::Type::INVALID, DragAction::MOVE);
            drop_target.set_types(&[super::DocumentRow::static_type(), FolderRow::static_type()]);
            drop_target.connect_drop(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |_: &DropTarget, value: &glib::Value, _: f64, _: f64| {
                    if let Ok(doc) = value.get::<super::DocumentRow>() {
                        let old_path = doc.path();
                        let filename = old_path.file_name().unwrap();
                        let target_path = obj.document().path().parent().unwrap().to_path_buf();
                        let new_path = target_path.join(filename);
                        if new_path == old_path {
                            return true;
                        }
                        doc.rename(new_path);
                        return true;
                    } else if let Ok(other) = value.get::<FolderRow>() {
                        // Under no circumstance accept the library root folder
                        if other.folder().is_root() {
                            return true;
                        }
                        let old_path = other.folder().path();
                        let filename = old_path.file_name().unwrap();
                        let target_path = obj.document().path().parent().unwrap().to_path_buf();
                        if target_path.starts_with(&old_path) {
                            return true;
                        }
                        let new_path = target_path.join(filename);
                        if new_path == old_path {
                            return true;
                        }
                        other.rename(new_path);
                        return true;
                    }
                    false
                }
            ));

            obj.add_controller(drop_target);
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
        let imp = obj.imp();
        imp.bind(data);
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
        self.emit_by_name::<()>("needs-attention", &[]);
        self.imp().rename_popover.get().unwrap().popup();
    }

    pub fn rename(&self, path: PathBuf) {
        if let Err(e) = self.document().rename(path) {
            self.document().notify(&e.to_string())
        }
    }
}
