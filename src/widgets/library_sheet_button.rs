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
    use gtk::{Builder, CompositeTemplate, Label, PopoverMenu, TemplateChild};

    use crate::data::SheetObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_sheet_button.ui")]
    pub struct LibrarySheetButton {
        #[template_child]
        pub(super) sheet_name_label: TemplateChild<Label>,

        pub(super) sheet_object: RefCell<Option<SheetObject>>,
        pub(super) bindings: RefCell<Vec<Binding>>,

        context_menu_popover: RefCell<Option<PopoverMenu>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibrarySheetButton {
        const NAME: &'static str = "LibrarySheet";
        type Type = super::LibrarySheetButton;
        type ParentType = gtk::ToggleButton;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibrarySheetButton {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let builder = Builder::from_resource(
                "/fi/sevonj/TheftMD/ui/library_sheet_button_context_menu.ui",
            );
            let popover = builder
                .object::<MenuModel>("context-menu")
                .expect("LibrarySheetButton context-menu model failed");
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
            obj.insert_action_group("sheet", Some(&actions));

            let action = gio::SimpleAction::new("delete", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    obj.emit_by_name::<()>("delete-requested", &[]);
                }
            ));
            actions.add_action(&action);

            obj.connect_destroy(move |obj| {
                let popover = obj
                    .imp()
                    .context_menu_popover
                    .take()
                    .expect("LibrarySheetButton context menu uninitialized");
                popover.unparent();
            });
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("delete-requested").build()])
        }
    }

    impl WidgetImpl for LibrarySheetButton {}
    impl ButtonImpl for LibrarySheetButton {}
    impl ToggleButtonImpl for LibrarySheetButton {}
}

use std::path::PathBuf;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;

use crate::data::SheetObject;

glib::wrapper! {
    pub struct LibrarySheetButton(ObjectSubclass<imp::LibrarySheetButton>)
        @extends gtk::ToggleButton, gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl LibrarySheetButton {
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
            .expect("LibrarySheetButton data uninitialized")
            .path()
    }

    fn bind(&self, data: &SheetObject) {
        self.imp().sheet_object.replace(Some(data.clone()));

        let title_label = self.imp().sheet_name_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let title_binding = data
            .bind_property("stem", &title_label, "label")
            .sync_create()
            .build();
        bindings.push(title_binding);
    }
}
