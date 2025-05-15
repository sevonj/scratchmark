//! Expandable folder widget for library browser
//!

mod imp {
    use std::cell::RefCell;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::Binding;
    use glib::subclass::Signal;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::Label;
    use gtk::{CompositeTemplate, TemplateChild};

    use crate::data::FolderObject;
    use crate::widgets::LibrarySheetButton;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/fi/sevonj/TheftMD/ui/library_folder.ui")]
    pub struct LibraryFolder {
        #[template_child]
        pub(super) title: TemplateChild<Label>,
        #[template_child]
        pub(super) subdir_vbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) content_vbox: TemplateChild<gtk::Box>,

        pub(super) folder_object: RefCell<Option<FolderObject>>,
        pub(super) bindings: RefCell<Vec<Binding>>,
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
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("sheet-clicked")
                        .param_types([LibrarySheetButton::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryFolder {}
    impl BinImpl for LibraryFolder {}
}

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

    pub fn refresh_content(&self) {
        let opt = self.imp().folder_object.borrow();
        let folder = opt.as_ref().expect("FolderObject not bound");

        let entries = folder.content();

        for entry in entries {
            let Ok(meta) = entry.metadata() else {
                return;
            };

            if meta.is_dir() {
                let data = FolderObject::new(entry.path());
                let folder = LibraryFolder::new(&data);
                self.imp().subdir_vbox.append(&folder);
                folder.refresh_content();

                let this = self;
                folder.connect_closure(
                    "sheet-clicked",
                    false,
                    closure_local!(
                        #[weak]
                        this,
                        move |_folder: LibraryFolder, button: LibrarySheetButton| {
                            this.emit_by_name::<()>("sheet-clicked", &[&button]);
                        }
                    ),
                );
            } else if meta.is_file() {
                let data = SheetObject::new(entry.path());
                let button = LibrarySheetButton::new(&data);
                self.imp().content_vbox.append(&button);

                let this = self;
                button.connect_clicked(clone!(
                    #[weak]
                    this,
                    move |button| {
                        this.emit_by_name::<()>("sheet-clicked", &[button]);
                    }
                ));
            }
        }
    }

    fn bind(&self, data: &FolderObject) {
        self.imp().folder_object.replace(Some(data.clone()));

        let title_label = self.imp().title.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let title_binding = data
            .bind_property("stem", &title_label, "label")
            .sync_create()
            .build();
        bindings.push(title_binding);
    }
}
