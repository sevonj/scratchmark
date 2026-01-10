mod folder_view;
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

    use super::FolderView;
    use super::ProjectView;
    use crate::data::DocumentObject;
    use crate::data::FolderObject;
    use crate::widgets::LibraryDocument;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::LibraryView)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/library_view.ui")]
    pub struct LibraryView {
        #[template_child]
        pub(super) projects_container: TemplateChild<gtk::Box>,

        pub(super) open_document: RefCell<Option<PathBuf>>,
        #[property(get, set)]
        selected_item_path: RefCell<PathBuf>,

        /// Cleared when found.
        #[property(nullable, get, set)]
        selected_item_from_last_session: RefCell<Option<PathBuf>>,
        pub(super) projects: RefCell<HashMap<PathBuf, ProjectView>>,

        #[property(get, set)]
        ignore_hidden_files: Cell<bool>,
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

            let drafts = ProjectView::new_draft_table();
            let drafts_path = drafts.root_path();
            self.load_project(drafts);
            self.select_item(drafts_path);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("document-selected")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-rename-requested")
                        .param_types([FolderObject::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("document-rename-requested")
                        .param_types([DocumentObject::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-delete-requested")
                        .param_types([FolderObject::static_type()])
                        .build(),
                    Signal::builder("document-delete-requested")
                        .param_types([DocumentObject::static_type()])
                        .build(),
                    Signal::builder("folder-trash-requested")
                        .param_types([FolderObject::static_type()])
                        .build(),
                    Signal::builder("document-trash-requested")
                        .param_types([DocumentObject::static_type()])
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
        pub(super) fn has_folder(&self, path: &Path) -> bool {
            for project in self.projects.borrow().deref().values() {
                if path.starts_with(project.path()) {
                    return project.has_folder(path);
                }
            }
            false
        }

        pub(super) fn has_document(&self, path: &Path) -> bool {
            for project in self.projects.borrow().deref().values() {
                if path.starts_with(project.path()) {
                    return project.has_document(path);
                }
            }
            false
        }

        pub(super) fn get_folder(&self, path: &Path) -> Option<FolderView> {
            for project in self.projects.borrow().deref().values() {
                if path.starts_with(project.path()) {
                    return project.get_folder(path);
                }
            }
            None
        }

        pub(super) fn get_document(&self, path: &Path) -> Option<LibraryDocument> {
            for project in self.projects.borrow().deref().values() {
                if path.starts_with(project.path()) {
                    return project.get_document(path);
                }
            }
            None
        }

        pub(super) fn refresh_content(&self) {
            for project in self.projects.borrow().deref().values() {
                project.refresh_content();
            }
            self.refresh_selection();
        }

        pub(super) fn load_project(&self, project: ProjectView) {
            let obj = self.obj();
            self.connect_folder(project.root_folder().folder_object());
            project.connect_closure(
                "folder-added",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: ProjectView, folder: FolderObject| {
                        this.connect_folder(&folder);
                    }
                ),
            );
            project.connect_closure(
                "document-added",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: ProjectView, document: LibraryDocument| {
                        this.connect_document(document.document_object());
                    }
                ),
            );
            project.connect_closure(
                "close-project-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |project: ProjectView| {
                        obj.emit_by_name::<()>("close-project-requested", &[&project.path()]);
                    }
                ),
            );

            self.projects_container.append(&project);
            self.projects
                .borrow_mut()
                .insert(project.path(), project.clone());

            obj.bind_property("ignore_hidden_files", &project, "ignore_hidden_files")
                .sync_create()
                .build();
        }

        pub(super) fn select_item(&self, path: PathBuf) {
            let obj = self.obj();

            if let Some(old_selection) = self.get_folder(&obj.selected_item_path()) {
                old_selection.folder_object().set_is_selected(false);
            } else if let Some(old_selection) = self.get_document(&obj.selected_item_path()) {
                old_selection.document_object().set_is_selected(false);
            }

            if let Some(new_selection) = self.get_folder(&path) {
                new_selection.folder_object().set_is_selected(true);
            } else if let Some(new_selection) = self.get_document(&path) {
                new_selection.document_object().set_is_selected(true);
            }

            obj.set_selected_item_path(path.clone());
        }

        fn connect_folder(&self, folder: &FolderObject) {
            let obj = self.obj();
            let path = folder.path();

            if obj.selected_item_from_last_session().as_ref() == Some(&path) {
                self.select_item(path);
                obj.set_selected_item_from_last_session(None::<PathBuf>);
            }

            folder.connect_closure(
                "selected",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |folder: FolderObject| {
                        this.select_item(folder.path());
                    }
                ),
            );

            folder.connect_closure(
                "rename-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |folder: FolderObject, new_path: PathBuf| {
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
                    move |_: FolderObject, path: PathBuf| {
                        obj.emit_by_name::<()>("document-selected", &[&path]);
                    }
                ),
            );

            folder.connect_closure(
                "subfolder-created",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: FolderObject, _path: PathBuf| {
                        this.refresh_content();
                    }
                ),
            );

            folder.connect_closure(
                "document-created",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: FolderObject, _path: PathBuf| {
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
                    move |folder: FolderObject| {
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
                    move |folder: FolderObject| {
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
                    move |_: FolderObject, msg: String| {
                        obj.emit_by_name::<()>("notify-err", &[&msg]);
                    }
                ),
            );
        }

        fn connect_document(&self, doc: &DocumentObject) {
            let obj = self.obj();

            let path = doc.path();
            let is_open = Some(&path) == obj.imp().open_document.borrow().as_ref();

            if is_open || obj.selected_item_from_last_session().as_ref() == Some(&path) {
                self.select_item(path);
                obj.set_selected_item_from_last_session(None::<PathBuf>);
            }

            if is_open {
                doc.set_is_open_in_editor(true);
            }

            doc.connect_closure(
                "selected",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |doc: DocumentObject| {
                        this.select_item(doc.path());
                        let path = doc.path();
                        this.obj().emit_by_name::<()>("document-selected", &[&path]);
                    }
                ),
            );

            doc.connect_closure(
                "duplicated",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_doc: DocumentObject| {
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
                    move |doc: DocumentObject, new_path: PathBuf| {
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
                    move |doc: DocumentObject| {
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
                    move |doc: DocumentObject| {
                        obj.emit_by_name::<()>("document-delete-requested", &[&doc]);
                    }
                ),
            );
        }

        /// Attempts to select a valid item if current selection path is bad
        pub(super) fn refresh_selection(&self) {
            let obj = self.obj();
            let selected_path = obj.selected_item_path();
            let selection_is_gone =
                !self.has_document(&selected_path) && !self.has_folder(&selected_path);
            if selection_is_gone {
                if let Some(ancestor) = self.find_existing_ancestor(&selected_path) {
                    self.select_item(ancestor);
                } else if let Some(first_project_root) = self.projects.borrow().keys().next() {
                    self.select_item(first_project_root.to_path_buf());
                }
            }
        }

        fn find_existing_ancestor(&self, item_path: &Path) -> Option<PathBuf> {
            for project in self.projects.borrow().deref().values() {
                let project_path = project.path();
                if item_path.starts_with(&project_path) {
                    let mut working_path = item_path.to_path_buf();
                    while working_path != project_path {
                        let parent = working_path.parent()?;
                        if project.has_folder(parent) {
                            return Some(parent.to_path_buf());
                        }
                        working_path = parent.to_path_buf();
                    }
                    break;
                }
            }
            None
        }
    }
}

use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;

use glib::Object;
use gtk::prelude::BoxExt;

use crate::widgets::LibraryDocument;
use folder_view::FolderView;
use project_view::ProjectView;

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
    pub fn open_project_paths(&self) -> Vec<String> {
        let mut paths = vec![];
        for project in self.imp().projects.borrow().deref().values() {
            if !project.is_builtin() {
                paths.push(project.path().to_str().unwrap().to_owned());
            }
        }
        paths
    }

    pub fn expanded_folder_paths(&self) -> Vec<String> {
        let mut paths = vec![];
        for project in self.imp().projects.borrow().deref().values() {
            paths.append(&mut project.expanded_folder_paths());
        }
        paths
    }

    pub fn expand_folder(&self, path: PathBuf) {
        for project in self.imp().projects.borrow().deref().values() {
            if path.starts_with(project.path()) {
                project.expand_folder(path);
                return;
            }
        }
    }

    pub fn get_folder(&self, path: &Path) -> Option<FolderView> {
        self.imp().get_folder(path)
    }

    pub fn get_document(&self, path: &Path) -> Option<LibraryDocument> {
        self.imp().get_document(path)
    }

    pub fn add_project(&self, path: PathBuf) {
        for project in self.imp().projects.borrow().deref().values() {
            let compare = project.path();
            if path.starts_with(&compare) || compare.starts_with(&path) {
                return;
            }
        }
        let project = ProjectView::new(path);
        self.imp().load_project(project.clone());
        project.refresh_content();
    }

    pub fn remove_project(&self, path: &Path) {
        let imp = self.imp();
        let project = imp.projects.borrow_mut().remove(path).unwrap();
        imp.projects_container.remove(&project);
        imp.refresh_selection();
    }

    pub fn refresh_content(&self) {
        self.imp().refresh_content();
    }

    pub fn open_document_path(&self) -> Option<PathBuf> {
        self.imp().open_document.borrow().clone()
    }

    pub fn set_open_document_path(&self, path: Option<PathBuf>) {
        if let Some(old) = self
            .open_document_path()
            .and_then(|path| self.get_document(&path))
        {
            old.document_object().set_is_open_in_editor(false);
        }
        if let Some(new) = path.as_ref().and_then(|path| self.get_document(path)) {
            new.document_object().set_is_open_in_editor(true);
        }
        self.imp().open_document.replace(path);
    }

    pub fn prompt_rename_selected(&self) {
        if let Some(dir) = self.get_folder(&self.selected_item_path()) {
            dir.prompt_rename();
        } else if let Some(doc) = self.get_document(&self.selected_item_path()) {
            doc.prompt_rename();
        }
    }
}
