use crate::ui::Event;
use crossbeam::channel::{Receiver, Sender};
use crossterm::event::{KeyCode, KeyEvent};
use debug_stub_derive::DebugStub;
use eyre::Result;
use metaflac::Tag;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
};
use tui::widgets::{List, ListItem, ListState};

#[derive(Debug)]
pub enum EditAppState {
    SongSelect,
    MetadataEdit,
}

#[derive(Debug)]
pub enum InputMode {
    Navigation,
    TextInput,
}

#[derive(Debug)]
pub struct EditApp {
    pub current_dir_path: PathBuf,
    pub current_dir_file_names: Vec<String>,
    pub current_dir_file_paths: Vec<PathBuf>,
    pub current_editing_song: Song,
    pub state: EditAppState,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub messages: Vec<String>,
    pub dir_list: DirList,
    pub input: Arc<Input>,
}

impl EditApp {
    pub fn new(rx_input: Receiver<Event<KeyEvent>>) -> Self {
        Self {
            current_dir_path: directories_next::UserDirs::new()
                .unwrap()
                .audio_dir()
                .unwrap()
                .to_path_buf(),
            current_dir_file_names: vec![],
            state: EditAppState::SongSelect,
            input_mode: InputMode::Navigation,
            input_buffer: String::new(),
            messages: vec![],
            dir_list: DirList::new(vec![]),
            current_editing_song: Song::default(),
            current_dir_file_paths: vec![],
            input: Arc::new(Input::new(rx_input)),
        }
    }
    pub fn on_tick(&mut self) {
        self.poll_dir_for_files();
        self.dir_list.set_items(self.current_dir_file_names.clone());
    }
    fn poll_dir_for_files(&mut self) {
        let (current_dir_file_names, current_dir_file_paths) =
            get_files_in_dir(&self.current_dir_path).unwrap();
        self.current_dir_file_names = current_dir_file_names;
        self.current_dir_file_paths = current_dir_file_paths;
    }

    // TODO refactor for other modes as well
    pub fn on_up(&mut self) {
        match self.state {
            EditAppState::SongSelect => self.dir_list.previous(),
            EditAppState::MetadataEdit => self.current_editing_song.previous(),
        }
    }

    pub fn on_down(&mut self) {
        match self.state {
            EditAppState::SongSelect => self.dir_list.next(),
            EditAppState::MetadataEdit => self.current_editing_song.next(),
        }
    }

    pub fn on_enter(&mut self) {
        match self.state {
            EditAppState::SongSelect => {
                if let Some(index) = self.dir_list.state.selected() {
                    self.current_editing_song =
                        Song::read_music_file(self.current_dir_file_paths.get(index).unwrap())
                            .unwrap();
                    self.state = EditAppState::MetadataEdit;
                }
            }
            EditAppState::MetadataEdit => {
                if let Some(index) = self.current_editing_song.state.selected() {
                    std::fs::write("ok1", "ok1").unwrap();
                    let res = Input::start_editing(self.input.clone());

                    self.current_editing_song.edit(index, " ".to_string());
                };
            }
        }
    }
    pub fn on_esc(&mut self) {
        match self.state {
            EditAppState::SongSelect => {
                self.dir_list.unselect();
            }
            EditAppState::MetadataEdit => {
                self.state = EditAppState::SongSelect;
            }
        }
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

#[derive(Debug)]
pub struct DirList {
    pub items: Vec<String>,
    pub state: ListState,
}

impl DirList {
    pub fn new(items: Vec<String>) -> DirList {
        DirList {
            items,
            state: ListState::default(),
        }
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        if self.items != items {
            self.items = items;

            self.state = ListState::default();
        }
    }

    /// Select the next item.
    /// If current selection is the last item in the list, it will return to the top
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
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
                    self.items.len() - 1
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

#[derive(DebugStub)]
pub struct Song {
    pub file_path: PathBuf,
    pub file_name: String,
    #[debug_stub = "metaflac::Tag"]
    pub tag: Tag,
    pub title: Option<String>,
    pub artists: Option<Vec<String>>,
    pub album: Option<String>,

    pub items: Vec<String>,
    pub state: ListState,
}

impl Song {
    pub fn read_music_file(path: &Path) -> Result<Self> {
        let tag = Tag::read_from_path(path)?;
        let mut song = Self {
            file_path: path.to_path_buf(),
            file_name: path.file_name().unwrap().to_str().unwrap().to_owned(),
            tag,
            ..Default::default()
        };

        song.init();
        song.populate_list_items();
        Ok(song)
    }

    fn init(&mut self) {
        self.init_title();
        self.init_artist();
        self.init_album();
    }

    fn init_title(&mut self) {
        self.title = self
            .tag
            .get_vorbis("TITLE")
            .map(|mut title| title.next().unwrap().to_owned());
    }

    fn init_artist(&mut self) {
        self.artists = self
            .tag
            .get_vorbis("ARTIST")
            .map(|artists| artists.map(|e| e.to_owned()).collect::<Vec<String>>());
    }
    /// I want only one album, okay?
    /// If i have to change this in the future, so be it.
    fn init_album(&mut self) {
        self.album = self
            .tag
            .get_vorbis("ALBUM")
            .map(|mut album| album.next().unwrap().to_owned());
    }

    fn init_picture(&mut self) {
        todo!()
    }

    /// Guarantees display to be in a specific order
    /// Filename, song title, song artists, song album
    fn populate_list_items(&mut self) {
        let mut file_name_string = String::from("File name: ");
        file_name_string.push_str(&self.file_name);

        let mut title_string = String::from("Title: ");
        title_string.push_str(&self.title.clone().unwrap_or_else(|| "None".to_string()));

        let mut artist_string = String::from("Artists: ");
        for artist in self.artists.as_ref().unwrap_or(&vec!["None".to_string()]) {
            artist_string.push_str(artist);
            if artist == "None" {
                continue;
            }
            artist_string.push(':');
        }

        let mut album_string = String::from("Album: ");
        album_string.push_str(&self.album.clone().unwrap_or_else(|| "None".to_string()));

        let items = vec![file_name_string, title_string, artist_string, album_string];

        self.items = items;
    }

    pub fn edit(&mut self, index: usize, new_value: String) {
        match index {
            0 => {
                self.edit_filename(new_value);
                self.populate_list_items();
            }
            1 => {}
            2 => {}
            3 => {}
            _ => {}
        }
    }

    fn edit_filename(&mut self, new_value: String) {
        self.file_name = new_value;
    }

    fn edit_title(&mut self) {}

    fn edit_artist(&mut self) {}

    fn edit_album(&mut self) {}

    //METADATA_BLOCK_PICTURE

    /// Select the next item.
    /// If current selection is the last item in the list, it will return to the top
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
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
                    self.items.len() - 1
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

impl Default for Song {
    fn default() -> Self {
        Self {
            file_path: Default::default(),
            file_name: Default::default(),
            tag: Default::default(),
            title: Default::default(),
            artists: Default::default(),
            album: Default::default(),
            items: vec![],
            state: ListState::default(),
        }
    }
}

#[derive(Debug)]
pub enum InputEvent {
    Init,
    Clear,
    DoneEditing,
}

// TODO Implement better input handling that isn't tied to ui
#[derive(Debug)]
pub struct Input {
    pub editing: AtomicBool,
    pub buffer: Arc<Mutex<String>>,
    pub messages: Arc<Mutex<Vec<String>>>,
    pub rx_events: Receiver<Event<KeyEvent>>,
    tx_input: Sender<InputEvent>,
    rx_input: Receiver<InputEvent>,
}

impl Input {
    pub fn new(rx_input: Receiver<Event<KeyEvent>>) -> Self {
        let (tx, rx) = crossbeam::channel::unbounded();
        Self {
            editing: AtomicBool::new(false),
            buffer: Default::default(),
            messages: Default::default(),
            rx_events: rx_input,
            tx_input: tx,
            rx_input: rx,
        }
    }

    pub fn init(&self) {}

    pub fn push(&self, char: char) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push(char);
    }

    pub fn commit(&self) {
        let mut buffer = self.buffer.lock().unwrap();
        let mut messages = self.messages.lock().unwrap();

        messages.push(buffer.clone());
        buffer.clear();
    }

    pub fn start_editing(input_struct: Arc<Self>) {
        std::fs::write("ok", "ok").unwrap();
        let handle = thread::spawn(move || {
            std::fs::write("ok2", "ok2").unwrap();
            let input_struct = input_struct;
            loop {
                match input_struct.rx_events.recv().unwrap() {
                    Event::Input(event) => match event.code {
                        KeyCode::Esc => {
                            input_struct.push(' ');
                            input_struct.tx_input.send(InputEvent::DoneEditing).unwrap();
                        }
                        KeyCode::Char(c) => input_struct.push(c),
                        KeyCode::Enter => {
                            input_struct.commit();
                            input_struct
                                .editing
                                .store(true, std::sync::atomic::Ordering::Relaxed);
                            input_struct.tx_input.send(InputEvent::DoneEditing).unwrap();
                            break;
                        }
                        _ => {}
                    },
                    Event::Tick => {}
                    Event::DoneEditing => {
                        input_struct
                            .editing
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        });
    }
}
