mod imp {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::Label;
    use gtk::ListBoxRow;
    use gtk::glib;
    use gtk::glib::clone;

    use adw::NavigationPage;
    use gtk::CompositeTemplate;
    use gtk::ListBox;
    use gtk::MenuButton;
    use gtk::gio::SimpleAction;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib::Properties;
    use gtk::glib::VariantTy;

    use crate::data::Document;
    use crate::data::Folder;
    use crate::data::SortMethod;
    use crate::widgets::library::document_preview_row::DocumentPreviewRow;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::FolderView)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library/folder_view.ui")]
    pub struct FolderView {
        #[template_child]
        listbox: TemplateChild<ListBox>,
        #[template_child]
        empty_placeholder: TemplateChild<Label>,
        #[template_child]
        sort_button: TemplateChild<MenuButton>,
        rows: RefCell<HashMap<PathBuf, DocumentPreviewRow>>,
        // pub(super) project: OnceLock<Project>,

        //  #[property(nullable, get, set)]
        //  open_document_path: RefCell<Option<PathBuf>>,
        //  #[property(nullable, get, set)]
        //  selected_item_path: RefCell<Option<PathBuf>>,
        //   previous_open_document: RefCell<Option<Document>>,

        // Props are bound one-way, from library to here.
        #[property(nullable, get, set)]
        open_document: RefCell<Option<PathBuf>>,
        #[property(nullable, get, set)]
        folder: RefCell<Option<Folder>>,
        #[property(get, set)]
        sort_method: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FolderView {
        const NAME: &'static str = "FolderView";
        type Type = super::FolderView;
        type ParentType = NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FolderView {
        fn constructed(&self) {
            let obj = self.obj();

            obj.connect_open_document_notify(move |obj| obj.imp().on_document_changed());
            obj.connect_folder_notify(move |obj| obj.imp().on_folder_changed());
            obj.connect_sort_method_notify(move |obj| obj.imp().on_sort_method_changed());

            self.listbox.connect_row_activated(move |_, row| {
                row.clone()
                    .downcast::<DocumentPreviewRow>()
                    .unwrap()
                    .on_click()
            });

            let actions = SimpleActionGroup::new();
            obj.insert_action_group("folder-view", Some(&actions));
            let action = SimpleAction::new_stateful(
                "sort-method",
                Some(VariantTy::STRING),
                &SortMethod::default().to_string().to_variant(),
            );
            action.connect_activate(clone!(
                #[weak]
                obj,
                move |action, param| {
                    let param = param.unwrap();
                    action.set_state(param);
                    obj.set_sort_method(param.get::<String>().unwrap());
                }
            ));
            actions.add_action(&action);

            self.parent_constructed();
        }
    }

    impl WidgetImpl for FolderView {}
    impl NavigationPageImpl for FolderView {}

    impl FolderView {
        fn on_document_changed(&self) {
            let obj = self.obj();
            let rows = self.rows.borrow();
            if let Some(path) = obj.open_document()
                && let Some(row) = rows.get(&path)
            {
                self.listbox.select_row(Some(row));
            } else {
                self.listbox.unselect_all();
            }
        }

        fn on_folder_changed(&self) {
            let obj = self.obj();
            self.clear();
            if let Some(folder) = self.folder.borrow().as_ref() {
                for doc in folder.documents().values() {
                    self.add_document(doc);
                }
                obj.set_title(&folder.name());
            } else {
                obj.set_title("");
            }
            self.refresh_empty_placeholder();
        }

        fn clear(&self) {
            self.rows.borrow_mut().clear();
            self.listbox.remove_all();
            self.refresh_empty_placeholder();
        }

        fn on_sort_method_changed(&self) {
            let obj = self.obj();
            let sort_method_str = obj.sort_method();
            let sort_method = SortMethod::try_from(sort_method_str.as_str()).unwrap();

            self.listbox.invalidate_sort();
            if sort_method.is_ascending() {
                self.sort_button.set_icon_name("library-sort-asc-symbolic");
            } else {
                self.sort_button.set_icon_name("library-sort-desc-symbolic");
            }

            fn to_doc(row: &ListBoxRow) -> Document {
                row.clone()
                    .downcast::<DocumentPreviewRow>()
                    .unwrap()
                    .document()
                    .clone()
            }

            fn sort_alphanumeric_asc(a: &ListBoxRow, b: &ListBoxRow) -> gtk::Ordering {
                to_doc(a)
                    .collation_key()
                    .cmp(to_doc(b).collation_key())
                    .into()
            }
            fn sort_alphanumeric_desc(a: &ListBoxRow, b: &ListBoxRow) -> gtk::Ordering {
                to_doc(b)
                    .collation_key()
                    .cmp(to_doc(a).collation_key())
                    .into()
            }
            fn sort_modified_asc(a: &ListBoxRow, b: &ListBoxRow) -> gtk::Ordering {
                to_doc(a).modified().cmp(&to_doc(b).modified()).into()
            }
            fn sort_modified_desc(a: &ListBoxRow, b: &ListBoxRow) -> gtk::Ordering {
                to_doc(b).modified().cmp(&to_doc(a).modified()).into()
            }

            self.listbox.set_sort_func(match sort_method {
                SortMethod::AlphanumericAsc => sort_alphanumeric_asc,
                SortMethod::AlphanumericDesc => sort_alphanumeric_desc,
                SortMethod::ModifiedAsc => sort_modified_asc,
                SortMethod::ModifiedDesc => sort_modified_desc,
            });
        }

        fn add_document(&self, document: &Document) {
            let is_selected = self.obj().open_document() == Some(document.path());

            let row = DocumentPreviewRow::new(document);
            self.listbox.insert(&row, 0);
            self.rows
                .borrow_mut()
                .insert(row.document().path(), row.clone());
            if is_selected {
                self.listbox.select_row(Some(&row));
            }
            self.refresh_empty_placeholder();
        }

        fn remove_document(&self, path: &Path) {
            if let Some(row) = self.rows.borrow_mut().remove(path) {
                self.listbox.remove(&row);
            }
            self.refresh_empty_placeholder();
        }

        fn refresh_empty_placeholder(&self) {
            self.empty_placeholder
                .set_visible(self.rows.borrow().is_empty());
        }
    }
}

use gtk::glib;

use glib::Object;

glib::wrapper! {
    pub struct FolderView(ObjectSubclass<imp::FolderView>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FolderView {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
