mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use std::time::Duration;

    use adw::subclass::prelude::*;
    use async_channel::Receiver;
    use async_channel::Sender;
    use glib::closure_local;
    use glib::subclass::*;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::glib::timeout_add_local;
    use gtk::prelude::*;

    use gtk::CompositeTemplate;
    use gtk::glib::MainContext;

    use crate::data::FolderObject;
    use crate::data::SheetObject;
    use crate::widgets::LibraryFolder;
    use crate::widgets::LibraryProjectErrPlaceholder;
    use crate::widgets::LibrarySheet;

    #[derive(Debug)]
    enum ProjectEntry {
        Dir { path: PathBuf, depth: u32 },
        File { path: PathBuf, depth: u32 },
    }

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/library_project.ui")]
    pub struct LibraryProject {
        #[template_child]
        pub(super) project_root_vbox: TemplateChild<gtk::Box>,

        pub(super) root_folder: RefCell<Option<LibraryFolder>>,
        pub(super) subfolders: RefCell<HashMap<PathBuf, LibraryFolder>>,
        pub(super) sheets: RefCell<HashMap<PathBuf, LibrarySheet>>,
        /// Is this a builtin project (drafts)
        pub(super) is_builtin: Cell<bool>,
        /// Project folder is inaccessible or deleted
        is_invalid: Cell<bool>,
        crawler_rx: RefCell<Option<Receiver<ProjectEntry>>>,
        crawler_tx: RefCell<Option<Sender<ProjectEntry>>>,
        pub(super) expanded_folders: RefCell<HashSet<PathBuf>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryProject {
        const NAME: &'static str = "LibraryProject";
        type Type = super::LibraryProject;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibraryProject {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

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
                                ProjectEntry::Dir { path, depth } => {
                                    imp.add_subfolder(path, depth);
                                }
                                ProjectEntry::File { path, depth } => {
                                    imp.add_sheet(path, depth);
                                }
                            }
                        }
                        glib::ControlFlow::Continue
                    }
                ),
            );
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("folder-added")
                        .param_types([LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("document-added")
                        .param_types([LibrarySheet::static_type()])
                        .build(),
                    Signal::builder("document-selected")
                        .param_types([PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-rename-requested")
                        .param_types([LibraryFolder::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("document-rename-requested")
                        .param_types([LibrarySheet::static_type(), PathBuf::static_type()])
                        .build(),
                    Signal::builder("folder-delete-requested")
                        .param_types([LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("document-delete-requested")
                        .param_types([LibrarySheet::static_type()])
                        .build(),
                    Signal::builder("folder-trash-requested")
                        .param_types([LibraryFolder::static_type()])
                        .build(),
                    Signal::builder("document-trash-requested")
                        .param_types([LibrarySheet::static_type()])
                        .build(),
                    Signal::builder("close-project-requested").build(),
                ]
            })
        }
    }

    impl WidgetImpl for LibraryProject {}
    impl BinImpl for LibraryProject {}

    impl LibraryProject {
        pub(super) fn setup_root(&self, root_folder: LibraryFolder) {
            assert!(self.root_folder.borrow().is_none());
            let vbox = &self.project_root_vbox;
            vbox.append(&root_folder);
            self.connect_folder(root_folder.clone());
            self.root_folder.replace(Some(root_folder));
            self.refresh_content();
        }

        pub(super) fn refresh_content(&self) {
            if self.is_invalid.get() {
                return;
            }
            let root_path = self.obj().root_path();
            if !root_path.exists() {
                self.mark_invalid();
                return;
            }

            let sender = self.crawler_tx.borrow().as_ref().unwrap().clone();

            MainContext::default().spawn_local(async move {
                let mut search_stack: VecDeque<(PathBuf, u32)> = VecDeque::from([(root_path, 1)]);
                let mut found_folders: Vec<(PathBuf, u32)> = vec![];
                let mut found_files: Vec<(PathBuf, u32)> = vec![];

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
                        let path = entry.path();
                        if metadata.is_dir() {
                            search_stack.push_back((path.clone(), depth + 1));
                            found_folders.push((path.clone(), depth + 1));
                            sender
                                .send(ProjectEntry::Dir { path, depth })
                                .await
                                .expect("Crawler failed to send dir path!");
                        } else {
                            if !path
                                .extension()
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                            {
                                continue;
                            }

                            found_files.push((path.clone(), depth + 1));
                            sender
                                .send(ProjectEntry::File { path, depth })
                                .await
                                .expect("Crawler failed to send file path!");
                        }
                    }
                }
            });

            self.prune();
        }

        fn mark_invalid(&self) {
            self.is_invalid.replace(true);
            self.root_folder
                .borrow()
                .as_ref()
                .unwrap()
                .set_visible(false);
            let err_placeholder = LibraryProjectErrPlaceholder::new(&self.obj().root_path());
            let obj = self.obj();
            err_placeholder.connect_closure(
                "close-project-requested",
                false,
                closure_local!(
                    #[weak]
                    obj,
                    move |_: LibraryProjectErrPlaceholder| {
                        obj.emit_by_name::<()>("close-project-requested", &[]);
                    }
                ),
            );
            self.project_root_vbox.append(&err_placeholder);
        }

        fn add_subfolder(&self, path: PathBuf, depth: u32) {
            let mut subfolders = self.subfolders.borrow_mut();
            if subfolders.contains_key(&path) {
                return;
            }

            let folder = LibraryFolder::new(&FolderObject::new(path.clone(), depth));
            self.connect_folder(folder.clone());
            subfolders.insert(path.clone(), folder.clone());

            let parent_path = path.parent().unwrap();
            if let Some(parent) = subfolders.get(parent_path) {
                parent.add_subfolder(folder.clone());
            } else if *parent_path == self.root_folder.borrow().as_ref().unwrap().path() {
                self.root_folder
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .add_subfolder(folder.clone());
            } else {
                panic!("Tried to add a folder, but couldn't find its parent.");
            }

            if self.expanded_folders.borrow().contains(&path) {
                folder.set_expanded(true);
            }
        }

        fn add_sheet(&self, path: PathBuf, depth: u32) {
            let mut sheets = self.sheets.borrow_mut();
            let subfolders = self.subfolders.borrow();
            if sheets.contains_key(&path) {
                return;
            }

            let sheet = LibrarySheet::new(&SheetObject::new(path.clone(), depth));
            sheets.insert(path.clone(), sheet.clone());

            let parent_path = path.parent().unwrap();
            if let Some(parent) = subfolders.get(parent_path) {
                parent.add_sheet(sheet.clone());
            } else if *parent_path == self.root_folder.borrow().as_ref().unwrap().path() {
                self.root_folder
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .add_sheet(sheet.clone());
            } else {
                panic!("Tried to add a sheet, but couldn't find its parent.");
            }

            self.obj().emit_by_name::<()>("document-added", &[&sheet]);
        }

        /// Remove widgets for entries that don't exist in the library anymore
        fn prune(&self) {
            let mut subfolders = self.subfolders.borrow_mut();
            let mut sheets = self.sheets.borrow_mut();
            let mut dead_folders = vec![];
            let mut dead_sheets = vec![];
            for (path, folder) in subfolders.iter() {
                if !path.exists() {
                    dead_folders.push(path.clone());

                    let parent_path = path.parent().unwrap();

                    if let Some(parent) = subfolders.get(parent_path) {
                        parent.remove_subfolder(folder);
                    } else if *parent_path == self.root_folder.borrow().as_ref().unwrap().path() {
                        self.root_folder
                            .borrow()
                            .as_ref()
                            .unwrap()
                            .remove_subfolder(folder);
                    }
                }
            }
            for (path, sheet) in sheets.iter() {
                if !path.exists() {
                    dead_sheets.push(path.clone());

                    let parent_path = path.parent().unwrap();

                    if let Some(parent) = subfolders.get(parent_path) {
                        parent.remove_sheet(sheet);
                    } else if *parent_path == self.root_folder.borrow().as_ref().unwrap().path() {
                        self.root_folder
                            .borrow()
                            .as_ref()
                            .unwrap()
                            .remove_sheet(sheet);
                    }
                }
            }
            for path in dead_folders {
                subfolders
                    .remove(&path)
                    .expect("dead folder entry disappeared?");
            }
            for path in dead_sheets {
                sheets.remove(&path).expect("dead sheet entry disappeared?");
            }
        }

        fn connect_folder(&self, folder: LibraryFolder) {
            self.obj().emit_by_name::<()>("folder-added", &[&folder]);

            folder.connect_closure(
                "subfolder-created",
                false,
                closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: LibraryFolder, _path: PathBuf| {
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
                    move |_: LibraryFolder, _path: PathBuf| {
                        this.refresh_content();
                    }
                ),
            );
        }
    }
}

use std::path::{Path, PathBuf};

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::closure_local;
use sourceview5::prelude::*;

use glib::Object;

use crate::data::FolderObject;
use crate::util::path_builtin_library;

use crate::widgets::LibraryFolder;
use crate::widgets::LibrarySheet;

glib::wrapper! {
    pub struct LibraryProject(ObjectSubclass<imp::LibraryProject>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LibraryProject {
    /// New standard project
    pub fn new(path: PathBuf) -> Self {
        let this: Self = Object::builder().build();
        this.imp().is_builtin.replace(false);
        let root = LibraryFolder::new_project_root(&FolderObject::new(path.clone(), 0));
        root.connect_closure(
            "close-project-requested",
            false,
            closure_local!(
                #[weak]
                this,
                move |_: LibraryFolder| {
                    this.emit_by_name::<()>("close-project-requested", &[]);
                }
            ),
        );
        this.imp().setup_root(root);
        this
    }

    /// Builtin drafts project
    pub fn new_draft_table() -> Self {
        let this: Self = Object::builder().build();
        this.imp().is_builtin.replace(true);
        let root = LibraryFolder::new_drafts_root(&FolderObject::new(path_builtin_library(), 0));
        this.imp().setup_root(root);
        this
    }

    pub fn path(&self) -> PathBuf {
        self.root_folder().path()
    }

    pub fn root_path(&self) -> PathBuf {
        self.imp().root_folder.borrow().as_ref().unwrap().path()
    }

    pub fn root_folder(&self) -> LibraryFolder {
        self.imp().root_folder.borrow().clone().unwrap()
    }

    pub fn is_builtin(&self) -> bool {
        self.imp().is_builtin.get()
    }

    pub fn expanded_folder_paths(&self) -> Vec<String> {
        let mut paths = vec![];
        if self.root_folder().is_expanded() {
            paths.push(self.root_path().to_str().unwrap().to_owned());
        }
        for (path, folder) in self.imp().subfolders.borrow().iter() {
            if folder.is_expanded() {
                paths.push(path.to_str().unwrap().to_owned());
            }
        }
        paths
    }

    pub fn expand_folder(&self, path: PathBuf) {
        let imp = self.imp();
        if let Some(folder) = imp.subfolders.borrow().get(&path) {
            folder.set_expanded(true);
        } else if path == self.root_path() {
            self.root_folder().set_expanded(true);
        }
        imp.expanded_folders.borrow_mut().insert(path);
    }

    pub fn get_folder(&self, path: &Path) -> Option<LibraryFolder> {
        let sub = self.imp().subfolders.borrow().get(path).cloned();
        if sub.is_some() {
            sub
        } else if *path == self.root_path() {
            Some(self.root_folder())
        } else {
            None
        }
    }

    pub fn get_sheet(&self, path: &Path) -> Option<LibrarySheet> {
        self.imp().sheets.borrow().get(path).cloned()
    }

    pub fn refresh_content(&self) {
        self.imp().refresh_content();
    }
}
