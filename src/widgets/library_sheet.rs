//! Sheet button widget for library browser
//!

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
    use gtk::prelude::*;

    use gdk::Rectangle;
    use gio::{MenuModel, SimpleActionGroup};
    use glib::Binding;
    use glib::subclass::Signal;
    use gtk::DragSource;
    use gtk::ToggleButton;
    use gtk::{Builder, CompositeTemplate, FileLauncher, Label, PopoverMenu, TemplateChild};

    use crate::data::SheetObject;
    use crate::widgets::SheetRenamePopover;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_sheet.ui")]
    pub struct LibrarySheet {
        #[template_child]
        pub(super) button: TemplateChild<ToggleButton>,
        #[template_child]
        pub(super) sheet_name_label: TemplateChild<Label>,

        pub(super) sheet_object: RefCell<Option<SheetObject>>,
        pub(super) bindings: RefCell<Vec<Binding>>,

        context_menu_popover: RefCell<Option<PopoverMenu>>,
        pub(super) rename_popover: RefCell<Option<SheetRenamePopover>>,
        pub(super) drag_source: RefCell<Option<DragSource>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibrarySheet {
        const NAME: &'static str = "LibrarySheet";
        type Type = super::LibrarySheet;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibrarySheet {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            self.setup_context_menu();
            self.setup_rename_menu();
            self.setup_drag();

            self.button.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.emit_by_name::<()>("selected", &[]);
                }
            ));

            let actions = SimpleActionGroup::new();
            obj.insert_action_group("sheet", Some(&actions));

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
                    this.rename_popover.borrow().as_ref().unwrap().popup();
                }
            ));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("delete", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    obj.emit_by_name::<()>("delete-requested", &[]);
                }
            ));
            actions.add_action(&action);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("selected").build(),
                    Signal::builder("rename-requested")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("delete-requested").build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibrarySheet {}
    impl BinImpl for LibrarySheet {}

    impl LibrarySheet {
        fn setup_context_menu(&self) {
            let obj = self.obj();

            let builder =
                Builder::from_resource("/fi/sevonj/TheftMD/ui/library_sheet_context_menu.ui");
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

            let menu = SheetRenamePopover::default();
            menu.set_parent(&*obj);

            menu.connect_closure(
                "committed",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_popover: SheetRenamePopover, path: PathBuf| {
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

            obj.add_controller(drag_source.clone());
            let _ = self.drag_source.replace(Some(drag_source));
        }
    }
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;

use glib::Object;

use crate::data::SheetObject;

glib::wrapper! {
pub struct LibrarySheet(ObjectSubclass<imp::LibrarySheet>)
    @extends adw::Bin, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LibrarySheet {
    pub fn new(data: &SheetObject) -> Self {
        let this: Self = Object::builder().build();
        this.bind(data);
        this
    }

    pub fn path(&self) -> PathBuf {
        self.imp()
            .sheet_object
            .borrow()
            .as_ref()
            .expect("LibrarySheet data uninitialized")
            .path()
    }

    pub fn stem(&self) -> String {
        self.imp()
            .sheet_object
            .borrow()
            .as_ref()
            .expect("LibrarySheet data uninitialized")
            .stem()
    }

    pub fn rename(&self, path: PathBuf) {
        assert!(path.parent().is_some_and(|p| p.is_dir()));
        self.emit_by_name::<()>("rename-requested", &[&path]);
    }

    pub fn set_active(&self, is_active: bool) {
        self.imp().button.set_active(is_active);
    }

    fn bind(&self, data: &SheetObject) {
        self.imp().sheet_object.replace(Some(data.clone()));
        let path = data.path();
        self.imp()
            .rename_popover
            .borrow()
            .as_ref()
            .unwrap()
            .set_path(path);

        let title_label = self.imp().sheet_name_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let title_binding = data
            .bind_property("stem", &title_label, "label")
            .sync_create()
            .build();
        bindings.push(title_binding);
    }
}
