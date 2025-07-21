//! Library browser is located in the left sidebar.
//!

mod imp {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::glib::closure_local;
    use gtk::prelude::*;

    use crate::widgets::LibraryFolder;
    use crate::widgets::LibrarySheet;
    use crate::widgets::library_project::LibraryProject;
    use gtk::CompositeTemplate;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library_browser.ui")]
    pub struct LibraryBrowser {
        #[template_child]
        pub(super) library_container: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) projects_container: TemplateChild<gtk::Box>,
        #[template_child]
        no_projects_status: TemplateChild<adw::Bin>,

        pub(super) selected_sheet: RefCell<Option<PathBuf>>,

        pub(super) projects: RefCell<Vec<LibraryProject>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryBrowser {
        const NAME: &'static str = "LibraryBrowser";
        type Type = super::LibraryBrowser;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibraryBrowser {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("sheet-selected")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-rename-requested")
                        .param_types([LibraryFolder::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("sheet-rename-requested")
                        .param_types([LibrarySheet::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-delete-requested")
                        .param_types([LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("sheet-delete-requested")
                        .param_types([LibrarySheet::static_type()])
                        .build(),
                    Signal::builder("folder-trash-requested")
                        .param_types([LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("sheet-trash-requested")
                        .param_types([LibrarySheet::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryBrowser {}
    impl BinImpl for LibraryBrowser {}

    impl LibraryBrowser {
        pub(super) fn refresh_content(&self) {
            let binding = self.projects.borrow();
            let projects: &Vec<LibraryProject> = binding.as_ref();
            for project in projects {
                project.refresh_content();
            }
        }

        pub(super) fn add_project(&self, project: LibraryProject) {
            let path = project.path();
            for project in self.projects.borrow_mut().iter() {
                if project.root_folder().path() == path {
                    return;
                }
            }

            project.connect_closure(
                "folder-added",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryProject, folder: LibraryFolder| {
                        this.connect_folder(folder);
                    }
                ),
            );
            project.connect_closure(
                "sheet-added",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryProject, sheet: LibrarySheet| {
                        this.connect_sheet(sheet);
                    }
                ),
            );

            self.projects_container.append(&project);
            self.projects.borrow_mut().push(project.clone());
            project.refresh_content();
            self.no_projects_status.set_visible(false);
        }

        fn connect_folder(&self, folder: LibraryFolder) {
            let obj = self.obj();

            folder.connect_closure(
                "rename-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |folder: LibraryFolder, new_path: PathBuf| {
                        obj.emit_by_name::<()>("folder-rename-requested", &[&folder, &new_path]);
                    }
                ),
            );

            folder.connect_closure(
                "sheet-created",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryFolder, path: PathBuf| {
                        obj.emit_by_name::<()>("sheet-selected", &[&path]);
                    }
                ),
            );

            folder.connect_closure(
                "folder-created",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryFolder, _path: PathBuf| {
                        this.refresh_content();
                    }
                ),
            );

            folder.connect_closure(
                "sheet-created",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryFolder, _path: PathBuf| {
                        this.refresh_content();
                    }
                ),
            );

            folder.connect_closure(
                "trash-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryFolder, folder: LibraryFolder| {
                        obj.emit_by_name::<()>("folder-trash-requested", &[&folder]);
                    }
                ),
            );

            folder.connect_closure(
                "delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryFolder, folder: LibraryFolder| {
                        obj.emit_by_name::<()>("folder-delete-requested", &[&folder]);
                    }
                ),
            );
        }

        fn connect_sheet(&self, sheet: LibrarySheet) {
            let obj = self.obj();

            sheet.connect_closure(
                "selected",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |sheet: LibrarySheet| {
                        sheet.set_active(false);
                        let path = sheet.path();
                        obj.emit_by_name::<()>("sheet-selected", &[&path]);
                    }
                ),
            );

            sheet.connect_closure(
                "duplicated",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibrarySheet| {
                        obj.refresh_content();
                    }
                ),
            );

            sheet.connect_closure(
                "rename-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |sheet: LibrarySheet, new_path: PathBuf| {
                        obj.emit_by_name::<()>("sheet-rename-requested", &[&sheet, &new_path]);
                    }
                ),
            );

            sheet.connect_closure(
                "trash-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |button: LibrarySheet| {
                        obj.emit_by_name::<()>("sheet-trash-requested", &[&button]);
                    }
                ),
            );

            sheet.connect_closure(
                "delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |button: LibrarySheet| {
                        obj.emit_by_name::<()>("sheet-delete-requested", &[&button]);
                    }
                ),
            );
        }
    }
}

use std::path::{Path, PathBuf};

use adw::subclass::prelude::*;
use gtk::glib;

use glib::Object;

use crate::widgets::LibraryProject;
use crate::widgets::LibrarySheet;

use super::LibraryFolder;

glib::wrapper! {
    pub struct LibraryBrowser(ObjectSubclass<imp::LibraryBrowser>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for LibraryBrowser {
    fn default() -> Self {
        let this: Self = Object::builder().build();
        this.imp().add_project(LibraryProject::new_appdata());
        this.refresh_content();
        this
    }
}

impl LibraryBrowser {
    pub fn project_paths(&self) -> Vec<String> {
        let mut paths = vec![];

        let binding = self.imp().projects.borrow();
        let projects: &Vec<LibraryProject> = binding.as_ref();
        for project in &projects[1..] {
            paths.push(project.root_folder().path().to_str().unwrap().to_owned());
        }
        paths
    }

    pub fn expanded_folder_paths(&self) -> Vec<String> {
        let mut paths = vec![];

        let binding = self.imp().projects.borrow();
        let projects: &Vec<LibraryProject> = binding.as_ref();
        for project in projects {
            paths.append(&mut project.expanded_folder_paths());
        }
        paths
    }

    pub fn get_folder(&self, path: &Path) -> Option<LibraryFolder> {
        let binding = self.imp().projects.borrow();
        let projects: &Vec<LibraryProject> = binding.as_ref();
        for project in projects {
            let opt = project.get_folder(path);
            if opt.is_some() {
                return opt;
            }
        }
        None
    }

    pub fn get_sheet(&self, path: &Path) -> Option<LibrarySheet> {
        let binding = self.imp().projects.borrow();
        let projects: &Vec<LibraryProject> = binding.as_ref();
        for project in projects {
            let opt = project.get_sheet(path);
            if opt.is_some() {
                return opt;
            }
        }
        None
    }

    pub fn add_project(&self, path: PathBuf) {
        self.imp().add_project(LibraryProject::new(path));
    }

    pub fn refresh_content(&self) {
        self.imp().refresh_content();
    }

    pub fn selected_sheet(&self) -> Option<PathBuf> {
        self.imp().selected_sheet.borrow().clone()
    }

    pub fn set_selected_sheet(&self, path: Option<PathBuf>) {
        if let Some(old_path) = self.imp().selected_sheet.borrow().as_ref() {
            if let Some(old_button) = self.get_sheet(old_path) {
                old_button.set_active(false);
            }
        }

        if let Some(path) = &path {
            if let Some(button) = self.get_sheet(path) {
                button.set_active(true);
            }
        };

        self.imp().selected_sheet.replace(path);
    }

    pub fn rename_selected_sheet(&self) {
        let Some(selected_path) = self.selected_sheet() else {
            return;
        };

        if let Some(sheet) = self.get_sheet(&selected_path) {
            sheet.prompt_rename();
        }
    }
}
