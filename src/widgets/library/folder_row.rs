mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::gdk;
    use gtk::gio;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::glib::closure_local;
    use gtk::prelude::*;

    use gtk::Builder;
    use gtk::CompositeTemplate;
    use gtk::DragSource;
    use gtk::DropTarget;
    use gtk::FileLauncher;
    use gtk::Image;
    use gtk::Label;
    use gtk::ListBoxRow;
    use gtk::PopoverMenu;
    use gtk::TemplateChild;
    use gtk::gio::MenuModel;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib::Binding;
    use gtk::glib::Properties;
    use gtk::glib::subclass::Signal;

    use super::DocumentRow;
    use crate::data::Folder;
    use crate::widgets::library::document_create_popover::DocumentCreatePopover;
    use crate::widgets::library::folder_create_popover::FolderCreatePopover;
    use crate::widgets::library::item_rename_popover::ItemRenamePopover;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::FolderRow)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/folder_row.ui")]
    pub struct FolderRow {
        #[template_child]
        pub(super) expand_icon: TemplateChild<Image>,
        #[template_child]
        pub(super) folder_icon: TemplateChild<Image>,
        #[template_child]
        pub(super) title: TemplateChild<Label>,
        #[template_child]
        pub(super) title_row: TemplateChild<gtk::Box>,

        pub(super) folder: OnceLock<Folder>,
        pub(super) bindings: RefCell<Vec<Binding>>,
        #[property(get, set)]
        pub(super) is_expanded: Cell<bool>,

        pub(super) context_menu_popover: RefCell<Option<PopoverMenu>>,
        pub(super) document_create_popover: OnceLock<DocumentCreatePopover>,
        pub(super) folder_create_popover: OnceLock<FolderCreatePopover>,
        pub(super) rename_popover: OnceLock<ItemRenamePopover>,
        pub(super) drag_source: RefCell<Option<DragSource>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FolderRow {
        const NAME: &'static str = "FolderRow";
        type Type = super::FolderRow;
        type ParentType = ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FolderRow {
        fn constructed(&self) {
            let obj = self.obj();

            obj.connect_notify(Some("is-expanded"), move |obj, _| {
                obj.imp().on_expanded_changed();
            });

            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("needs-attention").build(),
                    Signal::builder("prompt-create-document").build(),
                    Signal::builder("prompt-create-subfolder").build(),
                ]
            })
        }
    }

    impl WidgetImpl for FolderRow {}
    impl ListBoxRowImpl for FolderRow {}

    impl FolderRow {
        pub(super) fn folder(&self) -> &Folder {
            self.folder.get().unwrap()
        }

        fn on_expanded_changed(&self) {
            if self.is_expanded.get() {
                self.expand_icon.set_icon_name("down-small-symbolic".into());
                if !self.folder().is_root() {
                    self.folder_icon.set_icon_name(Some("folder-open-symbolic"));
                }
            } else {
                self.expand_icon
                    .set_icon_name("right-small-symbolic".into());
                if !self.folder().is_root() {
                    self.folder_icon.set_icon_name(Some("folder-symbolic"));
                }
            }
        }

        pub(super) fn toggle_expand(&self) {
            self.obj().set_is_expanded(!self.is_expanded.get());
        }

        pub(super) fn setup_context_menu(&self, resource_path: &str) {
            let obj = self.obj();

            let builder = Builder::from_resource(resource_path);
            let popover = builder
                .object::<MenuModel>("context-menu")
                .expect("FolderItem context-menu model failed");
            let menu = PopoverMenu::builder()
                .menu_model(&popover)
                .has_arrow(false)
                .build();
            menu.set_halign(gtk::Align::Start);
            menu.set_parent(obj.as_ref());
            let _ = self.context_menu_popover.replace(Some(menu));

            let gesture = gtk::GestureClick::new();
            gesture.set_button(gdk::ffi::GDK_BUTTON_SECONDARY as u32);
            gesture.connect_released(clone!(
                #[weak(rename_to = imp)]
                self,
                move |gesture, _n, x, y| {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    if let Some(popover) = imp.context_menu_popover.borrow().as_ref() {
                        popover
                            .set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
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

        pub(super) fn setup_rename_menu(&self) {
            let obj = self.obj();
            let rename_popover = ItemRenamePopover::for_folder();
            rename_popover.set_parent(&*obj);
            rename_popover.set_path(self.folder.get().unwrap().path());
            rename_popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_popover: ItemRenamePopover, path: PathBuf| {
                        if let Err(e) = imp.folder().rename(path) {
                            imp.folder().notify(&e.to_string())
                        }
                    }
                ),
            );
            self.rename_popover.set(rename_popover).unwrap();
        }

        pub(super) fn setup_document_create_menu(&self) {
            let obj = self.obj();
            let popover = DocumentCreatePopover::new(self.folder().path());
            popover.set_parent(&*obj);
            popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: DocumentCreatePopover, name: PathBuf| {
                        if let Err(e) = imp.folder().create_document(name) {
                            imp.folder().notify(&e.to_string())
                        }
                    }
                ),
            );
            self.document_create_popover.set(popover).unwrap();
        }

        pub(super) fn setup_folder_create_menu(&self) {
            let obj = self.obj();
            let popover = FolderCreatePopover::new(self.folder().path());
            popover.set_parent(&*obj);
            popover.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: FolderCreatePopover, name: PathBuf| {
                        if let Err(e) = imp.folder().create_subfolder(name) {
                            imp.folder().notify(&e.to_string())
                        }
                    }
                ),
            );
            self.folder_create_popover.set(popover).unwrap();
        }

        pub(super) fn setup_drag(&self) {
            let obj = self.obj();

            let drag_source = DragSource::new();
            drag_source.set_actions(gdk::DragAction::MOVE);
            drag_source.set_content(Some(&gdk::ContentProvider::for_value(&obj.to_value())));

            obj.add_controller(drag_source.clone());
            let _ = self.drag_source.replace(Some(drag_source));
        }

        pub(super) fn setup_drop(&self) {
            let obj = self.obj();

            let drop_target = DropTarget::new(glib::types::Type::INVALID, gdk::DragAction::MOVE);
            drop_target.set_types(&[DocumentRow::static_type(), super::FolderRow::static_type()]);
            drop_target.connect_drop(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |_: &DropTarget, value: &glib::Value, _: f64, _: f64| {
                    if let Ok(doc) = value.get::<DocumentRow>() {
                        let old_path = doc.path();
                        let filename = old_path.file_name().unwrap();
                        let target_path = obj.folder().path();
                        let new_path = target_path.join(filename);
                        if new_path == old_path {
                            return true;
                        }
                        doc.rename(new_path);
                        obj.set_is_expanded(true);
                        return true;
                    } else if let Ok(other) = value.get::<super::FolderRow>() {
                        // Under no circumstance accept the library root folder
                        if other.folder().is_root() {
                            return true;
                        }
                        let old_path = other.folder().path();
                        let filename = old_path.file_name().unwrap();
                        let target_path = obj.folder().path();
                        if target_path.starts_with(&old_path) {
                            return true;
                        }
                        let new_path = target_path.join(filename);
                        if new_path == old_path {
                            return true;
                        }
                        other.rename(new_path);
                        obj.set_is_expanded(true);
                        return true;
                    }
                    false
                }
            ));

            obj.add_controller(drop_target);
        }

        pub(super) fn setup_actions_common(&self) {
            let obj = self.obj();
            let actions = SimpleActionGroup::new();
            obj.insert_action_group("folder", Some(&actions));

            let action = gio::SimpleAction::new("create-document", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| obj.emit_by_name("prompt-create-document", &[])
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("create-subfolder", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| obj.emit_by_name("prompt-create-subfolder", &[])
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("filemanager", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    let file = gio::File::for_path(obj.folder().path());
                    FileLauncher::new(Some(&file)).open_containing_folder(
                        None::<&adw::ApplicationWindow>,
                        None::<&gio::Cancellable>,
                        |_| {},
                    );
                }
            ));
            actions.add_action(&action);
        }

        pub(super) fn setup_actions_subfolder(&self) {
            let obj = self.obj();
            let actions = SimpleActionGroup::new();
            obj.insert_action_group("subfolder", Some(&actions));

            let action = gio::SimpleAction::new("rename-begin", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    obj.prompt_rename();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("trash", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    if let Err(e) = imp.folder().trash() {
                        imp.folder().notify(&e.to_string())
                    }
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("delete", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    if let Err(e) = imp.folder().delete() {
                        imp.folder().notify(&e.to_string())
                    }
                }
            ));
            actions.add_action(&action);
        }

        pub(super) fn setup_actions_project_root(&self) {
            let obj = self.obj();
            let actions = SimpleActionGroup::new();
            obj.insert_action_group("project-root", Some(&actions));

            let action = gio::SimpleAction::new("close-project", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    if let Err(e) = imp.folder().close_project() {
                        imp.folder().notify(&e.to_string())
                    }
                }
            ));
            actions.add_action(&action);
        }

        pub(super) fn bind(&self, folder: &Folder) {
            self.folder.set(folder.clone()).unwrap();

            self.title_row
                .set_margin_start(std::cmp::max(12 * folder.depth() as i32, 0));
            let title_label = self.title.get();
            let mut bindings = self.bindings.borrow_mut();

            let title_binding = folder
                .bind_property("name", &title_label, "label")
                .sync_create()
                .build();
            bindings.push(title_binding);
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;

use glib::Object;
use gtk::ListBoxRow;

use crate::data::Folder;
use crate::data::FolderType;
use crate::widgets::library::DocumentRow;

glib::wrapper! {
    pub struct FolderRow(ObjectSubclass<imp::FolderRow>)
        @extends ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl FolderRow {
    pub fn new(folder: &Folder) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        imp.bind(folder);

        imp.setup_actions_common();
        match folder.kind() {
            FolderType::Subfolder => {
                imp.setup_rename_menu();
                imp.setup_context_menu(
                    "/org/scratchmark/Scratchmark/ui/library/folder_context_menu.ui",
                );
                imp.setup_drag();
                imp.setup_actions_subfolder();
            }
            FolderType::ProjectRoot => {
                imp.setup_context_menu(
                    "/org/scratchmark/Scratchmark/ui/library/root_context_menu.ui",
                );
                imp.folder_icon.set_icon_name(Some("project-symbolic"));
                imp.setup_actions_project_root();
            }
            FolderType::DraftsRoot => {
                imp.setup_context_menu(
                    "/org/scratchmark/Scratchmark/ui/library/drafts_context_menu.ui",
                );
                imp.folder_icon.set_icon_name(Some("draft-table-symbolic"));
            }
        }
        imp.setup_document_create_menu();
        imp.setup_folder_create_menu();

        imp.setup_drop();
        obj.set_is_expanded(false);
        obj
    }

    pub fn folder(&self) -> &Folder {
        self.imp().folder()
    }

    pub fn on_click(&self) {
        self.imp().toggle_expand();
        self.folder().select();
    }

    pub fn prompt_rename(&self) {
        self.emit_by_name::<()>("needs-attention", &[]);
        self.imp().rename_popover.get().unwrap().popup();
    }

    pub fn prompt_create_document(&self) {
        self.emit_by_name::<()>("needs-attention", &[]);
        self.imp().document_create_popover.get().unwrap().popup();
    }

    pub fn prompt_create_folder(&self) {
        self.emit_by_name::<()>("needs-attention", &[]);
        self.imp().folder_create_popover.get().unwrap().popup();
    }

    pub fn rename(&self, path: PathBuf) {
        if let Err(e) = self.folder().rename(path) {
            self.folder().notify(&e.to_string())
        }
    }
}
