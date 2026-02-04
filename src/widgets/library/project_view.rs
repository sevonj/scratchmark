mod imp {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::glib::clone;
    use gtk::glib::closure_local;
    use gtk::glib::subclass::*;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;
    use gtk::ListBox;
    use gtk::ListBoxRow;

    use crate::data::Document;
    use crate::data::Folder;
    use crate::data::Project;
    use crate::widgets::library::DocumentRow;
    use crate::widgets::library::FolderRow;
    use crate::widgets::library::err_placeholder_row::ErrPlaceholderRow;
    use crate::widgets::library::item_create_row::ItemCreateRow;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::ProjectView)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/project_view.ui")]
    pub struct ProjectView {
        #[template_child]
        pub(super) project_vbox: TemplateChild<ListBox>,
        pub(super) project_items: RefCell<HashMap<PathBuf, ListBoxRow>>,
        pub(super) project: OnceLock<Project>,

        #[property(nullable, get, set)]
        open_document_path: RefCell<Option<PathBuf>>,
        #[property(nullable, get, set)]
        selected_item_path: RefCell<Option<PathBuf>>,
        previous_open_document: RefCell<Option<Document>>,

        expanded_folders_queue: RefCell<HashSet<PathBuf>>,
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

    #[glib::derived_properties]
    impl ObjectImpl for ProjectView {
        fn constructed(&self) {
            let obj = self.obj();

            self.project_vbox.set_focusable(false);
            self.project_vbox
                .set_sort_func(crate::widgets::library::sort::sort_alphanumeric);
            self.project_vbox.connect_row_activated(clone!(
                #[weak]
                obj,
                move |_vbox, row| {
                    if let Ok(folder_item) = row.clone().downcast::<FolderRow>() {
                        folder_item.on_click();
                        obj.set_selected_item_path(Some(folder_item.folder().path()));
                    } else if let Ok(document_item) = row.clone().downcast::<DocumentRow>() {
                        document_item.on_click();
                        obj.set_selected_item_path(Some(document_item.document().path()));
                    };
                }
            ));

            obj.connect_open_document_path_notify(move |obj| {
                let imp = obj.imp();
                if let Some(prev) = imp.previous_open_document.borrow().as_ref() {
                    prev.set_is_open_in_editor(false);
                }
                let new = obj
                    .open_document_path()
                    .and_then(|path| imp.document_item(&path));
                if let Some(row) = &new {
                    row.document().set_is_open_in_editor(true);
                    obj.imp().project_vbox.select_row(Some(row));
                }
                imp.previous_open_document
                    .replace(new.map(|row| row.document().clone()));
            });

            obj.connect_selected_item_path_notify(move |obj| {
                obj.imp().refresh_selection();
            });

            self.project_vbox.connect_selected_rows_changed(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.refresh_selection();
                }
            ));

            self.parent_constructed();
        }
    }

    impl WidgetImpl for ProjectView {}
    impl BinImpl for ProjectView {}

    impl ProjectView {
        pub(super) fn project(&self) -> &Project {
            self.project.get().unwrap()
        }

        pub(super) fn project_item(&self, path: &Path) -> Option<ListBoxRow> {
            self.project_items.borrow().get(path).cloned()
        }

        pub(super) fn document_item(&self, path: &Path) -> Option<DocumentRow> {
            self.project_item(path).and_downcast()
        }

        pub(super) fn folder_item(&self, path: &Path) -> Option<FolderRow> {
            self.project_item(path).and_downcast()
        }

        fn refresh_selection(&self) {
            let obj = self.obj();
            let Some(path) = obj.selected_item_path() else {
                self.project_vbox.unselect_all();
                return;
            };
            let Some(row) = self.project_item(&path) else {
                self.project_vbox.unselect_all();
                return;
            };
            if !row.is_selected() {
                self.project_vbox.select_row(Some(&row));
            }
        }

        fn add_document(&self, doc: &Document) {
            let obj = self.obj();
            let path = doc.path();
            if self.has_item(&path) || !self.is_item_visible(&path) {
                return;
            }
            let doc_item = DocumentRow::new(doc);
            self.project_vbox.insert(&doc_item, -1);
            self.project_vbox.invalidate_sort();
            self.project_items
                .borrow_mut()
                .insert(doc.path(), doc_item.clone().upcast());

            if obj.open_document_path().is_some_and(|open| open == path) {
                self.previous_open_document.replace(Some(doc.clone()));
            }
            if obj.selected_item_path().is_some_and(|sel| sel == path) {
                self.project_vbox.select_row(Some(&doc_item));
            }
        }

        fn add_folder(&self, folder: &Folder) {
            let obj = self.obj();
            let path = folder.path();
            if self.has_item(&path) || !self.is_item_visible(&path) {
                return;
            }
            let folder_row = FolderRow::new(folder);
            self.project_vbox.insert(&folder_row, -1);
            self.project_vbox.invalidate_sort();
            self.project_items
                .borrow_mut()
                .insert(path.clone(), folder_row.clone().upcast());

            folder_row.connect_is_expanded_notify(clone!(
                #[weak(rename_to = imp)]
                self,
                move |folder| {
                    if folder.is_expanded() {
                        for subfolder in folder.folder().subfolders().values() {
                            imp.add_folder(subfolder);
                        }
                        for document in folder.folder().documents().values() {
                            imp.add_document(document);
                        }
                    } else {
                        for path in folder.folder().subfolders().keys() {
                            imp.remove_item(path);
                        }
                        for path in folder.folder().documents().keys() {
                            imp.remove_item(path);
                        }
                    }
                }
            ));

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

            let expand_queued = self.expanded_folders_queue.borrow().contains(&path);
            let contains_open_document = obj.open_document_path().is_some_and(|open| open == path);
            let is_selected = obj.selected_item_path().is_some_and(|sel| sel == path);

            if contains_open_document || expand_queued || is_selected {
                self.expand_folder(&path);
            }
            if is_selected {
                self.project_vbox.select_row(Some(&folder_row));
            }
        }

        fn remove_item(&self, path: &Path) {
            let Some(item) = self.project_items.borrow_mut().remove(path) else {
                return;
            };
            self.project_vbox.remove(&item);

            if let Ok(folder) = item.downcast::<FolderRow>() {
                for path in folder.folder().subfolders().keys() {
                    self.remove_item(path);
                }
                for path in folder.folder().documents().keys() {
                    self.remove_item(path);
                }
            }
        }

        /// Expand all parents
        pub(super) fn make_visible(&self, path: &Path) {
            if !path.starts_with(self.project().path()) {
                return;
            }

            if !self.has_item(path) {
                if let Some(folder) = self.project().get_folder(path) {
                    self.add_folder(&folder);
                } else if let Some(document) = self.project().get_document(path) {
                    self.add_document(&document);
                }
            }
            self.expand_folder(path);
            if let Some(parent_path) = path.parent() {
                self.make_visible(parent_path);
            }
        }

        pub(super) fn expand_folder(&self, path: &Path) {
            if let Some(folder) = self.project_item(path).and_downcast::<FolderRow>() {
                folder.set_is_expanded(true);
                self.expanded_folders_queue.borrow_mut().remove(path);
            } else {
                self.expanded_folders_queue
                    .borrow_mut()
                    .insert(path.to_path_buf());
            }
        }

        fn has_item(&self, path: &Path) -> bool {
            self.project_items.borrow().contains_key(path)
        }

        fn is_item_visible(&self, path: &Path) -> bool {
            let components = path.parent().unwrap().components();
            let mut check_path = PathBuf::new();
            for component in components {
                check_path.push(component);

                if let Some(l) = self.project_items.borrow().get(&check_path)
                    && let Ok(f) = l.clone().downcast::<FolderRow>()
                    && !f.is_expanded()
                {
                    return false;
                }
            }
            true
        }

        pub(super) fn refresh_content(&self) {
            self.project().refresh_content();
            self.project_vbox.invalidate_sort();
        }

        fn mark_invalid(&self) {
            let err_placeholder: ErrPlaceholderRow = ErrPlaceholderRow::new(&self.project().path());
            err_placeholder.connect_closure(
                "close-project-requested",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: ErrPlaceholderRow| {
                        imp.project.get().unwrap().close();
                    }
                ),
            );
            self.project_items.borrow_mut().clear();
            self.project_vbox.remove_all();
            self.project_vbox.append(&err_placeholder);
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
                        imp.add_folder(&folder);
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
                        imp.add_document(&doc);
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
                self.add_folder(folder);
            }
            for doc in project.documents().values() {
                self.add_document(doc);
            }
        }

        pub(super) fn prompt_create_document(&self, parent_path: PathBuf) {
            let Some(parent) = self.project().get_folder(&parent_path) else {
                return;
            };
            let Some(parent_row) = self.folder_item(&parent_path) else {
                return;
            };
            parent_row.set_is_expanded(true);
            let item_create_row = ItemCreateRow::for_document(&parent);

            self.project_vbox.append(&item_create_row);
            self.project_vbox.invalidate_sort();

            item_create_row.connect_closure(
                "cancelled",
                true,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |item_create_row: ItemCreateRow| {
                        glib::idle_add_local_once(clone!(
                            #[weak]
                            imp,
                            move || {
                                imp.project_vbox.remove(&item_create_row);
                            }
                        ));
                    }
                ),
            );

            item_create_row.connect_closure(
                "committed",
                true,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    #[weak]
                    parent_row,
                    move |item_create_row: ItemCreateRow, name: PathBuf| {
                        let parent_folder = parent_row.folder();
                        if let Err(e) = parent_folder.create_document(name) {
                            parent_folder.notify_err(&e.to_string());
                        }
                        glib::idle_add_local_once(clone!(
                            #[weak]
                            imp,
                            move || {
                                imp.project_vbox.remove(&item_create_row);
                            }
                        ));
                    }
                ),
            );
        }

        pub(super) fn prompt_create_subfolder(&self, parent_path: PathBuf) {
            let Some(parent) = self.project().get_folder(&parent_path) else {
                return;
            };
            let Some(parent_row) = self.folder_item(&parent_path) else {
                return;
            };
            parent_row.set_is_expanded(true);
            let item_create_row = ItemCreateRow::for_folder(&parent);

            self.project_vbox.append(&item_create_row);
            self.project_vbox.invalidate_sort();

            item_create_row.connect_closure(
                "cancelled",
                true,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |item_create_row: ItemCreateRow| {
                        glib::idle_add_local_once(clone!(
                            #[weak]
                            imp,
                            move || {
                                imp.project_vbox.remove(&item_create_row);
                            }
                        ));
                    }
                ),
            );

            item_create_row.connect_closure(
                "committed",
                true,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    #[weak]
                    parent_row,
                    move |item_create_row: ItemCreateRow, name: PathBuf| {
                        let parent_folder = parent_row.folder();
                        if let Err(e) = parent_folder.create_subfolder(name) {
                            parent_folder.notify_err(&e.to_string());
                        }
                        glib::idle_add_local_once(clone!(
                            #[weak]
                            imp,
                            move || {
                                imp.project_vbox.remove(&item_create_row);
                            }
                        ));
                    }
                ),
            );
        }
    }
}

use std::path::Path;
use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;

use glib::Object;

use crate::data::Project;
use crate::widgets::library::DocumentRow;
use crate::widgets::library::FolderRow;

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
        self.imp().project()
    }

    pub fn document_item(&self, path: &Path) -> Option<DocumentRow> {
        self.imp().document_item(path)
    }

    pub fn folder_item(&self, path: &Path) -> Option<FolderRow> {
        self.imp().folder_item(path)
    }

    pub fn refresh_content(&self) {
        self.imp().refresh_content();
    }

    pub fn expanded_folder_paths(&self) -> Vec<String> {
        let mut paths = vec![];
        for (path, row) in self.imp().project_items.borrow().iter() {
            if !self.project().has_item(path) {
                continue;
            }
            if row
                .clone()
                .downcast::<FolderRow>()
                .is_ok_and(|folder_row| folder_row.is_expanded())
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
        self.imp().prompt_create_document(parent_path);
    }

    pub fn prompt_create_subfolder(&self, parent_path: PathBuf) {
        self.imp().prompt_create_subfolder(parent_path);
    }
}
