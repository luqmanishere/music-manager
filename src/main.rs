use std::{path::Path, sync::Arc};

use clap::{crate_authors, crate_version, App as CApp, AppSettings, Arg, ArgMatches};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use edit::{
    app::App,
    io::{handler::IoAsyncHandler, IoEvent},
    start_ui,
};
use eyre::{eyre, Context, Result};
use image::ImageFormat;
use log::{debug, info};
use metaflac::Tag;
use youtube_dl::{
    SearchOptions, SingleVideo as Video, YoutubeDl,
    YoutubeDlOutput::{Playlist, SingleVideo},
};

use crate::data::{database::Database, song::Song};

mod data;
mod edit;

/// Main function
///
/// Made async to support async
#[tokio::main]
async fn main() -> Result<()> {
    // This program manages music in FLAC format
    // Additional formats are to be added later

    // Setup clap
    let matches = setup_cli();
    match matches.subcommand_name() {
        Some("download") => {
            download(
                matches
                    .subcommand_matches("download")
                    .ok_or_else(|| eyre!("No arguments gave to subcommand download"))?,
            )
            .await?;
        }
        Some("edit") => {
            edit(
                matches
                    .subcommand_matches("edit")
                    .ok_or_else(|| eyre!("No arguments gave to subcommand edit"))?,
            )
            .await?;
        }
        Some("list") => {
            list(
                matches
                    .subcommand_matches("list")
                    .ok_or_else(|| eyre!("No arguments gave to subcommand list"))?,
            )?;
        }
        Some("remove") => {
            remove(
                matches
                    .subcommand_matches("remove")
                    .ok_or_else(|| eyre!("No arguments gave to subcommand remove"))?,
            )?;
        }
        Some("search") => {
            search(
                matches
                    .subcommand_matches("search")
                    .ok_or_else(|| eyre!("No arguments gave to subcommand search"))?,
            )?;
        }
        Some(_) => {
            // TODO: handle the error instead of panicking
            panic!("CLAP IS NOT WORKING");
        }
        None => {}
    }

    Ok(())
}

fn setup_cli() -> ArgMatches {
    CApp::new("music-manager")
        .about("Manage music the way the author likes")
        .version(crate_version!())
        .author(crate_authors!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::HelpRequired)
        .subcommand(
            CApp::new("download")
                .about("Downloads the song title given")
                .arg(
                    Arg::new("search-only")
                        .long("search-only")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("title")
                        .about("The title of the song to be downloaded")
                        .takes_value(true)
                        .required(true)
                        .multiple_values(true)
                        .use_delimiter(false)
                        .index(1),
                ),
        )
        .subcommand(CApp::new("edit").about("Edit song library"))
        .subcommand(CApp::new("list").about("List songs registered in the database"))
        .subcommand(
            CApp::new("remove")
                .about("Remove a song registered in the database")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(Arg::new("id").takes_value(true).long("id").short('i'))
                .arg(
                    Arg::new("title")
                        .takes_value(true)
                        .long("title")
                        .short('t')
                        .forbid_empty_values(true),
                ),
        )
        .subcommand(
            CApp::new("search")
                .about("Search for songs in database")
                .arg(
                    Arg::new("title")
                        .takes_value(true)
                        .required(true)
                        .forbid_empty_values(true),
                ),
        )
        .get_matches()
}

async fn download(args: &ArgMatches) -> Result<()> {
    let music_dir = directories_next::UserDirs::new()
        .ok_or_else(|| eyre!("directories_next failed to initialize"))?;
    let music_dir = music_dir
        .audio_dir()
        .ok_or_else(|| eyre!("directories_next failed to retrieve music dir"))?
        .to_path_buf();
    let title = args
        .values_of("title")
        .ok_or_else(|| eyre!("Song title is not given"))?
        .collect::<Vec<&str>>()
        .join(" ");
    let search_options = SearchOptions::youtube(title).with_count(5);
    let ytsearch = YoutubeDl::search_for(&search_options)
        .socket_timeout("10")
        .run()?;

    match ytsearch {
        Playlist(playlist) => {
            let entries = playlist
                .entries
                .ok_or_else(|| eyre!("Can't get video entries"))?;

            let mut count = 1;
            let mut entries_vec = vec![];
            for video in &entries {
                entries_vec.push(format!(
                    "{}. Title: {}, Channel:{}",
                    count,
                    video.title,
                    video.channel.as_ref().unwrap()
                ));
                count += 1;
            }

            if !args.is_present("search-only") {
                println!("[Enter] or [Space] to select: ");

                if let Some(selection) = Select::with_theme(&ColorfulTheme::default())
                    .items(&entries_vec)
                    .default(0)
                    .interact_opt()?
                {
                    let output_format = music_dir.join("%(title)s.%(ext)s");
                    let video = &entries
                        .get(selection)
                        .ok_or_else(|| eyre!("Can't get entry number: {}", selection))?;

                    let mut video_title = video.title.replace("/", "_").replace(":", " -");
                    video_title.push_str(".opus");

                    let mut filename_opus = music_dir.join(&video_title);
                    filename_opus.set_extension("opus");
                    let mut filename_flac = filename_opus.with_extension("flac");

                    if !filename_opus.exists() && !filename_flac.exists() {
                        // Download if opus does not exist
                        println!(
                            "Downloading: {} from channel: {} using youtube-dl...",
                            video.title,
                            video.channel.as_ref().unwrap()
                        );
                        youtube_dl_download_audio(video, &output_format)?;

                        ffmpeg_convert_to_flac(&filename_opus, &filename_flac)?;
                    } else if !filename_flac.exists() && filename_opus.exists() {
                        // File is downloaded, but not yet converted
                        ffmpeg_convert_to_flac(&filename_opus, &filename_flac)?;
                    } else {
                        // If opus file does not exist
                        println!("Song is already downloaded");
                    }

                    let rename_file = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Do you want to rename the file?")
                        .default(true)
                        .interact()?;

                    if rename_file {
                        let mut filename_new_input = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("File name: ")
                            .default(
                                filename_flac
                                    .file_name()
                                    .unwrap()
                                    .to_str()
                                    .unwrap()
                                    .to_string(),
                            )
                            .interact()?;
                        filename_new_input.push_str(".flac");

                        let mut filename_new = filename_flac.clone();
                        filename_new.set_file_name(&filename_new_input);

                        std::fs::rename(&filename_flac, filename_new)?;
                        filename_flac.set_file_name(filename_new_input);
                        println!("File rename successful");
                    }

                    // Add the song to the database
                    let edit_metadata = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Do you want to edit metadata now?")
                        .default(true)
                        .interact()?;
                    if edit_metadata {
                        let song_title: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Song title")
                            .default(video_title)
                            .interact()?;
                        let song_artist: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Song artist: ")
                            .default(video.channel.clone().unwrap())
                            .interact()?;
                        let song_album: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Song album: ")
                            .default("Unknown".to_string())
                            .interact()?;

                        let mut tag = Tag::read_from_path(&filename_flac)?;
                        tag.set_vorbis("TITLE", vec![song_title.clone()]);
                        tag.set_vorbis("ARTIST", vec![song_artist.clone()]);
                        tag.set_vorbis("ALBUM", vec![song_album.clone()]);

                        let request = reqwest::get(video.thumbnail.clone().unwrap()).await;
                        match request {
                            Ok(request) => {
                                let picture =
                                    image::load_from_memory(&request.bytes().await?.to_vec())?;
                                let mut vect = vec![];
                                // BUG: Figure out why the picture is black and white
                                picture.write_to(&mut vect, ImageFormat::Jpeg)?;
                                tag.add_picture(
                                    "image/jpeg",
                                    metaflac::block::PictureType::CoverFront,
                                    vect,
                                );
                            }
                            Err(e) => {
                                println!("Error: {}", e);
                            }
                        };
                        tag.save()?;

                        let database = Database::open_from_path(music_dir.join("database.sqlite"))?;
                        database.insert_song(&Song {
                            file_path: filename_flac.clone(),
                            file_name: filename_flac
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string(),
                            title: Some(song_title),
                            artists: Some(vec![song_artist]),
                            album: Some(song_album),
                            youtube_id: Some(video.id.clone()),
                            thumbnail_url: Some(video.thumbnail.clone().unwrap()),
                            ..Default::default()
                        })?;
                        println!("Inserted into database");
                    }
                } else {
                    return Err(eyre!("User canceled"));
                }
            } else {
                // Shows search results
                println!("Search results: ");
                for entries in entries_vec {
                    println!("{}", entries);
                }
            }
        }
        SingleVideo(video) => {
            println!("Title: {}, Channel:{}", video.title, video.channel.unwrap())
            // TODO: handle the case of only 1 video coming up on the search (as impossible that is)
        }
    }

    Ok(())
}

fn youtube_dl_download_audio(video: &Video, output_format: &Path) -> Result<()> {
    let youtube_args = [
        "--audio-format",
        "opus",
        "--audio-quality",
        "0",
        "-x",
        "--output",
        output_format
            .to_str()
            .ok_or_else(|| eyre!("Can't convert path to str"))?,
    ];
    let youtube_dl = std::process::Command::new("youtube-dl")
        .args(youtube_args)
        .arg(&video.id)
        .status()?;
    if youtube_dl.success() {
        Ok(())
    } else {
        Err(eyre!("youtube-dl failed to download"))
    }
}

fn ffmpeg_convert_to_flac(input_file: &Path, output_file: &Path) -> Result<()> {
    let ffmpeg_args = [
        "-i",
        input_file.to_str().unwrap(),
        "-compression_level",
        "12",
        output_file.to_str().unwrap(),
    ];
    println!("Converting to FLAC format using ffmpeg...");
    let ffmpeg = std::process::Command::new("ffmpeg")
        .args(ffmpeg_args)
        .status()?;
    if ffmpeg.success() {
        println!("Conversion to FLAC successful");
        println!("Deleting old opus file...");
        std::fs::remove_file(input_file)?;
        Ok(())
    } else {
        return Err(eyre!("ffmpeg failed with code: {}", ffmpeg.code().unwrap()));
    }
}

/// Executed by the edit command.
///
/// Launches a TUI for editing metadata
async fn edit(_args: &ArgMatches) -> Result<()> {
    tui_logger::init_logger(log::LevelFilter::Trace).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Trace);
    tui_logger::set_log_file("/tmp/music-manager.log")
        .wrap_err_with(|| "Failed setting log file")?;
    info!("Logger started!");

    // Create channel for IoEvent
    let (sync_io_tx, mut sync_io_rx) = tokio::sync::mpsc::channel::<IoEvent>(100);

    // Create app
    let app = Arc::new(tokio::sync::Mutex::new(App::new(sync_io_tx.clone())));
    // Clone app for IoThread usage
    let app_ui = Arc::clone(&app);

    // Handle I/O
    tokio::spawn(async move {
        debug!("Io thread spawned!");
        let mut handler = IoAsyncHandler::new(app);
        while let Some(io_event) = sync_io_rx.recv().await {
            handler.handle_io_event(io_event).await;
        }
    });

    start_ui(&app_ui).await?;

    Ok(())
}

fn list(_args: &ArgMatches) -> Result<()> {
    let music_dir = directories_next::UserDirs::new().unwrap();
    let music_dir = music_dir.audio_dir().unwrap();
    let database = Database::open_from_path(music_dir.join("database.sqlite"))?;

    let songs = database.query_all_song_data()?;

    println!("List of songs in database:");
    let mut count = 1;
    for song in songs {
        let song_title = song.title.clone().unwrap_or_else(|| "None".to_string());
        let song_id = song.id.unwrap();
        let song_artist = song.artists.unwrap();
        let song_artist = song_artist.first().unwrap();
        println!(
            "{}. {} - {} [ID: {}]",
            count, song_title, song_artist, song_id,
        );
        count += 1;
    }
    Ok(())
}

fn remove(args: &ArgMatches) -> Result<()> {
    let music_dir = directories_next::UserDirs::new().unwrap();
    let music_dir = music_dir
        .audio_dir()
        .ok_or_else(|| eyre!("Couldn't get user music dir."))?;
    let database = Database::open_from_path(music_dir.join("database.sqlite"))?;

    match args.value_of("title") {
        Some(song_title) => {
            println!("Searching via song title...");
            match database.search_song(song_title) {
                Ok(songs) => {
                    println!("Results found!");
                    let to_delete = MultiSelect::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select songs to delete:")
                        .items(&songs)
                        .interact()?;
                    for index in to_delete {
                        let song = songs.get(index).unwrap();
                        let id = song.id.unwrap();
                        println!(
                            "Removing: {} - {} [ID: {}]",
                            song.title.as_ref().unwrap(),
                            song.artists.as_ref().unwrap().join(":"),
                            id
                        );
                        match database.remove_song(id) {
                            Ok(_) => {
                                println!("Song removed from database. Removing old file...");
                                std::fs::remove_file(music_dir.join(&song.file_name))?;
                                println!("File removed.");
                            }
                            Err(e) => {
                                eprintln!("Error: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
        None => {
            let song_id = args.value_of("id").unwrap().parse::<usize>()?;
            let song = database.query_song_by_id(song_id)?;
            let song = song.first().unwrap();

            println!(
                "Removing: {} - {} [ID: {}]",
                song.title.as_ref().unwrap(),
                song.artists.as_ref().unwrap().join(":"),
                song.id.unwrap()
            );
            match database.remove_song(song_id) {
                Ok(_) => {
                    println!("Song removed from database. Removing old file...");
                    std::fs::remove_file(music_dir.join(&song.file_name))?;
                    println!("File removed.");
                }
                Err(e) => {
                    eprintln!("Error: {}", e)
                }
            }
        }
    }
    Ok(())
}

fn search(args: &ArgMatches) -> Result<()> {
    let song_title = args.value_of("title").unwrap();
    let music_dir = directories_next::UserDirs::new().unwrap();
    let music_dir = music_dir
        .audio_dir()
        .ok_or_else(|| eyre!("Couldn't get user music dir."))?;
    let database = Database::open_from_path(music_dir.join("database.sqlite"))?;

    match database.search_song(song_title) {
        Ok(songs) => {
            println!("Results found: ");
            let mut count = 1;

            for song in songs {
                let song_title = song.title.clone().unwrap();
                let song_artist = song.artists.clone().unwrap().join(":");
                let song_id = song.id.unwrap();
                let path = song.file_path;
                println!(
                    "{}. {} - {} [ID: {}]",
                    count, song_title, song_artist, song_id
                );
                println!("\tPath: {}", path.display());
                count += 1;
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}
