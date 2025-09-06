mod imp {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::glib::closure_local;
    use gtk::glib::subclass::Signal;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;
    use gtk::Image;
    use gtk::Revealer;
    use gtk::ToggleButton;

    use crate::data::ProjectState;
    use crate::util;
    use crate::widgets::ProjectSelectorEntry;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/project_selector.ui")]
    pub struct ProjectSelector {
        #[template_child]
        reveal_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        revealer: TemplateChild<Revealer>,
        #[template_child]
        expand_icon: TemplateChild<Image>,
        #[template_child]
        pub(super) projects_container: TemplateChild<gtk::Box>,
        pub(super) selected: RefCell<Option<PathBuf>>,
        pub(super) projects: RefCell<HashMap<PathBuf, ProjectSelectorEntry>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectSelector {
        const NAME: &'static str = "ProjectSelector";
        type Type = super::ProjectSelector;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProjectSelector {
        fn constructed(&self) {
            self.parent_constructed();

            let revealer: &Revealer = self.revealer.as_ref();
            self.reveal_toggle
                .bind_property("active", revealer, "reveal-child")
                .bidirectional()
                .sync_create()
                .build();

            self.reveal_toggle.connect_clicked(clone!(
                #[weak(rename_to = this)]
                self,
                move |toggle| {
                    if toggle.is_active() {
                        this.expand_icon.set_icon_name(Some("up-symbolic"));
                    } else {
                        this.expand_icon.set_icon_name(Some("down-symbolic"));
                    }
                }
            ));

            self.refresh();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("select-requested")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("remove-requested")
                        .param_types([PathBuf::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for ProjectSelector {}
    impl BinImpl for ProjectSelector {}

    impl ProjectSelector {
        fn refresh(&self) {
            let paths = ProjectState::list_projects();

            for path in paths {
                let entry = ProjectSelectorEntry::new(path);
                self.add_entry(entry);
            }
        }

        fn add_entry(&self, entry: ProjectSelectorEntry) {
            let obj = self.obj();

            entry.connect_closure(
                "select-clicked",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |entry: ProjectSelectorEntry| {
                        obj.emit_by_name::<()>("select-requested", &[&entry.path()]);
                    }
                ),
            );

            entry.connect_closure(
                "remove-clicked",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |entry: ProjectSelectorEntry| {
                        obj.emit_by_name::<()>("remove-requested", &[&entry.path()]);
                    }
                ),
            );

            self.projects_container.append(&entry);
            self.projects.borrow_mut().insert(entry.path(), entry);
        }
    }
}

use std::path::{Path, PathBuf};

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{
    glib::{self, property::PropertyGet},
    prelude::BoxExt,
};

glib::wrapper! {
    pub struct ProjectSelector(ObjectSubclass<imp::ProjectSelector>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ProjectSelector {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ProjectSelector {
    pub fn remove(&self, path: &Path) {
        let entry = self.imp().projects.borrow_mut().remove(path).unwrap();
        self.imp().projects_container.remove(&entry);
    }

    pub fn select(&self, path: &Path) {
        if let Some(selected) = self.imp().selected.borrow().as_ref() {
            self.imp()
                .projects
                .borrow()
                .get(selected)
                .unwrap()
                .set_active(false);
        }
        self.imp()
            .projects
            .borrow()
            .get(path)
            .unwrap()
            .set_active(true);
        self.imp().selected.replace(Some(path.to_path_buf()));
    }
}
