use std::path::{Path, PathBuf};

use tui_c::widgets::ListState;

use eyre::Result;

pub struct DirListState {
    pub current_dir_path: PathBuf,
    pub current_dir_file_names: Vec<String>,
    pub current_dir_file_paths: Vec<PathBuf>,
    pub state: ListState,
}

impl DirListState {
    pub fn new() -> Self {
        let current_dir_path = directories_next::UserDirs::new()
            .unwrap()
            .audio_dir()
            .unwrap()
            .to_path_buf();

        let (current_dir_file_names, current_dir_file_paths) =
            get_files_in_dir(&current_dir_path).unwrap();
        DirListState {
            current_dir_path,
            current_dir_file_names,
            current_dir_file_paths,
            state: ListState::default(),
        }
    }

    pub fn poll(&mut self) {
        let (dir_file_names, dir_file_paths) = get_files_in_dir(&self.current_dir_path).unwrap();
        self.set_items(dir_file_names, dir_file_paths);
    }

    fn set_items(&mut self, dir_file_names: Vec<String>, dir_file_paths: Vec<PathBuf>) {
        if self.current_dir_file_names != dir_file_names {
            self.current_dir_file_names = dir_file_names;

            self.state = ListState::default();
        }

        if self.current_dir_file_paths != dir_file_paths {
            self.current_dir_file_paths = dir_file_paths;

            self.state = ListState::default();
        }
    }

    /// Select the next item.
    /// If current selection is the last item in the list, it will return to the top
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.current_dir_file_names.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Selects the previous item
    /// Selects the bottom most item if selection already reached the top
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.current_dir_file_names.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    #[allow(dead_code)]
    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

pub fn get_files_in_dir(path: &Path) -> Result<(Vec<String>, Vec<PathBuf>)> {
    let mut paths = std::fs::read_dir(path)?
        .map(|res| res.map(|e| (e.path())))
        .collect::<Result<Vec<_>, _>>()?;
    let mut file_name = paths
        .iter()
        .map(|e| e.file_name().unwrap().to_str().unwrap().to_owned())
        .collect::<Vec<String>>();

    // Sort the randomness
    file_name.sort();
    paths.sort();
    Ok((file_name, paths))
}
