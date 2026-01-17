mod imp {
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::closure_local;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;

    use super::super::err_placeholder_item::ErrPlaceholderItem;
    use crate::data::Project;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/project_view.ui")]
    pub struct ProjectView {
        #[template_child]
        pub(super) project_root_vbox: TemplateChild<gtk::Box>,

        pub(super) project: OnceLock<Project>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectView {
        const NAME: &'static str = "ProjectView";
        type Type = super::ProjectView;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProjectView {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for ProjectView {}
    impl BinImpl for ProjectView {}

    impl ProjectView {
        pub(super) fn mark_invalid(&self) {
            let err_placeholder = ErrPlaceholderItem::new(&self.project.get().unwrap().root_path());
            err_placeholder.connect_closure(
                "close-project-requested",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: ErrPlaceholderItem| {
                        imp.project.get().unwrap().close();
                    }
                ),
            );
            self.project_root_vbox.append(&err_placeholder);
        }
    }
}

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::closure_local;
use gtk::prelude::*;

use glib::Object;

use crate::data::Project;

glib::wrapper! {
    pub struct ProjectView(ObjectSubclass<imp::ProjectView>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ProjectView {
    pub fn new(project: &Project) -> Self {
        let obj: Self = Object::builder().build();
        obj.bind(project);
        obj
    }

    pub fn project(&self) -> &Project {
        self.imp().project.get().unwrap()
    }

    fn bind(&self, project: &Project) {
        let imp = self.imp();
        imp.project.get_or_init(|| project.clone());
        project.connect_closure(
            "became-invalid",
            false,
            closure_local!(
                #[weak]
                imp,
                move |_: Project| {
                    imp.mark_invalid();
                }
            ),
        );
        let root_folder = project.root_folder();
        imp.project_root_vbox.append(&root_folder);
    }
}
