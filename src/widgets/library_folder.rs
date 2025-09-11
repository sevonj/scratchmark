//! Expandable folder widget for library browser
//!

mod imp {
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use gtk::gdk;
    use gtk::gio;
    use gtk::glib;
    use gtk::glib::closure_local;
    use gtk::prelude::*;

    use gdk::Rectangle;
    use gio::{MenuModel, SimpleActionGroup};
    use glib::Binding;
    use glib::subclass::Signal;
    use gtk::{Builder, Button, FileLauncher, Image, Label, PopoverMenu};
    use gtk::{CompositeTemplate, TemplateChild};
    use gtk::{DragSource, DropTarget};

    use crate::data::FolderObject;
    use crate::util;
    use crate::widgets::ItemRenamePopover;
    use crate::widgets::LibrarySheet;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library_folder.ui")]
    pub struct LibraryFolder {
        #[template_child]
        pub(super) expand_button_cont: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) expand_button: TemplateChild<Button>,
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
        pub(super) sheets_vbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) title_row: TemplateChild<gtk::Box>,

        pub(super) is_project_root: Cell<bool>,
        pub(super) folder_object: RefCell<Option<FolderObject>>,
        pub(super) bindings: RefCell<Vec<Binding>>,
        pub(super) expanded: RefCell<bool>,
        pub(super) subdirs: RefCell<Vec<super::LibraryFolder>>,
        pub(super) sheets: RefCell<Vec<LibrarySheet>>,

        pub(super) context_menu_popover: RefCell<Option<PopoverMenu>>,
        pub(super) rename_popover: RefCell<Option<ItemRenamePopover>>,
        pub(super) drag_source: RefCell<Option<DragSource>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryFolder {
        const NAME: &'static str = "LibraryFolder";
        type Type = super::LibraryFolder;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibraryFolder {
        fn constructed(&self) {
            self.parent_constructed();

            self.setup_context_menu();
            self.setup_rename_menu();
            self.setup_drag();
            self.setup_drop();

            let this = self;
            self.expand_button.connect_clicked(clone!(
                #[weak]
                this,
                move |_| {
                    this.toggle_expand();
                }
            ));

            self.set_expanded(false);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("rename-requested")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("trash-requested")
                        .param_types([super::LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("delete-requested")
                        .param_types([super::LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("folder-created")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("sheet-created")
                        .param_types([PathBuf::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryFolder {}
    impl BinImpl for LibraryFolder {}

    impl LibraryFolder {
        /// Filepath
        pub(super) fn path(&self) -> PathBuf {
            self.folder_object
                .borrow()
                .as_ref()
                .expect("LibraryFolder data uninitialized")
                .path()
        }

        /// Display name
        pub(super) fn name(&self) -> String {
            self.folder_object
                .borrow()
                .as_ref()
                .expect("LibraryFolder data uninitialized")
                .name()
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
            let mut sheets = self.sheets.borrow_mut();
            if !sheets.is_empty() {
                fn compare(a: &LibrarySheet, b: &LibrarySheet) -> std::cmp::Ordering {
                    a.stem().to_lowercase().cmp(&b.stem().to_lowercase())
                }
                sheets.sort_unstable_by(compare);

                for i in (0..sheets.len() - 1).rev() {
                    let child = &sheets[i + 1];
                    let sibling = Some(&sheets[i]);
                    self.sheets_vbox.reorder_child_after(child, sibling);
                }
            }

            let mut subdirs = self.subdirs.borrow_mut();
            if !subdirs.is_empty() {
                fn compare(
                    a: &super::LibraryFolder,
                    b: &super::LibraryFolder,
                ) -> std::cmp::Ordering {
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

        pub(super) fn add_subfolder(&self, folder: super::LibraryFolder) {
            self.subdirs_vbox.append(&folder);
            self.subdirs.borrow_mut().push(folder);
        }

        pub(super) fn add_sheet(&self, sheet: LibrarySheet) {
            self.sheets_vbox.append(&sheet);
            self.sheets.borrow_mut().push(sheet);
        }

        fn setup_context_menu(&self) {
            let obj = self.obj();

            let builder = Builder::from_resource(
                "/org/scratchmark/Scratchmark/ui/library_folder_context_menu.ui",
            );
            let popover = builder
                .object::<MenuModel>("context-menu")
                .expect("LibraryFolder context-menu model failed");
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
                        popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
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

        fn setup_rename_menu(&self) {
            let obj = self.obj();

            let menu = ItemRenamePopover::for_folder();
            menu.set_parent(&*obj);

            menu.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_popover: ItemRenamePopover, path: PathBuf| {
                        obj.emit_by_name::<()>("rename-requested", &[&path]);
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

        fn setup_drag(&self) {
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
            drop_target.set_types(&[
                LibrarySheet::static_type(),
                super::LibraryFolder::static_type(),
            ]);
            drop_target.connect_drop(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |_: &DropTarget, value: &glib::Value, _: f64, _: f64| {
                    if let Ok(sheet) = value.get::<LibrarySheet>() {
                        let Ok(old_path) = sheet.path().canonicalize() else {
                            return true;
                        };
                        let filename = old_path.file_name().unwrap();
                        let Ok(target_path) = obj.path().canonicalize() else {
                            return true;
                        };
                        let new_path = target_path.join(filename);
                        if new_path == old_path {
                            return true;
                        }
                        sheet.rename(new_path);
                        obj.imp().set_expanded(true);
                        return true;
                    } else if let Ok(folder) = value.get::<super::LibraryFolder>() {
                        // Under no circumstance accept the library root folder
                        if folder.is_root() {
                            return true;
                        }
                        let Ok(old_path) = folder.path().canonicalize() else {
                            return true;
                        };
                        let filename = old_path.file_name().unwrap();
                        let Ok(target_path) = obj.path().canonicalize() else {
                            return true;
                        };
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

        pub(super) fn setup_actions(&self) {
            let obj = self.obj();

            let actions = SimpleActionGroup::new();
            obj.insert_action_group("folder", Some(&actions));

            let action = gio::SimpleAction::new("create-sheet", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    let path = util::untitled_sheet_path(obj.path());
                    util::create_sheet_file(&path);
                    obj.emit_by_name::<()>("sheet-created", &[&path]);
                    obj.imp().sort_children();
                    obj.imp().set_expanded(true);
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("create-folder", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    let path = util::untitled_folder_path(obj.path());
                    util::create_folder(&path);
                    obj.emit_by_name::<()>("folder-created", &[&path]);
                    obj.imp().sort_children();
                    obj.imp().set_expanded(true);
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
                #[weak]
                obj,
                move |_action, _parameter| {
                    assert!(!obj.is_root());
                    obj.emit_by_name::<()>("trash-requested", &[&obj]);
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("delete", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    assert!(!obj.is_root());
                    obj.emit_by_name::<()>("delete-requested", &[&obj]);
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

use crate::data::FolderObject;
use crate::widgets::LibrarySheet;

glib::wrapper! {
    pub struct LibraryFolder(ObjectSubclass<imp::LibraryFolder>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LibraryFolder {
    /// Normal folder
    pub fn new(data: &FolderObject) -> Self {
        let this: Self = Object::builder().build();
        this.imp().setup_actions();
        this.bind(data);
        this
    }

    /// Project root folder
    pub fn new_project_root(data: &FolderObject) -> Self {
        let this = Self::new(data);
        this.imp().is_project_root.replace(true);
        this.imp()
            .folder_icon
            .set_icon_name(Some("library-symbolic"));
        this.imp().expand_icon.set_visible(false);
        this.imp().expand_button.set_sensitive(false);
        this.imp().title.set_label("Library");
        this.imp().content_vbox.set_margin_start(0);
        this.imp().content_vbox.set_margin_bottom(8); // Provide some empty space to aid dragging
        this.imp().set_expanded(true);
        if let Some(popover) = this.imp().rename_popover.take() {
            popover.unparent();
        }
        this
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

    pub fn add_subfolder(&self, folder: LibraryFolder) {
        self.imp().add_subfolder(folder);
    }

    pub fn add_sheet(&self, sheet: LibrarySheet) {
        self.imp().add_sheet(sheet);
    }

    pub fn remove_subfolder(&self, folder: &LibraryFolder) {
        self.imp().subdirs_vbox.remove(folder);
    }

    pub fn remove_sheet(&self, sheet: &LibrarySheet) {
        self.imp().sheets_vbox.remove(sheet);
    }

    pub fn rename(&self, path: PathBuf) {
        assert!(!self.is_root());
        assert!(path.parent().is_some_and(|p| p.is_dir()));
        self.emit_by_name::<()>("rename-requested", &[&path]);
    }

    fn bind(&self, data: &FolderObject) {
        self.imp().folder_object.replace(Some(data.clone()));
        let path = data.path();
        self.imp()
            .rename_popover
            .borrow()
            .as_ref()
            .unwrap()
            .set_path(path);

        self.imp()
            .title_row
            .set_margin_start(std::cmp::max(12 * data.depth() as i32, 0));
        let title_label = self.imp().title.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let title_binding = data
            .bind_property("name", &title_label, "label")
            .sync_create()
            .build();
        bindings.push(title_binding);
    }
}
