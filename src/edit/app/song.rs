use std::path::{Path, PathBuf};

use log::{debug, error, info, warn};
use metaflac::Tag;
use tui::widgets::ListState;

use eyre::Result;

pub struct Song {
    pub file_path: PathBuf,
    pub file_name: String,
    pub tag: Tag,
    pub title: Option<String>,
    pub artists: Option<Vec<String>>,
    pub album: Option<String>,

    pub items: Vec<String>,
    pub state: ListState,
    pub initialized: bool,
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
        self.initialized = true;
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

    fn _init_picture(&mut self) {
        todo!()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Guarantees display to be in a specific order
    /// Filename, song title, song artists, song album
    fn populate_list_items(&mut self) {
        debug!("Populating list items");
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

    pub fn edit(&mut self, new_value: String) {
        let index = self.state.selected().unwrap_or(10);
        match index {
            // Edit filename
            0 => {
                self.edit_filename(new_value);
                self.populate_list_items();
            }
            // Edit title
            1 => {
                self.edit_title(new_value);
                self.populate_list_items()
            }
            // Edit artists
            2 => {
                self.edit_artist(new_value);
                self.populate_list_items()
            }
            // Edit album
            3 => {
                self.edit_album(new_value);
                self.populate_list_items()
            }
            // Error codes
            10 => {
                warn!(target: "song_edit", "Unable to get index value for MetadataListWidget. 'Tis a bug");
            }
            _ => {}
        }
    }

    /// Edits the name of the file
    /// Note: file_name edits are applied immediately
    fn edit_filename(&mut self, mut new_file_name: String) {
        if !new_file_name.contains("flac") {
            new_file_name.push_str(".flac")
        }
        let mut new_file_path = self.file_path.clone();
        new_file_path.set_file_name(&new_file_name);

        match std::fs::rename(&self.file_path, &new_file_path) {
            Ok(_) => {
                self.file_name = new_file_name;
                self.file_path = new_file_path;
                info!(target: "song_edit", "Set filename to: {}", self.file_name);
                match Tag::read_from_path(&self.file_path) {
                    Ok(tag) => {
                        self.tag = tag;
                        self.init();
                    }
                    Err(e) => {
                        error!("Error reading FLAC tags: {}", e);
                    }
                };
            }
            Err(e) => {
                error!("File renaming failed: {}", e);
            }
        }
    }

    fn edit_title(&mut self, new_value: String) {
        self.title = Some(new_value);
        self.tag
            .set_vorbis("TITLE", vec![self.title.as_ref().unwrap()]);
    }

    fn edit_artist(&mut self, new_value: String) {
        let mut artists = vec![];
        for artist in new_value.split(':') {
            artists.push(artist.to_string());
        }
        self.tag.set_vorbis("ARTIST", artists.clone());
        self.artists = Some(artists);
    }

    fn edit_album(&mut self, new_album_value: String) {
        self.tag.set_vorbis("ALBUM", vec![new_album_value.clone()]);
        self.album = Some(new_album_value);
    }

    fn _edit_picture(&mut self, _new_value: &[u8]) {
        // TODO: Implement setting a picture
        todo!()
    }

    pub fn write_tag_changes(&mut self) -> Result<()> {
        self.tag.write_to_path(&self.file_path)?;
        info!("Wrote tags to file!");
        Ok(())
    }

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
            initialized: false,
        }
    }
}
