use std::cmp::Ordering;
use std::path::Component;
use std::path::PathBuf;

use gtk::prelude::*;

use gtk::ListBoxRow;
use gtk::glib::CollationKey;

use crate::widgets::library::DocumentRow;
use crate::widgets::library::FolderRow;
use crate::widgets::library::item_create_row::ItemCreateRow;

#[derive(PartialEq, Eq)]
pub struct SortComponent<'a> {
    pub component: Component<'a>,
    pub is_dir: bool,
}

impl PartialOrd for SortComponent<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SortComponent<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.is_dir == other.is_dir {
            let a = CollationKey::from(self.component.as_os_str().to_string_lossy());
            let b = CollationKey::from(other.component.as_os_str().to_string_lossy());
            a.cmp(&b)
        } else if self.is_dir {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl<'a> SortComponent<'a> {
    pub fn new(component: Component<'a>, is_dir: bool) -> Self {
        Self { component, is_dir }
    }
}

pub fn sort_alphanumeric(a: &ListBoxRow, b: &ListBoxRow) -> gtk::Ordering {
    let (a_path, a_is_dir) = row_to_path(a).unwrap();
    let (b_path, b_is_dir) = row_to_path(b).unwrap();

    let mut a_components = a_path
        .parent()
        .unwrap()
        .components()
        .map(|component| SortComponent::new(component, true))
        .chain(
            a_path
                .components()
                .next_back()
                .map(|component| SortComponent::new(component, a_is_dir)),
        );

    let mut b_components = b_path
        .parent()
        .unwrap()
        .components()
        .map(|component| SortComponent::new(component, true))
        .chain(
            b_path
                .components()
                .next_back()
                .map(|component| SortComponent::new(component, b_is_dir)),
        );

    loop {
        let Some(a) = a_components.next() else {
            if b_components.next().is_none() {
                return gtk::Ordering::Equal;
            } else {
                return gtk::Ordering::Smaller;
            }
        };
        let Some(b) = b_components.next() else {
            return gtk::Ordering::Larger;
        };

        match a.cmp(&b) {
            Ordering::Equal => continue,
            ord => return ord.into(),
        }
    }
}

fn row_to_path(row: &ListBoxRow) -> Option<(PathBuf, bool)> {
    if let Ok(row) = row.clone().downcast::<DocumentRow>() {
        Some((row.document().path(), false))
    } else if let Ok(row) = row.clone().downcast::<FolderRow>() {
        Some((row.folder().path(), true))
    } else if let Ok(row) = row.clone().downcast::<ItemCreateRow>() {
        Some((row.parent_path().join("~"), row.is_dir())) // tilde should sort this as last semi reliably. TODO: refactor is_dir to handle it instead
    } else {
        None
    }
}
