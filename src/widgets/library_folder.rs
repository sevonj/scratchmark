//! Expandable folder widget for library browser
//!

mod imp {
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::clone;
    use gtk::gdk;
    use gtk::gio;
    use gtk::glib;
    use gtk::prelude::*;

    use gdk::Rectangle;
    use gio::{MenuModel, SimpleActionGroup};
    use glib::Binding;
    use glib::subclass::Signal;
    use gtk::{Builder, Button, FileLauncher, Image, Label, PopoverMenu};
    use gtk::{CompositeTemplate, TemplateChild};

    use crate::data::FolderObject;
    use crate::widgets::LibrarySheetButton;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_folder.ui")]
    pub struct LibraryFolder {
        #[template_child]
        pub(super) expand_button: TemplateChild<Button>,
        #[template_child]
        pub(super) expand_icon: TemplateChild<Image>,
        #[template_child]
        pub(super) title: TemplateChild<Label>,
        #[template_child]
        pub(super) content_vbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) subdirs_vbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) sheets_vbox: TemplateChild<gtk::Box>,

        pub(super) folder_object: RefCell<Option<FolderObject>>,
        pub(super) bindings: RefCell<Vec<Binding>>,
        pub(super) expanded: RefCell<bool>,
        pub(super) subdirs: RefCell<Vec<super::LibraryFolder>>,
        pub(super) sheets: RefCell<Vec<LibrarySheetButton>>,

        context_menu_popover: RefCell<Option<PopoverMenu>>,
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
            let obj = self.obj();

            let this = self;
            self.expand_button.connect_clicked(clone!(
                #[weak]
                this,
                move |_| {
                    this.toggle_expand();
                }
            ));

            self.set_expand(false);

            let builder =
                Builder::from_resource("/fi/sevonj/TheftMD/ui/library_folder_context_menu.ui");
            let popover = builder
                .object::<MenuModel>("context-menu")
                .expect("LibraryFolder context-menu model failed");
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

            let actions = SimpleActionGroup::new();
            obj.insert_action_group("folder", Some(&actions));

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

            let action = gio::SimpleAction::new("delete", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    obj.emit_by_name::<()>("folder-delete-requested", &[&obj]);
                }
            ));
            actions.add_action(&action);

            obj.connect_destroy(move |obj| {
                let popover = obj
                    .imp()
                    .context_menu_popover
                    .take()
                    .expect("LibraryFolder context menu uninitialized");
                popover.unparent();
            });
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("folder-delete-requested")
                        .param_types([super::LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("sheet-clicked")
                        .param_types([LibrarySheetButton::static_type()])
                        .build(),
                    Signal::builder("sheet-delete-requested")
                        .param_types([LibrarySheetButton::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryFolder {}
    impl BinImpl for LibraryFolder {}

    impl LibraryFolder {
        fn toggle_expand(&self) {
            let expand = !*self.expanded.borrow();
            self.set_expand(expand);
        }

        pub(super) fn set_expand(&self, expand: bool) {
            self.expanded.replace(expand);

            if expand {
                self.expand_icon.set_icon_name("pan-down-symbolic".into());
                self.subdirs_vbox.set_visible(true);
                self.sheets_vbox.set_visible(true);
                self.obj().refresh_content();
            } else {
                self.expand_icon.set_icon_name("pan-end-symbolic".into());
                self.subdirs_vbox.set_visible(false);
                self.sheets_vbox.set_visible(false);
            }
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use glib::Object;
use glib::clone;
use glib::closure_local;
use gtk::glib;
use gtk::prelude::*;

use crate::data::FolderObject;
use crate::data::SheetObject;

use super::LibrarySheetButton;

glib::wrapper! {
    pub struct LibraryFolder(ObjectSubclass<imp::LibraryFolder>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LibraryFolder {
    pub fn new(data: &FolderObject) -> Self {
        let this: Self = Object::builder().build();
        this.bind(data);
        this
    }

    pub fn new_root(data: &FolderObject) -> Self {
        let this: Self = Object::builder().build();
        this.bind(data);
        this.imp().expand_icon.set_visible(false);
        this.imp().expand_button.set_sensitive(false);
        this.imp().title.set_label("Library");
        this.imp().content_vbox.set_margin_start(0);
        this.imp().set_expand(true);
        this
    }

    pub fn path(&self) -> PathBuf {
        self.imp()
            .folder_object
            .borrow()
            .as_ref()
            .expect("LibraryFolder data uninitialized")
            .path()
    }

    pub fn refresh_content(&self) {
        self.prune_invalid_children();

        for subdir in self.imp().subdirs.borrow().iter() {
            subdir.refresh_content();
        }

        let opt = self.imp().folder_object.borrow();
        let folder = opt.as_ref().expect("FolderObject not bound");

        let entries = folder.content();
        for entry in entries {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let path = entry.path();
            if self.has_child(&path) {
                continue;
            }

            if meta.is_dir() {
                self.add_subdir(FolderObject::new(path));
            } else if meta.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                self.add_sheet(SheetObject::new(path));
            }
        }
    }

    fn add_subdir(&self, data: FolderObject) {
        let folder = LibraryFolder::new(&data);
        self.imp().subdirs_vbox.append(&folder);
        folder.refresh_content();
        let this = self;

        folder.connect_closure(
            "folder-delete-requested",
            false,
            closure_local!(
                #[weak]
                this,
                move |_: super::LibraryFolder, folder: super::LibraryFolder| {
                    this.emit_by_name::<()>("folder-delete-requested", &[&folder]);
                }
            ),
        );
        folder.connect_closure(
            "sheet-clicked",
            false,
            closure_local!(
                #[weak]
                this,
                move |_: LibraryFolder, button: LibrarySheetButton| {
                    this.emit_by_name::<()>("sheet-clicked", &[&button]);
                }
            ),
        );
        folder.connect_closure(
            "sheet-delete-requested",
            false,
            closure_local!(
                #[weak]
                this,
                move |_: super::LibraryFolder, button: LibrarySheetButton| {
                    this.emit_by_name::<()>("sheet-delete-requested", &[&button]);
                }
            ),
        );

        self.imp().subdirs.borrow_mut().push(folder);
    }

    fn add_sheet(&self, data: SheetObject) {
        let button = LibrarySheetButton::new(&data);
        self.imp().sheets_vbox.append(&button);

        let this = self;
        button.connect_clicked(clone!(
            #[weak]
            this,
            move |button| {
                this.emit_by_name::<()>("sheet-clicked", &[button]);
            }
        ));
        button.connect_closure(
            "delete-requested",
            false,
            closure_local!(
                #[weak]
                this,
                move |button: LibrarySheetButton| {
                    this.emit_by_name::<()>("sheet-delete-requested", &[&button]);
                }
            ),
        );

        self.imp().sheets.borrow_mut().push(button);
    }

    fn has_child(&self, path: &PathBuf) -> bool {
        for subdir in self.imp().subdirs.borrow().iter() {
            if subdir.path() == *path {
                return true;
            }
        }
        for sheet in self.imp().sheets.borrow().iter() {
            if sheet.path() == *path {
                return true;
            }
        }
        false
    }

    fn prune_invalid_children(&self) {
        let mut sheets = self.imp().sheets.borrow_mut();
        for i in (0..sheets.len()).rev() {
            let sheet = &sheets[i];
            if !sheet.path().exists() {
                self.imp().sheets_vbox.remove(sheet);
                sheets.remove(i);
            }
        }

        let mut subdirs = self.imp().subdirs.borrow_mut();
        for i in (0..subdirs.len()).rev() {
            let subdir = &subdirs[i];
            if !subdir.path().exists() {
                self.imp().subdirs_vbox.remove(subdir);
                subdirs.remove(i);
            }
        }
    }

    fn bind(&self, data: &FolderObject) {
        self.imp().folder_object.replace(Some(data.clone()));

        let title_label = self.imp().title.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let title_binding = data
            .bind_property("name", &title_label, "label")
            .sync_create()
            .build();
        bindings.push(title_binding);
    }
}
