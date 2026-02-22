mod imp {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::glib::closure_local;

    use gtk::glib::Properties;

    use crate::data::Document;
    use crate::data::Folder;
    use crate::data::Project;
    use crate::data::ProjectItem;
    use crate::widgets::library::document_row::DocumentRow;
    use crate::widgets::library::folder_row::FolderRow;
    use crate::widgets::library::project_err_placeholder::ProjectErrPlaceholder;
    use crate::widgets::library::project_list_box::ProjectListBox;
    use crate::widgets::library::project_list_box::ProjectRow;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ProjectView)]
    pub struct ProjectView {
        pub(super) listbox: ProjectListBox,
        pub(super) project: OnceLock<Project>,

        #[property(nullable, get, set)]
        open_document_path: RefCell<Option<PathBuf>>,
        #[property(nullable, get, set)]
        selected_item_path: RefCell<Option<PathBuf>>,
        previous_open_document: RefCell<Option<Document>>,
        expanded_folders_queue: RefCell<HashSet<PathBuf>>,
        #[property(get, set)]
        sort_method: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectView {
        const NAME: &'static str = "ProjectView";
        type Type = super::ProjectView;
        type ParentType = adw::Bin;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectView {
        fn constructed(&self) {
            let obj = self.obj();

            obj.set_child(Some(&self.listbox));
            obj.bind_property("sort_method", &self.listbox, "sort_method")
                .sync_create()
                .build();

            obj.connect_open_document_path_notify(move |obj| obj.imp().on_open_document_changed());
            obj.connect_selected_item_path_notify(move |obj| obj.imp().refresh_listbox_selection());

            self.parent_constructed();
        }
    }

    impl WidgetImpl for ProjectView {}
    impl BinImpl for ProjectView {}

    impl ProjectView {
        pub(super) fn document_item(&self, path: &Path) -> Option<DocumentRow> {
            self.listbox.get(path).and_then(|item| match item {
                ProjectRow::Doc(document_row) => Some(document_row),
                _ => None,
            })
        }

        pub(super) fn folder_item(&self, path: &Path) -> Option<FolderRow> {
            self.listbox.get(path).and_then(|item| match item {
                ProjectRow::Dir(folder_row) => Some(folder_row),
                _ => None,
            })
        }

        fn refresh_listbox_selection(&self) {
            let obj = self.obj();
            if let Some(path) = obj.selected_item_path()
                && let Some(item) = self.listbox.get(&path)
            {
                if !item.is_selected() {
                    self.listbox.select_row(&path);
                }
            } else {
                self.listbox.unselect_all();
            }
        }

        fn insert_item(&self, item: ProjectItem) {
            let obj = self.obj();
            let path = item.path();
            if self.listbox.has(&path) || !self.is_item_visible(&path) {
                return;
            }

            let project_row = match &item {
                ProjectItem::Doc(doc) => {
                    let document_row = DocumentRow::new(doc);
                    self.connect_document_row(&document_row);
                    ProjectRow::Doc(document_row)
                }
                ProjectItem::Dir(dir) => {
                    let folder_row = FolderRow::new(dir);
                    self.connect_folder_row(&folder_row);
                    ProjectRow::Dir(folder_row)
                }
            };

            self.listbox.insert(project_row);

            let is_selected = obj.selected_item_path().is_some_and(|sel| sel == path);

            if is_selected {
                self.listbox.select_row(&path);
            }

            match item {
                ProjectItem::Doc(doc) => {
                    if obj.open_document_path().is_some_and(|open| open == path) {
                        self.previous_open_document.replace(Some(doc.clone()));
                    }
                }
                ProjectItem::Dir(folder) => {
                    let expand_queued = self.expanded_folders_queue.borrow().contains(&path);
                    let contains_open_document =
                        obj.open_document_path().is_some_and(|open| open == path);

                    if contains_open_document || expand_queued || is_selected {
                        self.expand_folder(&path);
                    }
                    if folder.is_root() {
                        folder.connect_closure(
                            "metadata-changed",
                            true,
                            closure_local!(
                                #[weak(rename_to = imp)]
                                self,
                                move |_: Folder| {
                                    imp.listbox.invalidate_sort();
                                }
                            ),
                        );
                    }
                }
            }
        }

        fn connect_document_row(&self, document_row: &DocumentRow) {
            document_row.connect_closure(
                "needs-attention",
                true,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |document_row: DocumentRow| {
                        imp.obj()
                            .set_selected_item_path(Some(document_row.document().path()));
                    }
                ),
            );
        }

        fn connect_folder_row(&self, folder_row: &FolderRow) {
            let obj = self.obj();
            let folder = folder_row.folder();
            folder_row.connect_is_expanded_notify(clone!(
                #[weak(rename_to = imp)]
                self,
                move |folder_row| {
                    if folder_row.is_expanded() {
                        for dir in folder_row.folder().subfolders().values() {
                            imp.insert_item(ProjectItem::Dir(dir.clone()));
                        }
                        for doc in folder_row.folder().documents().values() {
                            imp.insert_item(ProjectItem::Doc(doc.clone()));
                        }
                    } else {
                        for path in folder_row.folder().subfolders().keys() {
                            imp.remove_item(path);
                        }
                        for path in folder_row.folder().documents().keys() {
                            imp.remove_item(path);
                        }
                    }
                }
            ));

            folder_row.connect_closure(
                "needs-attention",
                true,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |folder_row: FolderRow| {
                        imp.obj()
                            .set_selected_item_path(Some(folder_row.folder().path()));
                    }
                ),
            );

            folder_row.connect_closure(
                "prompt-create-subfolder",
                true,
                closure_local!(
                    #[weak]
                    obj,
                    move |folder_row: FolderRow| {
                        obj.prompt_create_subfolder(folder_row.folder().path());
                    }
                ),
            );

            folder_row.connect_closure(
                "prompt-create-document",
                true,
                closure_local!(
                    #[weak]
                    obj,
                    move |folder_row: FolderRow| {
                        obj.prompt_create_document(folder_row.folder().path());
                    }
                ),
            );

            folder.connect_closure(
                "document-created",
                true,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: Folder, path: PathBuf| {
                        obj.make_visible(&path);
                    }
                ),
            );
        }

        fn remove_item(&self, path: &Path) {
            let Some(item) = self.listbox.remove(path) else {
                return;
            };

            if let ProjectRow::Dir(folder_row) = item {
                for path in folder_row.folder().subfolders().keys() {
                    self.remove_item(path);
                }
                for path in folder_row.folder().documents().keys() {
                    self.remove_item(path);
                }
            }
        }

        /// Expand all parents
        pub(super) fn make_visible(&self, path: &Path) {
            if !path.starts_with(self.project.get().unwrap().path()) {
                return;
            }

            if !self.listbox.has(path) {
                if let Some(folder) = self.project.get().unwrap().get_folder(path) {
                    self.insert_item(ProjectItem::Dir(folder));
                } else if let Some(document) = self.project.get().unwrap().get_document(path) {
                    self.insert_item(ProjectItem::Doc(document));
                }
            }
            self.expand_folder(path);
            if let Some(parent_path) = path.parent() {
                self.make_visible(parent_path);
            }
        }

        pub(super) fn expand_folder(&self, path: &Path) {
            if let Some(ProjectRow::Dir(folder_view)) = self.listbox.get(path) {
                folder_view.set_is_expanded(true);
                self.expanded_folders_queue.borrow_mut().remove(path);
            } else {
                self.expanded_folders_queue
                    .borrow_mut()
                    .insert(path.to_path_buf());
            }
        }

        fn is_item_visible(&self, path: &Path) -> bool {
            let components = path.parent().unwrap().components();
            let mut check_path = PathBuf::new();
            for component in components {
                check_path.push(component);

                if let Some(item) = self.listbox.get(&check_path)
                    && let ProjectRow::Dir(folder_row) = item
                    && !folder_row.is_expanded()
                {
                    return false;
                }
            }
            true
        }

        fn mark_invalid(&self) {
            let err_placeholder: ProjectErrPlaceholder =
                ProjectErrPlaceholder::new(&self.project.get().unwrap().path());
            err_placeholder.connect_closure(
                "close-project-requested",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: ProjectErrPlaceholder| {
                        imp.project.get().unwrap().close();
                    }
                ),
            );
            self.listbox.clear();
            self.obj().set_child(Some(&err_placeholder));
        }

        pub(super) fn bind(&self, project: &Project) {
            self.project.get_or_init(|| project.clone());
            project.connect_closure(
                "became-invalid",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: Project| {
                        imp.mark_invalid();
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
                        imp.insert_item(ProjectItem::Dir(folder));
                    }
                ),
            );
            project.connect_closure(
                "document-added",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: Project, doc: Document| {
                        imp.insert_item(ProjectItem::Doc(doc));
                    }
                ),
            );
            project.connect_closure(
                "item-removed",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: Project, path: PathBuf| {
                        imp.remove_item(&path);
                    }
                ),
            );

            for folder in project.folders().values() {
                self.insert_item(ProjectItem::Dir(folder.clone()));
            }
            for doc in project.documents().values() {
                self.insert_item(ProjectItem::Doc(doc.clone()));
            }
        }

        fn on_open_document_changed(&self) {
            if let Some(prev) = self.previous_open_document.borrow_mut().take() {
                prev.set_is_open_in_editor(false);
            }

            if let Some(path) = self.obj().open_document_path()
                && let Some(row) = self.document_item(&path)
            {
                let doc = row.document().clone();
                doc.set_is_open_in_editor(true);
                self.listbox.select_row(&path);
                self.previous_open_document.replace(Some(doc));
            } else {
                self.listbox.unselect_all();
            }
        }
    }
}

use std::path::Path;
use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;

use glib::Object;

use crate::data::Project;
use crate::widgets::library::document_row::DocumentRow;
use crate::widgets::library::folder_row::FolderRow;
use crate::widgets::library::project_list_box::ProjectRow;

glib::wrapper! {
    pub struct ProjectView(ObjectSubclass<imp::ProjectView>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ProjectView {
    pub fn new(project: &Project) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        imp.bind(project);
        obj
    }

    pub fn project(&self) -> &Project {
        self.imp().project.get().unwrap()
    }

    pub fn document_item(&self, path: &Path) -> Option<DocumentRow> {
        self.imp().document_item(path)
    }

    pub fn folder_item(&self, path: &Path) -> Option<FolderRow> {
        self.imp().folder_item(path)
    }

    pub fn refresh_content(&self) {
        self.project().refresh_content();
    }

    pub fn expanded_folder_paths(&self) -> Vec<String> {
        let mut paths = vec![];
        for (path, item) in self.imp().listbox.rows().iter() {
            if !self.project().has_item(path) {
                continue;
            }
            if let ProjectRow::Dir(folder_row) = item
                && folder_row.is_expanded()
            {
                paths.push(path.to_str().unwrap().to_owned());
            }
        }
        paths
    }

    pub fn make_visible(&self, path: &Path) {
        self.imp().make_visible(path);
    }

    pub fn prompt_create_document(&self, parent_path: PathBuf) {
        if let Some(parent) = self.folder_item(&parent_path) {
            parent.prompt_create_document();
        }
    }

    pub fn prompt_create_subfolder(&self, parent_path: PathBuf) {
        if let Some(parent) = self.folder_item(&parent_path) {
            parent.prompt_create_folder();
        }
    }
}
