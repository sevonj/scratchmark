use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::util;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectState {
    #[serde(skip)]
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub open_file: Option<PathBuf>,
    #[serde(default)]
    pub expanded_folders: Vec<PathBuf>,
}

impl ProjectState {
    pub fn new(path: PathBuf) -> Self {
        let name = util::incremented_path(path.clone())
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        Self {
            name,
            path,
            open_file: None,
            expanded_folders: vec![],
        }
    }

    pub fn load(state_file_path: &Path) -> Result<Self, ()> {
        let name = state_file_path
            .to_owned()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let mut data = String::new();
        if let Err(_) = File::open(state_file_path).and_then(|mut f| f.read_to_string(&mut data)) {
            return Err(());
        };
        let Ok(mut this) = serde_json::from_str::<Self>(&data) else {
            return Err(());
        };
        this.name = name;
        Ok(this)
    }

    pub fn save(&self) -> Result<(), ()> {
        let Ok(data) = serde_json::to_string(self) else {
            return Err(());
        };
        let filepath = util::path_project_list_dir()
            .join(&self.name)
            .with_extension("json");

        let Ok(mut f) = File::create(filepath) else {
            return Err(());
        };
        util::ensure_project_list_dir();
        f.write(data.as_bytes()).map(|_| ()).map_err(|_| ())
    }

    pub fn list_projects() -> Vec<PathBuf> {
        let mut list = vec![];
        let Ok(entries) = util::path_project_list_dir().read_dir() else {
            return list;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_file() {
                list.push(entry.path());
            }
        }
        list
    }
}
