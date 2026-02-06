mod document;
mod document_stats;
mod folder;
mod markdown_buffer;
mod project;
mod sort;

use std::{path::PathBuf, time::SystemTime};

pub use document::Document;
pub use document_stats::DocumentStats;
pub use folder::{Folder, FolderType};
pub use markdown_buffer::MarkdownBuffer;
pub use project::Project;
pub use sort::{ProjectSorter, SortMethod};

use gtk::glib::CollationKey;

#[derive(Debug, Clone)]
pub enum ProjectItem {
    Doc(Document),
    Dir(Folder),
}

impl ProjectItem {
    pub fn path(&self) -> PathBuf {
        match self {
            ProjectItem::Doc(doc) => doc.path(),
            ProjectItem::Dir(dir) => dir.path(),
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, ProjectItem::Dir(_))
    }

    pub fn modified(&self) -> SystemTime {
        match self {
            ProjectItem::Doc(doc) => doc.modified(),
            ProjectItem::Dir(dir) => dir.modified(),
        }
    }

    pub fn collation_key(&self) -> &CollationKey {
        match self {
            ProjectItem::Doc(doc) => doc.collation_key(),
            ProjectItem::Dir(dir) => dir.collation_key(),
        }
    }
}
