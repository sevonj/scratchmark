mod imp {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::ListBox;
    use gtk::ListBoxRow;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::glib::clone;

    use super::ProjectRow;
    use crate::data::ProjectSorter;
    use crate::data::SortMethod;
    use crate::widgets::library::document_row::DocumentRow;
    use crate::widgets::library::folder_row::FolderRow;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ProjectListBox)]
    pub struct ProjectListBox {
        pub(super) listbox: ListBox,
        pub(super) rows: RefCell<HashMap<PathBuf, ProjectRow>>,
        pub(super) sorter: ProjectSorter,

        #[property(get, set)]
        sort_method: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectListBox {
        const NAME: &'static str = "ProjectListBox";
        type Type = super::ProjectListBox;
        type ParentType = adw::Bin;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectListBox {
        fn constructed(&self) {
            let obj = self.obj();

            obj.set_child(Some(&self.listbox));
            self.listbox.set_focusable(false);
            self.listbox.add_css_class("navigation-sidebar");

            let sorter = self.sorter.clone();
            obj.connect_sort_method_notify(move |obj| obj.imp().refresh_sort_method());

            self.listbox.set_sort_func(clone!(
                #[strong]
                sorter,
                move |a, b| {
                    fn row_to_path(row: &ListBoxRow) -> Option<PathBuf> {
                        if let Ok(row) = row.clone().downcast::<DocumentRow>() {
                            Some(row.document().path())
                        } else if let Ok(row) = row.clone().downcast::<FolderRow>() {
                            Some(row.folder().path())
                        } else {
                            None
                        }
                    }
                    sorter.sort(row_to_path(a).unwrap(), row_to_path(b).unwrap())
                }
            ));

            self.listbox.connect_row_activated(move |_vbox, row| {
                if let Ok(folder_item) = row.clone().downcast::<FolderRow>() {
                    folder_item.on_click();
                } else if let Ok(document_item) = row.clone().downcast::<DocumentRow>() {
                    document_item.on_click();
                };
            });

            self.parent_constructed();
        }
    }

    impl WidgetImpl for ProjectListBox {}
    impl BinImpl for ProjectListBox {}

    impl ProjectListBox {
        fn refresh_sort_method(&self) {
            let sort_method_str = self.obj().sort_method();
            if let Ok(sort_method) = SortMethod::try_from(sort_method_str.as_str()) {
                self.sorter.set_sort_method(sort_method);
                self.listbox.invalidate_sort();
            };
        }
    }
}

use std::cell::Ref;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::ListBoxRow;
use gtk::glib;
use gtk::glib::Object;

use crate::data::ProjectItem;
use crate::widgets::library::document_row::DocumentRow;
use crate::widgets::library::folder_row::FolderRow;

glib::wrapper! {
    pub struct ProjectListBox(ObjectSubclass<imp::ProjectListBox>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ProjectListBox {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ProjectListBox {
    pub fn get(&self, path: &Path) -> Option<ProjectRow> {
        self.imp().rows.borrow().get(path).cloned()
    }

    pub fn has(&self, path: &Path) -> bool {
        self.imp().rows.borrow().contains_key(path)
    }

    pub fn rows(&self) -> Ref<'_, HashMap<PathBuf, ProjectRow>> {
        self.imp().rows.borrow()
    }

    pub fn insert(&self, row: ProjectRow) {
        self.imp().sorter.insert(match &row {
            ProjectRow::Doc(row) => ProjectItem::Doc(row.document().clone()),
            ProjectRow::Dir(row) => ProjectItem::Dir(row.folder().clone()),
        });
        self.imp().listbox.insert(&row.to_list_box_row(), 0);
        self.imp().rows.borrow_mut().insert(row.path(), row);
    }

    pub fn remove(&self, path: &Path) -> Option<ProjectRow> {
        let opt = self.imp().rows.borrow_mut().remove(path);
        if let Some(row) = &opt {
            self.imp().listbox.remove(&row.to_list_box_row());
            self.imp().sorter.remove(path);
        }
        opt
    }

    pub fn clear(&self) {
        self.imp().listbox.remove_all();
        self.imp().rows.borrow_mut().clear();
        self.imp().sorter.clear();
    }

    pub fn select_row(&self, path: &Path) {
        self.imp()
            .listbox
            .select_row(self.get(path).map(|v| v.to_list_box_row()).as_ref());
    }

    pub fn unselect_all(&self) {
        self.imp().listbox.unselect_all();
    }

    pub fn invalidate_sort(&self) {
        self.imp().listbox.invalidate_sort();
    }
}

#[derive(Debug, Clone)]
pub enum ProjectRow {
    Doc(DocumentRow),
    Dir(FolderRow),
}

impl ProjectRow {
    pub fn is_selected(&self) -> bool {
        match self {
            ProjectRow::Doc(document_row) => document_row.is_selected(),
            ProjectRow::Dir(folder_row) => folder_row.is_selected(),
        }
    }

    pub fn path(&self) -> PathBuf {
        match self {
            ProjectRow::Doc(document_row) => document_row.document().path(),
            ProjectRow::Dir(folder_row) => folder_row.folder().path(),
        }
    }

    pub fn to_list_box_row(&self) -> ListBoxRow {
        match self {
            ProjectRow::Doc(document_row) => document_row.clone().upcast(),
            ProjectRow::Dir(folder_row) => folder_row.clone().upcast(),
        }
    }
}
