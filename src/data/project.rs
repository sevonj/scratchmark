mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::collections::VecDeque;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use std::time::Duration;
    use std::time::SystemTime;

    use adw::subclass::prelude::*;
    use async_channel::Receiver;
    use async_channel::Sender;
    use glib::closure_local;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::glib::Properties;
    use gtk::glib::clone;
    use gtk::glib::timeout_add_local;
    use gtk::prelude::*;

    use gtk::glib::MainContext;

    use crate::data::Document;
    use crate::data::Folder;

    #[derive(Debug)]
    enum CrawlMsg {
        FoundDir {
            path: PathBuf,
            depth: u32,
            modified: SystemTime,
        },
        FoundFile {
            path: PathBuf,
            depth: u32,
            modified: SystemTime,
        },
        Done,
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Project)]
    pub struct Project {
        pub(super) path: OnceLock<PathBuf>,
        /// Project folder is inaccessible or deleted
        is_invalid: Cell<bool>,
        crawler_rx: RefCell<Option<Receiver<CrawlMsg>>>,
        crawler_tx: RefCell<Option<Sender<CrawlMsg>>>,

        pub(super) folders: RefCell<HashMap<PathBuf, super::Folder>>,
        pub(super) documents: RefCell<HashMap<PathBuf, Document>>,

        #[property(get, set)]
        ignore_hidden_files: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Project {
        const NAME: &'static str = "Project";
        type Type = super::Project;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Project {
        fn constructed(&self) {
            let obj = self.obj();

            obj.connect_ignore_hidden_files_notify(move |obj| {
                obj.refresh_content();
            });

            let (sender, receiver) = async_channel::unbounded();
            self.crawler_tx.replace(Some(sender));
            self.crawler_rx.replace(Some(receiver));

            timeout_add_local(
                Duration::from_millis(100),
                clone!(
                    #[strong]
                    obj,
                    move || {
                        let imp = obj.imp();
                        let mut bind = imp.crawler_rx.borrow_mut();
                        let receiver = bind.as_mut().unwrap();
                        while let Ok(entry) = receiver.try_recv() {
                            match entry {
                                CrawlMsg::FoundDir {
                                    path,
                                    depth,
                                    modified,
                                } => {
                                    imp.add_folder(Folder::new_subfolder(path, depth, modified));
                                }
                                CrawlMsg::FoundFile {
                                    path,
                                    depth,
                                    modified,
                                } => {
                                    imp.add_document(Document::new(path, depth, modified));
                                }
                                CrawlMsg::Done => {
                                    imp.prune();
                                }
                            }
                        }
                        glib::ControlFlow::Continue
                    }
                ),
            );
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("folder-added")
                        .param_types([Folder::static_type()])
                        .build(),
                    Signal::builder("document-added")
                        .param_types([Document::static_type()])
                        .build(),
                    Signal::builder("item-removed")
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
                    Signal::builder("close-project-requested").build(),
                    Signal::builder("became-invalid").build(),
                ]
            })
        }
    }

    impl WidgetImpl for Project {}
    impl BinImpl for Project {}

    impl Project {
        pub(super) fn refresh_content(&self) {
            if self.is_invalid.get() {
                return;
            }
            if !self.path().is_dir() {
                self.mark_invalid();
                return;
            }

            let ignore_hidden = self.ignore_hidden_files.get();
            let sender = self.crawler_tx.borrow().as_ref().unwrap().clone();
            let root_path = self.path().to_path_buf();

            MainContext::default().spawn_local(async move {
                let mut search_stack: VecDeque<(PathBuf, u32)> = VecDeque::from([(root_path, 1)]);

                loop {
                    let Some((folder, depth)) = search_stack.pop_front() else {
                        break;
                    };
                    let Ok(entries) = folder.read_dir() else {
                        continue;
                    };
                    for entry in entries {
                        let Ok(entry) = entry else {
                            continue;
                        };
                        let Ok(metadata) = entry.metadata() else {
                            continue;
                        };
                        if ignore_hidden
                            && entry.file_name().as_os_str().as_encoded_bytes()[0] == b'.'
                        {
                            continue;
                        }
                        let path = entry.path();
                        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

                        if metadata.is_dir() {
                            search_stack.push_back((path.clone(), depth + 1));
                            sender
                                .send(CrawlMsg::FoundDir {
                                    path,
                                    depth,
                                    modified,
                                })
                                .await
                                .unwrap();
                        } else {
                            if !path
                                .extension()
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                            {
                                continue;
                            }

                            sender
                                .send(CrawlMsg::FoundFile {
                                    path,
                                    depth,
                                    modified,
                                })
                                .await
                                .unwrap();
                        }
                    }
                }
                sender.send(CrawlMsg::Done).await.unwrap();
            });
        }

        pub(super) fn path(&self) -> &Path {
            self.path.get().unwrap()
        }

        fn mark_invalid(&self) {
            self.is_invalid.replace(true);
            self.obj().emit_by_name::<()>("became-invalid", &[]);
        }

        fn add_document(&self, doc: Document) {
            let mut documents = self.documents.borrow_mut();
            let path = doc.path();
            if let Some(already_existing) = documents.get(&path) {
                already_existing.set_modified(doc.modified());
                return;
            }

            documents.insert(path.clone(), doc.clone());

            let folders = self.folders.borrow();
            let parent = folders
                .get(path.parent().unwrap())
                .expect("Tried to add a document, but couldn't find its parent.");
            parent.add_document(doc.clone());

            drop(documents);
            drop(folders);

            self.obj().emit_by_name::<()>("document-added", &[&doc]);
        }

        pub(super) fn add_folder(&self, folder: Folder) {
            let mut folders = self.folders.borrow_mut();
            let path = folder.path();
            if folders.contains_key(&path) {
                return;
            }

            folders.insert(path.clone(), folder.clone());
            self.connect_folder(&folder);

            if !folder.is_root() {
                let parent = folders
                    .get(path.parent().unwrap())
                    .expect("Tried to add a folder, but couldn't find its parent.");
                parent.add_subfolder(folder.clone());
            }

            drop(folders);

            self.obj().emit_by_name::<()>("folder-added", &[&folder]);

            /*
            if self.expanded_folders.borrow().contains(&path) {
                folder_view.set_expanded(true);
            }

            */
        }

        /// Remove widgets for entries that don't exist in the library anymore
        fn prune(&self) {
            let mut folders = self.folders.borrow_mut();
            let mut documents = self.documents.borrow_mut();
            let mut dead_folders = vec![];
            let mut dead_documents = vec![];
            for path in folders.keys() {
                let is_hidden = path
                    .file_name()
                    .is_some_and(|s| s.as_encoded_bytes()[0] == b'.');
                let prune_hidden = self.ignore_hidden_files.get() && is_hidden;

                if !path.exists() || prune_hidden {
                    dead_folders.push(path.clone());

                    let parent_path = path.parent().unwrap();
                    if let Some(parent) = folders.get(parent_path) {
                        parent.remove_subfolder(path);
                    }
                }
            }
            for path in documents.keys() {
                let is_hidden = path
                    .file_name()
                    .is_some_and(|s| s.as_encoded_bytes()[0] == b'.');
                let prune_hidden = self.ignore_hidden_files.get() && is_hidden;

                if !path.exists() || prune_hidden {
                    dead_documents.push(path.clone());

                    let parent_path = path.parent().unwrap();
                    if let Some(parent) = folders.get(parent_path) {
                        parent.remove_document(path);
                    }
                }
            }

            for path in &dead_documents {
                if let Some(parent) = folders.get(path.parent().unwrap()) {
                    parent.remove_document(path);
                }
            }
            for path in &dead_folders {
                if let Some(parent) = folders.get(path.parent().unwrap()) {
                    parent.remove_subfolder(path);
                }
            }

            for path in &dead_documents {
                documents.remove(path).unwrap();
            }
            for path in &dead_folders {
                folders.remove(path).unwrap();
            }

            for path in &dead_documents {
                self.obj().emit_by_name::<()>("item-removed", &[path]);
            }
            for path in &dead_folders {
                self.obj().emit_by_name::<()>("item-removed", &[path]);
            }
        }

        fn connect_folder(&self, folder: &Folder) {
            folder.connect_closure(
                "subfolder-created",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: Folder, _path: PathBuf| {
                        imp.refresh_content();
                    }
                ),
            );

            folder.connect_closure(
                "document-created",
                false,
                closure_local!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_: Folder, _path: PathBuf| {
                        imp.refresh_content();
                    }
                ),
            );
        }
    }
}

use std::cell::Ref;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::closure_local;
use sourceview5::prelude::*;

use glib::Object;

use crate::data::Document;
use crate::data::Folder;
use crate::data::FolderType;
use crate::util::file_actions;

glib::wrapper! {
    pub struct Project(ObjectSubclass<imp::Project>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Project {
    /// New standard project
    pub fn new(path: PathBuf) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        imp.path.set(path.clone()).unwrap();
        let root = Folder::new_project_root(path);
        root.connect_closure(
            "close-project-requested",
            false,
            closure_local!(
                #[weak]
                obj,
                move |_: Folder| {
                    obj.close();
                }
            ),
        );
        imp.add_folder(root);
        obj
    }

    /// Builtin drafts project
    pub fn new_draft_table() -> Self {
        let path = file_actions::path_builtin_library();

        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        imp.path.set(path.clone()).unwrap();
        let root = Folder::new_drafts_root(file_actions::path_builtin_library());
        imp.add_folder(root);
        obj
    }

    pub fn path(&self) -> PathBuf {
        self.imp().path().to_path_buf()
    }

    pub fn is_drafts(&self) -> bool {
        self.folders().get(&self.path()).unwrap().kind() == FolderType::DraftsRoot
    }

    pub fn has_item(&self, path: &Path) -> bool {
        self.has_document(path) || self.has_folder(path)
    }

    pub fn has_document(&self, path: &Path) -> bool {
        self.documents().contains_key(path)
    }

    pub fn has_folder(&self, path: &Path) -> bool {
        self.folders().contains_key(path)
    }

    pub fn documents(&self) -> Ref<'_, HashMap<PathBuf, Document>> {
        self.imp().documents.borrow()
    }

    pub fn folders(&self) -> Ref<'_, HashMap<PathBuf, Folder>> {
        self.imp().folders.borrow()
    }

    pub fn get_folder(&self, path: &Path) -> Option<Folder> {
        self.folders().get(path).cloned()
    }

    pub fn get_document(&self, path: &Path) -> Option<Document> {
        self.documents().get(path).cloned()
    }

    pub fn refresh_content(&self) {
        self.imp().refresh_content();
    }

    pub fn close(&self) {
        self.emit_by_name::<()>("close-project-requested", &[]);
    }
}
