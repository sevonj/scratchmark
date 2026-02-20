mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    use gtk::glib;
    use gtk::subclass::prelude::*;

    use gtk::glib::Properties;

    use super::SortMethod;
    use crate::data::ProjectItem;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ProjectSorter)]
    pub struct ProjectSorter {
        pub(super) sort_method: Cell<SortMethod>,
        pub(super) sort_cache: RefCell<HashMap<PathBuf, ProjectItem>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectSorter {
        const NAME: &'static str = "ProjectSorter";
        type Type = super::ProjectSorter;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectSorter {}

    impl ProjectSorter {
        pub(super) fn sort(&self, a: PathBuf, b: PathBuf) -> gtk::Ordering {
            let a_ancestors: Vec<_> = a.ancestors().collect();
            let b_ancestors: Vec<_> = b.ancestors().collect();
            let mut iter_a = a_ancestors.iter().rev();
            let mut iter_b = b_ancestors.iter().rev();

            let sort_cache = self.sort_cache.borrow_mut();

            let mut path_a;
            let mut path_b;
            loop {
                let path = iter_a.next().unwrap();
                if sort_cache.contains_key(*path) {
                    path_a = *path;
                    break;
                }
            }
            loop {
                let path = iter_b.next().unwrap();
                if sort_cache.contains_key(*path) {
                    path_b = *path;
                    break;
                }
            }

            loop {
                let Some(item_a) = sort_cache.get(path_a) else {
                    if sort_cache.get(path_b).is_none() {
                        return gtk::Ordering::Equal;
                    } else {
                        return gtk::Ordering::Smaller;
                    }
                };
                let Some(item_b) = sort_cache.get(path_b) else {
                    unreachable!();
                    // return gtk::Ordering::Larger;
                };

                if item_a.is_dir() == item_b.is_dir() {
                    match self.comp_items(item_a, item_b) {
                        gtk::Ordering::Equal => (),
                        ord => return ord,
                    };
                } else if item_a.is_dir() {
                    return gtk::Ordering::Smaller;
                } else {
                    return gtk::Ordering::Larger;
                }

                let Some(next_a) = iter_a.next() else {
                    if iter_b.next().is_none() {
                        return gtk::Ordering::Equal;
                    } else {
                        return gtk::Ordering::Smaller;
                    }
                };
                let Some(next_b) = iter_b.next() else {
                    return gtk::Ordering::Larger;
                };
                path_a = *next_a;
                path_b = *next_b;
            }
        }

        fn comp_items(&self, item_a: &ProjectItem, item_b: &ProjectItem) -> gtk::Ordering {
            match self.sort_method.get() {
                SortMethod::AlphanumericAsc => {
                    item_a.collation_key().cmp(item_b.collation_key()).into()
                }
                SortMethod::AlphanumericDesc => {
                    item_b.collation_key().cmp(item_a.collation_key()).into()
                }
                SortMethod::ModifiedAsc => match item_a.modified().cmp(&item_b.modified()) {
                    std::cmp::Ordering::Equal => {
                        item_a.collation_key().cmp(item_b.collation_key()).into()
                    }
                    not_equal => not_equal.into(),
                },
                SortMethod::ModifiedDesc => match item_b.modified().cmp(&item_a.modified()) {
                    std::cmp::Ordering::Equal => {
                        item_a.collation_key().cmp(item_b.collation_key()).into()
                    }
                    not_equal => not_equal.into(),
                },
            }
        }
    }
}

use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use gtk::glib;
use gtk::subclass::prelude::*;

use gtk::glib::Object;

use crate::data::ProjectItem;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SortMethod {
    #[default]
    AlphanumericAsc,
    AlphanumericDesc,
    ModifiedAsc,
    ModifiedDesc,
}

impl Display for SortMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortMethod::AlphanumericAsc => write!(f, "AlphanumericAsc"),
            SortMethod::AlphanumericDesc => write!(f, "AlphanumericDesc"),
            SortMethod::ModifiedAsc => write!(f, "ModifiedAsc"),
            SortMethod::ModifiedDesc => write!(f, "ModifiedDesc"),
        }
    }
}

impl TryFrom<&str> for SortMethod {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "AlphanumericAsc" => Ok(Self::AlphanumericAsc),
            "AlphanumericDesc" => Ok(Self::AlphanumericDesc),
            "ModifiedAsc" => Ok(Self::ModifiedAsc),
            "ModifiedDesc" => Ok(Self::ModifiedDesc),
            _ => Err(()),
        }
    }
}

impl SortMethod {
    pub fn is_ascending(&self) -> bool {
        match self {
            SortMethod::AlphanumericAsc | SortMethod::ModifiedAsc => true,
            SortMethod::AlphanumericDesc | SortMethod::ModifiedDesc => false,
        }
    }
}

glib::wrapper! {
    pub struct ProjectSorter(ObjectSubclass<imp::ProjectSorter>);
}

impl Default for ProjectSorter {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ProjectSorter {
    pub fn set_sort_method(&self, sort_method: SortMethod) {
        self.imp().sort_method.set(sort_method);
    }

    pub fn insert(&self, item: ProjectItem) {
        self.imp().sort_cache.borrow_mut().insert(item.path(), item);
    }

    pub fn remove(&self, path: &Path) {
        self.imp().sort_cache.borrow_mut().remove(path);
    }

    pub fn clear(&self) {
        self.imp().sort_cache.borrow_mut().clear();
    }

    pub fn sort(&self, a: PathBuf, b: PathBuf) -> gtk::Ordering {
        self.imp().sort(a, b)
    }
}
