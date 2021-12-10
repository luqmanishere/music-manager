use std::path::{Path, PathBuf};

use eyre::{eyre, Result};
use rusqlite::{params, Connection};
use time::OffsetDateTime;

use super::song::Song;

pub struct Database {
    connection: Connection,
    path: PathBuf,
}

impl Database {
    pub fn open_from_path<P>(path: P) -> Result<Database>
    where
        P: AsRef<Path>,
    {
        let conn = Connection::open(&path)?;

        // Create the table if it doesn't exist
        conn.execute(
            "
            CREATE TABLE IF NOT EXISTS songs (
                id                  INTEGER UNIQUE PRIMARY KEY,
                song_path           nTEXT NOT NULL,
                song_filename       nTEXT NOT NULL,
                song_title          nTEXT,
                song_artist         nTEXT,
                song_album          nTEXT,
                song_genre          nTEXT,
                song_youtube_id     nTEXT,
                song_thumbnail_url  nTEXT,
                date_added          DATETIME
            )
            ",
            [],
        )?;

        let database = Database {
            connection: conn,
            path: path.as_ref().to_path_buf(),
        };

        Ok(database)
    }

    /// Returns all records in the database
    pub fn query_all_song_data(&self) -> Result<Vec<Song>> {
        let mut stmt = self.connection.prepare(
            "
            SELECT id, song_path, song_filename, song_title, song_artist, song_album, song_genre,
                song_youtube_id, song_thumbnail_url, date_added FROM songs
            ",
        )?;
        let song_iter = stmt.query_map([], |row| {
            Ok(Song::from_database(
                row.get(0).ok(),
                row.get(1)?,
                row.get(2)?,
                row.get(3).ok(),
                row.get(4).ok(),
                row.get(5).ok(),
                row.get(6).ok(),
                row.get(7).ok(),
                row.get(8).ok(),
            )
            .unwrap())
        })?;
        let song_vec = song_iter.map(|song| song.unwrap()).collect::<Vec<Song>>();
        if song_vec.is_empty() {
            return Err(eyre!("No results were found. Is the database empty?"));
        }
        Ok(song_vec)
    }

    /// Find records by its name. Song title is the exact title of the song.
    /// If an undeterminate search is required, use `search_song`
    #[allow(dead_code)]
    pub fn query_song_by_name(&self, song_title: &str) -> Result<Vec<Song>> {
        let query = format!("SELECT * from songs WHERE song_title = '{}'", song_title);
        let mut stmt = self.connection.prepare(&query)?;
        let song_iter = stmt.query_map([], |row| {
            Ok(Song::from_database(
                row.get(0).ok(),
                row.get(1)?,
                row.get(2)?,
                row.get(3).ok(),
                row.get(4).ok(),
                row.get(5).ok(),
                row.get(6).ok(),
                row.get(7).ok(),
                row.get(8).ok(),
            )
            .unwrap())
        })?;
        let song_vec = song_iter.map(|song| song.unwrap()).collect::<Vec<Song>>();

        if song_vec.is_empty() {
            return Err(eyre!("No results were found. Is the database empty?"));
        }

        Ok(song_vec)
    }

    /// Find a record by its ID
    pub fn query_song_by_id(&self, song_id: usize) -> Result<Vec<Song>> {
        let query = format!("SELECT * from songs WHERE id = '{}'", song_id);
        let mut stmt = self.connection.prepare(&query)?;
        let song_iter = stmt.query_map([], |row| {
            Ok(Song::from_database(
                row.get(0).ok(),
                row.get(1)?,
                row.get(2)?,
                row.get(3).ok(),
                row.get(4).ok(),
                row.get(5).ok(),
                row.get(6).ok(),
                row.get(7).ok(),
                row.get(8).ok(),
            )
            .unwrap())
        })?;
        let song_vec = song_iter.map(|song| song.unwrap()).collect::<Vec<Song>>();

        if song_vec.is_empty() {
            return Err(eyre!("No results were found. Is the database empty?"));
        }

        Ok(song_vec)
    }

    /// Insert a record into the database
    pub fn insert_song(&self, song: &Song) -> Result<()> {
        let mut artist_string = String::new();
        for artist in song.artists.as_ref().unwrap_or(&vec!["None".to_string()]) {
            artist_string.push_str(artist);
            if artist == "None" {
                continue;
            }
            artist_string.push(':');
        }

        let sql = "
            INSERT INTO songs (
                song_path,
                song_filename, 
                song_title,
                song_artist,
                song_album,
                song_genre,
                song_youtube_id,
                song_thumbnail_url,
                date_added
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ";
        self.connection.execute(
            sql,
            params![
                song.file_path.to_str(),
                song.file_name,
                song.title,
                artist_string,
                song.album,
                song.genre,
                song.youtube_id,
                song.thumbnail_url,
                OffsetDateTime::now_utc()
            ],
        )?;
        Ok(())
    }

    /// Updates an existing record in the database. Note that id and timestamp can never be changed
    pub fn update_song(&self, song: &Song) -> Result<()> {
        let mut artist_string = String::new();
        for artist in song.artists.as_ref().unwrap_or(&vec!["None".to_string()]) {
            artist_string.push_str(artist);
            if artist == "None" {
                continue;
            }
            artist_string.push(':');
        }

        let sql = "
            UPDATE songs SET
                song_path = ?9,
                song_filename = ?2, 
                song_title = ?3,
                song_artist = ?4,
                song_album = ?5,
                song_genre = ?6,
                song_youtube_id = ?7,
                song_thumbnail_url = ?8
            WHERE id = ?1
        ";
        self.connection.execute(
            sql,
            params![
                song.id,
                song.file_name,
                song.title,
                artist_string,
                song.album,
                song.genre,
                song.youtube_id,
                song.thumbnail_url,
                song.file_path.to_str()
            ],
        )?;
        Ok(())
    }

    /// Returns songs that contains a string in its metadata
    pub fn search_song(&self, search_term: &str) -> Result<Vec<Song>> {
        let query = format!(
            "SELECT * from songs
                WHERE song_title LIKE '%{}%' 
                OR song_artist LIKE '%{}%' 
                OR song_album LIKE '%{}%'",
            search_term, search_term, search_term
        );
        let mut stmt = self.connection.prepare(&query)?;
        let song_iter = stmt.query_map([], |row| {
            Ok(Song::from_database(
                row.get(0).ok(),
                row.get(1)?,
                row.get(2)?,
                row.get(3).ok(),
                row.get(4).ok(),
                row.get(5).ok(),
                row.get(6).ok(),
                row.get(7).ok(),
                row.get(8).ok(),
            )
            .unwrap())
        })?;
        let song_vec = song_iter.map(|song| song.unwrap()).collect::<Vec<Song>>();

        if song_vec.is_empty() {
            return Err(eyre!("No results were found."));
        }

        Ok(song_vec)
    }
    pub fn remove_song(&self, id: usize) -> Result<()> {
        let query = format!("DELETE from songs WHERE id = {}", id);
        self.connection.execute(&query, [])?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn database_path(&self) -> PathBuf {
        self.path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    // TODO: Write tests for database actions
    use super::*;

    #[test]
    fn open_database() {
        let path = Path::new("/tmp/database.sqlite");
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }

        Database::open_from_path(path).unwrap();
    }

    #[test]
    fn open_database_write_song() {
        let path = Path::new("/tmp/database2.sqlite");
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
        let database = Database::open_from_path(path).unwrap();

        let original_song = Song {
            id: None,
            file_path: "test.flac".into(),
            file_name: "test".into(),
            tag: metaflac::Tag::new(),
            title: Some("test".to_string()),
            artists: Some(vec!["testing_art".to_string()]),
            album: None,
            genre: None,
            youtube_id: None,
            thumbnail_url: None,
            ..Default::default()
        };

        database.insert_song(&original_song).unwrap();

        let all_song = database.query_all_song_data().unwrap();
        let all_song = all_song.first().unwrap();

        dbg!(&original_song);
        dbg!(&all_song);
        assert!(Song::equate(&original_song, all_song));
    }

    #[test]
    fn open_database_update_song() {
        let path = Path::new("/tmp/database1.sqlite");
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
        let database = Database::open_from_path(path).unwrap();

        let original_song = Song {
            id: None,
            file_path: "test.flac".into(),
            file_name: "test".into(),
            tag: metaflac::Tag::new(),
            title: Some("test".to_string()),
            artists: Some(vec!["testing_art".to_string()]),
            album: None,
            genre: None,
            youtube_id: None,
            thumbnail_url: None,
            ..Default::default()
        };

        database.insert_song(&original_song).unwrap();
        database
            .insert_song(&Song {
                ..Default::default()
            })
            .unwrap();

        let mut new_song = database.query_song_by_name("test").unwrap();
        let mut new_song = new_song.get_mut(0).unwrap();

        new_song.title = Some("test2".to_string());
        database.update_song(new_song).unwrap();

        let updated_title = database.query_song_by_name("test2").unwrap();

        dbg!(&original_song);
        dbg!(&new_song);
    }
}
