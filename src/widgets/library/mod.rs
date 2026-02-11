mod document_create_popover;
mod document_row;
mod err_placeholder_row;
mod folder_create_popover;
mod folder_row;
mod item_rename_popover;
mod project_view;

mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::ops::Deref;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::glib::closure_local;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;
    use gtk::FileDialog;
    use gtk::gio::Cancellable;
    use gtk::gio::SimpleAction;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib::Properties;

    use super::DocumentRow;
    use super::FolderRow;
    use super::ProjectView;
    use crate::data::Document;
    use crate::data::Folder;
    use crate::data::Project;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::LibraryView)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/library_view.ui")]
    pub struct LibraryView {
        #[template_child]
        pub(super) projects_container: TemplateChild<gtk::Box>,

        #[property(nullable, get, set)]
        open_document_path: RefCell<Option<PathBuf>>,
        #[property(nullable, get, set)]
        selected_item_path: RefCell<Option<PathBuf>>,

        pub(super) projects: RefCell<HashMap<PathBuf, ProjectView>>,

        #[property(get, set)]
        ignore_hidden_files: Cell<bool>,
        #[property(get, set)]
        sort_method: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryView {
        const NAME: &'static str = "LibraryView";
        type Type = super::LibraryView;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LibraryView {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            let actions = SimpleActionGroup::new();
            obj.insert_action_group("library", Some(&actions));

            let action = SimpleAction::new("project-add", None);
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |_action, _parameter| {
                    let dialog = FileDialog::builder().build();
                    dialog.select_folder(
                        obj.root().and_downcast_ref::<gtk::Window>(),
                        None::<&Cancellable>,
                        clone!(
                            #[weak]
                            obj,
                            move |result| {
                                if let Ok(file) = result
                                    && let Some(path) = file.path()
                                {
                                    obj.add_project(path);
                                }
                            }
                        ),
                    );
                }
            ));
            actions.add_action(&action);

            let drafts = ProjectView::new(&Project::new_draft_table());
            let drafts_path = drafts.project().path();
            self.load_project(drafts);
            obj.set_selected_item_path(Some(drafts_path));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("open-document")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-rename-requested")
                        .param_types([Folder::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("document-rename-requested")
                        .param_types([Document::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-delete-requested")
                        .param_types([Folder::static_type()])
                        .build(),
                    Signal::builder("document-delete-requested")
                        .param_types([Document::static_type()])
                        .build(),
                    Signal::builder("folder-trash-requested")
                        .param_types([Folder::static_type()])
                        .build(),
                    Signal::builder("document-trash-requested")
                        .param_types([Document::static_type()])
                        .build(),
                    Signal::builder("close-project-requested")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    // Error that should be toasted to the user
                    Signal::builder("notify-err")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryView {}
    impl BinImpl for LibraryView {}

    impl LibraryView {
        pub(super) fn folder_item(&self, path: &Path) -> Option<FolderRow> {
            for project in self.projects.borrow().deref().values() {
                if path.starts_with(project.project().path()) {
                    return project.folder_item(path);
                }
            }
            None
        }

        pub(super) fn document_item(&self, path: &Path) -> Option<DocumentRow> {
            for project in self.projects.borrow().deref().values() {
                if path.starts_with(project.project().path()) {
                    return project.document_item(path);
                }
            }
            None
        }

        pub(super) fn refresh_content(&self) {
            for project in self.projects.borrow().deref().values() {
                project.refresh_content();
            }
        }

        pub(super) fn add_project(&self, path: PathBuf) {
            for project in self.projects.borrow().deref().values() {
                let compare = project.project().path();
                if path.starts_with(&compare) || compare.starts_with(&path) {
                    return;
                }
            }
            self.load_project(ProjectView::new(&Project::new(path)).clone());
        }

        fn load_project(&self, project_view: ProjectView) {
            let obj = self.obj();
            let project = project_view.project();

            project.connect_closure(
                "close-project-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |project: Project| {
                        obj.emit_by_name::<()>("close-project-requested", &[&project.path()]);
                    }
                ),
            );

            project.connect_closure(
                "document-added",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: Project, document: Document| {
                        imp.connect_document(&document);
                    }
                ),
            );

            project.connect_closure(
                "folder-added",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: Project, folder: Folder| {
                        imp.connect_folder(&folder);
                    }
                ),
            );
            // root is created before we have a chance to connect the signal
            self.connect_folder(&project.get_folder(&project.path()).unwrap());

            self.projects_container.append(&project_view);
            self.projects
                .borrow_mut()
                .insert(project.path(), project_view.clone());

            obj.bind_property("ignore_hidden_files", project, "ignore_hidden_files")
                .sync_create()
                .build();

            obj.bind_property("open_document_path", &project_view, "open_document_path")
                .sync_create()
                .build();
            obj.bind_property("selected_item_path", &project_view, "selected_item_path")
                .sync_create()
                .bidirectional()
                .build();
            obj.bind_property("sort_method", &project_view, "sort_method")
                .sync_create()
                .build();

            project.refresh_content();
            obj.set_selected_item_path(Some(project.path()));
        }

        fn connect_folder(&self, folder: &Folder) {
            let obj = self.obj();

            folder.connect_closure(
                "rename-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |folder: Folder, new_path: PathBuf| {
                        obj.emit_by_name::<()>("folder-rename-requested", &[&folder, &new_path]);
                    }
                ),
            );

            folder.connect_closure(
                "document-created",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: Folder, path: PathBuf| {
                        obj.emit_by_name::<()>("open-document", &[&path]);
                        obj.set_selected_item_path(Some(path));
                    }
                ),
            );

            folder.connect_closure(
                "subfolder-created",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: Folder, path: PathBuf| {
                        obj.set_selected_item_path(Some(path));
                    }
                ),
            );

            folder.connect_closure(
                "trash-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |folder: Folder| {
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
                    move |folder: Folder| {
                        obj.emit_by_name::<()>("folder-delete-requested", &[&folder]);
                    }
                ),
            );

            folder.connect_closure(
                "notify-err",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: Folder, msg: String| {
                        obj.emit_by_name::<()>("notify-err", &[&msg]);
                    }
                ),
            );
        }

        fn connect_document(&self, doc: &Document) {
            let obj = self.obj();

            let path = doc.path();
            let is_open = Some(&path) == obj.imp().open_document_path.borrow().as_ref();

            if is_open {
                doc.set_is_open_in_editor(true);
            }

            doc.connect_closure(
                "open",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |doc: Document| {
                        let path = doc.path();
                        obj.emit_by_name::<()>("open-document", &[&path]);
                        obj.set_selected_item_path(Some(path));
                    }
                ),
            );

            doc.connect_closure(
                "duplicated",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_doc: Document| {
                        obj.refresh_content();
                    }
                ),
            );

            doc.connect_closure(
                "rename-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |doc: Document, new_path: PathBuf| {
                        obj.emit_by_name::<()>("document-rename-requested", &[&doc, &new_path]);
                    }
                ),
            );

            doc.connect_closure(
                "trash-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |doc: Document| {
                        obj.emit_by_name::<()>("document-trash-requested", &[&doc]);
                    }
                ),
            );

            doc.connect_closure(
                "delete-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |doc: Document| {
                        obj.emit_by_name::<()>("document-delete-requested", &[&doc]);
                    }
                ),
            );
        }
    }
}

use std::cell::Ref;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;

use glib::Object;
use gtk::gio::Cancellable;
use gtk::gio::File;
use gtk::gio::FileCopyFlags;

use document_row::DocumentRow;
use folder_row::FolderRow;
use project_view::ProjectView;

use crate::error::ScratchmarkError;
use crate::util::file_actions;

glib::wrapper! {
    pub struct LibraryView(ObjectSubclass<imp::LibraryView>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for LibraryView {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibraryView {
    pub fn open_projects(&self) -> Vec<String> {
        let mut paths = vec![];
        for project in self.imp().projects.borrow().deref().values() {
            if !project.project().is_drafts() {
                paths.push(project.project().path().to_str().unwrap().to_owned());
            }
        }
        paths
    }

    pub fn expanded_folders(&self) -> Vec<String> {
        let mut paths = vec![];
        for project in self.imp().projects.borrow().deref().values() {
            paths.append(&mut project.expanded_folder_paths());
        }
        paths
    }

    pub fn make_visible(&self, path: &Path) {
        for project_view in self.imp().projects.borrow().deref().values() {
            if path.starts_with(project_view.project().path()) {
                project_view.make_visible(path);
                return;
            }
        }
    }

    pub fn add_project(&self, path: PathBuf) {
        self.imp().add_project(path);
    }

    pub fn create_folder(&self, path: PathBuf) -> Result<(), ScratchmarkError> {
        let Some(parent) = path.parent().and_then(|path| self.imp().folder_item(path)) else {
            return Err(ScratchmarkError::FileCreateFail);
        };
        let Some(filename) = path.file_name() else {
            return Err(ScratchmarkError::FileCreateFail);
        };
        parent.folder().create_subfolder(filename)?;
        Ok(())
    }

    pub fn create_document(&self, path: PathBuf) -> Result<(), ScratchmarkError> {
        let Some(parent) = path.parent().and_then(|path| self.imp().folder_item(path)) else {
            return Err(ScratchmarkError::FileCreateFail);
        };
        let Some(filename) = path.file_name() else {
            return Err(ScratchmarkError::FileCreateFail);
        };
        parent.folder().create_document(filename)?;
        Ok(())
    }

    pub fn remove_project(&self, path: &Path) {
        let imp = self.imp();
        let project = imp.projects.borrow_mut().remove(path).unwrap();
        imp.projects_container.remove(&project);
    }

    pub fn refresh_content(&self) {
        self.imp().refresh_content();
    }

    pub fn prompt_rename_selected(&self) {
        let Some(path) = self.selected_item_path() else {
            return;
        };

        if let Some(folder) = self.imp().folder_item(&path) {
            folder.prompt_rename();
        } else if let Some(doc) = self.imp().document_item(&path) {
            doc.prompt_rename();
        }
    }

    pub fn prompt_create_document(&self) {
        let Some(selected) = self.selected_item_path() else {
            return;
        };
        for project_view in self.projects().deref().values() {
            if selected.starts_with(project_view.project().path()) {
                if project_view.document_item(&selected).is_some() {
                    project_view.prompt_create_document(selected.parent().unwrap().to_path_buf());
                } else {
                    project_view.prompt_create_document(selected);
                }
                return;
            }
        }
    }

    pub fn prompt_create_subfolder(&self) {
        let Some(selected) = self.selected_item_path() else {
            return;
        };
        for project_view in self.projects().deref().values() {
            if selected.starts_with(project_view.project().path()) {
                if project_view.document_item(&selected).is_some() {
                    project_view.prompt_create_subfolder(selected.parent().unwrap().to_path_buf());
                } else {
                    project_view.prompt_create_subfolder(selected);
                }
                return;
            }
        }
    }

    pub fn move_item(&self, old_path: PathBuf, new_path: PathBuf) -> Result<(), ScratchmarkError> {
        let new_file = File::for_path(&new_path);
        if let Err(e) = File::for_path(&old_path).move_(
            &new_file,
            FileCopyFlags::NONE,
            None::<&Cancellable>,
            None,
        ) {
            println!("{e}");
            if old_path.is_dir() {
                file_actions::move_folder(&old_path, &new_path)?;
            } else {
                return Err(ScratchmarkError::ItemMoveFail);
            }
        }

        if let Some(selected_item_path) = self.selected_item_path() {
            if selected_item_path == old_path {
                self.set_selected_item_path(Some(new_path));
            } else if selected_item_path.starts_with(&old_path) {
                let relative = selected_item_path.strip_prefix(old_path).unwrap();
                let new_selected_path = new_path.join(relative);
                self.set_selected_item_path(Some(new_selected_path));
            }
        }

        self.refresh_content();
        Ok(())
    }

    fn projects(&self) -> Ref<'_, HashMap<PathBuf, ProjectView>> {
        self.imp().projects.borrow()
    }
}
