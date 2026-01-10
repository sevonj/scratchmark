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

    use gio::MenuModel;
    use gio::SimpleActionGroup;
    use gtk::Builder;
    use gtk::Button;
    use gtk::CompositeTemplate;
    use gtk::DragSource;
    use gtk::DropTarget;
    use gtk::FileLauncher;
    use gtk::Image;
    use gtk::Label;
    use gtk::PopoverMenu;
    use gtk::TemplateChild;
    use gtk::ToggleButton;
    use gtk::glib::Binding;

    use super::super::item_rename_popover::ItemRenamePopover;
    use super::FileButton;
    use crate::data::FolderObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/folder_view.ui")]
    pub struct FolderView {
        #[template_child]
        pub(super) expand_button_cont: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) expand_button: TemplateChild<ToggleButton>,
        #[template_child]
        pub(super) expand_icon: TemplateChild<Image>,
        #[template_child]
        pub(super) folder_icon: TemplateChild<Image>,
        #[template_child]
        pub(super) title: TemplateChild<Label>,
        #[template_child]
        pub(super) content_vbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) subdirs_vbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) documents_vbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) title_row: TemplateChild<gtk::Box>,

        pub(super) is_project_root: Cell<bool>,
        pub(super) folder_object: OnceLock<FolderObject>,
        pub(super) bindings: RefCell<Vec<Binding>>,
        pub(super) expanded: RefCell<bool>,
        pub(super) subdirs: RefCell<Vec<super::FolderView>>,
        pub(super) documents: RefCell<Vec<FileButton>>,

        pub(super) context_menu_popover: RefCell<Option<PopoverMenu>>,
        pub(super) rename_popover: RefCell<Option<ItemRenamePopover>>,
        pub(super) drag_source: RefCell<Option<DragSource>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FolderView {
        const NAME: &'static str = "FolderView";
        type Type = super::FolderView;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FolderView {
        fn constructed(&self) {
            self.parent_constructed();

            self.setup_drop();

            self.expand_button.connect_clicked(clone!(
                #[weak(rename_to = this)]
                self,
                move |_| {
                    this.toggle_expand();
                    this.folder_object().select();
                }
            ));

            self.set_expanded(false);
        }
    }

    impl WidgetImpl for FolderView {}
    impl BinImpl for FolderView {}

    impl FolderView {
        pub(super) fn prompt_rename(&self) {
            self.rename_popover.borrow().as_ref().unwrap().popup();
        }

        pub(super) fn folder_object(&self) -> &FolderObject {
            self.folder_object.get().unwrap()
        }

        /// Filepath
        pub(super) fn path(&self) -> PathBuf {
            self.folder_object().path()
        }

        /// Display name
        pub(super) fn name(&self) -> String {
            self.folder_object().name()
        }

        pub(super) fn set_expanded(&self, expanded: bool) {
            self.expanded.replace(expanded);

            if expanded {
                self.expand_icon.set_icon_name("down-small-symbolic".into());
                self.content_vbox.set_visible(true);
                if !self.is_project_root.get() {
                    self.folder_icon.set_icon_name(Some("folder-open-symbolic"));
                }
            } else {
                self.expand_icon
                    .set_icon_name("right-small-symbolic".into());
                self.content_vbox.set_visible(false);
                if !self.is_project_root.get() {
                    self.folder_icon.set_icon_name(Some("folder-symbolic"));
                }
            }
        }

        fn sort_children(&self) {
            let mut documents = self.documents.borrow_mut();
            if !documents.is_empty() {
                fn compare(a: &FileButton, b: &FileButton) -> std::cmp::Ordering {
                    a.stem().to_lowercase().cmp(&b.stem().to_lowercase())
                }
                documents.sort_unstable_by(compare);

                for i in (0..documents.len() - 1).rev() {
                    let child = &documents[i + 1];
                    let sibling = Some(&documents[i]);
                    self.documents_vbox.reorder_child_after(child, sibling);
                }
            }

            let mut subdirs = self.subdirs.borrow_mut();
            if !subdirs.is_empty() {
                fn compare(a: &super::FolderView, b: &super::FolderView) -> std::cmp::Ordering {
                    a.name().to_lowercase().cmp(&b.name().to_lowercase())
                }
                subdirs.sort_unstable_by(compare);

                for i in (0..subdirs.len() - 1).rev() {
                    let child = &subdirs[i + 1];
                    let sibling = Some(&subdirs[i]);
                    self.subdirs_vbox.reorder_child_after(child, sibling);
                }
            }
        }

        fn toggle_expand(&self) {
            let expanded = !*self.expanded.borrow();
            self.set_expanded(expanded);
        }

        pub(super) fn add_subfolder(&self, folder: super::FolderView) {
            self.subdirs_vbox.append(&folder);
            self.subdirs.borrow_mut().push(folder);
        }

        pub(super) fn add_document(&self, doc: FileButton) {
            self.documents_vbox.append(&doc);
            self.documents.borrow_mut().push(doc);
        }

        pub(super) fn setup_context_menu(&self, resource_path: &str) {
            let obj = self.obj();

            let builder = Builder::from_resource(resource_path);
            let popover = builder
                .object::<MenuModel>("context-menu")
                .expect("FolderView context-menu model failed");
            let menu = PopoverMenu::builder()
                .menu_model(&popover)
                .has_arrow(false)
                .build();
            let expand_button: &Button = self.expand_button.as_ref();
            menu.set_parent(expand_button);
            let _ = self.context_menu_popover.replace(Some(menu));

            let gesture = gtk::GestureClick::new();
            gesture.set_button(gdk::ffi::GDK_BUTTON_SECONDARY as u32);
            gesture.connect_released(clone!(
                #[weak(rename_to = this)]
                self,
                move |gesture, _n, x, y| {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    if let Some(popover) = this.context_menu_popover.borrow().as_ref() {
                        popover
                            .set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                        popover.popup();
                    };
                }
            ));
            self.expand_button.add_controller(gesture);

            obj.connect_destroy(move |obj| {
                if let Some(popover) = obj.imp().context_menu_popover.take() {
                    popover.unparent();
                }
            });
        }

        pub(super) fn setup_rename_menu(&self) {
            let obj = self.obj();

            let menu = ItemRenamePopover::for_folder();
            menu.set_parent(&*obj);

            menu.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_popover: ItemRenamePopover, path: PathBuf| {
                        if let Err(e) = this.folder_object().rename(path) {
                            this.folder_object().notify(&e.to_string())
                        }
                    }
                ),
            );

            let _ = self.rename_popover.replace(Some(menu));

            obj.connect_destroy(move |obj| {
                if let Some(popover) = obj.imp().rename_popover.take() {
                    popover.unparent();
                }
            });
        }

        pub(super) fn setup_drag(&self) {
            let obj = self.obj();

            let drag_source = DragSource::new();
            drag_source.set_actions(gdk::DragAction::COPY);
            drag_source.set_content(Some(&gdk::ContentProvider::for_value(&obj.to_value())));

            self.expand_button_cont.add_controller(drag_source.clone());
            let _ = self.drag_source.replace(Some(drag_source));
        }

        fn setup_drop(&self) {
            let obj = self.obj();

            let drop_target = DropTarget::new(glib::types::Type::INVALID, gdk::DragAction::COPY);
            drop_target.set_types(&[FileButton::static_type(), super::FolderView::static_type()]);
            drop_target.connect_drop(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |_: &DropTarget, value: &glib::Value, _: f64, _: f64| {
                    if let Ok(doc) = value.get::<FileButton>() {
                        let old_path = doc.path();
                        let filename = old_path.file_name().unwrap();
                        let target_path = obj.path();
                        let new_path = target_path.join(filename);
                        if new_path == old_path {
                            return true;
                        }
                        doc.rename(new_path);
                        obj.imp().set_expanded(true);
                        return true;
                    } else if let Ok(folder) = value.get::<super::FolderView>() {
                        // Under no circumstance accept the library root folder
                        if folder.is_root() {
                            return true;
                        }
                        let old_path = folder.path();
                        let filename = old_path.file_name().unwrap();
                        let target_path = obj.path();
                        if target_path.starts_with(&old_path) {
                            return true;
                        }
                        let new_path = target_path.join(filename);
                        if new_path == old_path {
                            return true;
                        }
                        folder.rename(new_path);
                        obj.imp().set_expanded(true);
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
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Err(e) = this.folder_object().create_document() {
                        this.folder_object().notify(&e.to_string())
                    }
                    this.sort_children();
                    this.set_expanded(true);
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("create-subfolder", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Err(e) = this.folder_object().create_subfolder() {
                        this.folder_object().notify(&e.to_string())
                    }
                    this.sort_children();
                    this.set_expanded(true);
                }
            ));
            actions.add_action(&action);

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
        }

        pub(super) fn setup_actions_subfolder(&self) {
            let obj = self.obj();
            let actions = SimpleActionGroup::new();
            obj.insert_action_group("subfolder", Some(&actions));

            let action = gio::SimpleAction::new("rename-begin", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    assert!(!this.obj().is_root());
                    this.rename_popover.borrow().as_ref().unwrap().popup();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("trash", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Err(e) = this.folder_object().trash() {
                        this.folder_object().notify(&e.to_string())
                    }
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("delete", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Err(e) = this.folder_object().delete() {
                        this.folder_object().notify(&e.to_string())
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
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Err(e) = this.folder_object().close_project() {
                        this.folder_object().notify(&e.to_string())
                    }
                }
            ));
            actions.add_action(&action);
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;

use glib::Object;
use gtk::ToggleButton;

use super::FileButton;
use crate::data::FolderObject;

glib::wrapper! {
    pub struct FolderView(ObjectSubclass<imp::FolderView>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FolderView {
    /// Normal folder
    pub fn new(data: &FolderObject) -> Self {
        let this: Self = Object::builder().build();
        let imp = this.imp();
        imp.setup_rename_menu();
        imp.setup_context_menu("/org/scratchmark/Scratchmark/ui/library/folder_context_menu.ui");
        imp.setup_drag();
        imp.setup_actions_common();
        imp.setup_actions_subfolder();
        this.bind(data);
        this
    }

    /// Project root folder
    pub fn new_project_root(data: &FolderObject) -> Self {
        let this: Self = Object::builder().build();
        let imp = this.imp();
        imp.setup_context_menu("/org/scratchmark/Scratchmark/ui/library/root_context_menu.ui");
        imp.is_project_root.replace(true);
        imp.folder_icon.set_icon_name(Some("project-symbolic"));
        imp.setup_actions_common();
        imp.setup_actions_project_root();
        this.bind(data);
        this
    }

    /// Special root folder for builtin drafts project
    pub fn new_drafts_root(data: &FolderObject) -> Self {
        let this: Self = Object::builder().build();
        let imp = this.imp();
        imp.setup_context_menu("/org/scratchmark/Scratchmark/ui/library/drafts_context_menu.ui");
        imp.is_project_root.replace(true);
        imp.folder_icon.set_icon_name(Some("draft-table-symbolic"));
        imp.setup_actions_common();
        this.bind(data);
        imp.title.set_label("Drafts");
        this
    }

    pub fn folder_object(&self) -> &FolderObject {
        self.imp().folder_object()
    }

    /// Is root folder of library
    pub fn is_root(&self) -> bool {
        self.imp().is_project_root.get()
    }

    /// Filepath
    pub fn path(&self) -> PathBuf {
        self.imp().path()
    }

    /// Display name
    pub fn name(&self) -> String {
        self.imp().name()
    }

    pub fn is_expanded(&self) -> bool {
        self.imp().expanded.borrow().to_owned()
    }

    pub fn set_expanded(&self, expanded: bool) {
        self.imp().set_expanded(expanded);
    }

    pub fn add_subfolder(&self, folder: FolderView) {
        self.imp().add_subfolder(folder);
    }

    pub fn add_document(&self, doc: FileButton) {
        self.imp().add_document(doc);
    }

    pub fn remove_subfolder(&self, folder: &FolderView) {
        self.imp().subdirs_vbox.remove(folder);
    }

    pub fn remove_document(&self, doc: &FileButton) {
        self.imp().documents_vbox.remove(doc);
    }

    pub fn prompt_rename(&self) {
        self.imp().prompt_rename();
    }

    pub fn rename(&self, path: PathBuf) {
        if let Err(e) = self.folder_object().rename(path) {
            self.folder_object().notify(&e.to_string())
        }
    }

    fn bind(&self, data: &FolderObject) {
        let imp = self.imp();
        imp.folder_object.get_or_init(|| data.clone());
        let path = data.path();

        let expand_button: &ToggleButton = imp.expand_button.as_ref();
        data.bind_property("is_selected", expand_button, "active")
            .bidirectional()
            .build();

        if let Some(rename_popover) = imp.rename_popover.borrow().as_ref() {
            rename_popover.set_path(path);
        }
        imp.title_row
            .set_margin_start(std::cmp::max(12 * data.depth() as i32, 0));
        let title_label = imp.title.get();
        let mut bindings = imp.bindings.borrow_mut();

        let title_binding = data
            .bind_property("name", &title_label, "label")
            .sync_create()
            .build();
        bindings.push(title_binding);
    }
}
