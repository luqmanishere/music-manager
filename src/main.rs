use std::{path::Path, sync::Arc};

use clap::{crate_authors, crate_version, App as CApp, AppSettings, Arg, ArgMatches};
use edit::{
    app::App,
    io::{handler::IoAsyncHandler, IoEvent},
    start_ui,
};
use eyre::{eyre, Context, Result};
use log::{debug, info};
use youtube_dl::{
    SearchOptions, SingleVideo as Video, YoutubeDl,
    YoutubeDlOutput::{Playlist, SingleVideo},
};

mod edit;

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
            )?;
        }
        Some("edit") => {
            edit(
                matches
                    .subcommand_matches("edit")
                    .ok_or_else(|| eyre!("No arguments gave to subcommand edit"))?,
            )
            .await?;
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
                        .index(1),
                ),
        )
        .subcommand(CApp::new("edit").about("Edit song library"))
        .get_matches()
}

fn download(args: &ArgMatches) -> Result<()> {
    let music_dir = directories_next::UserDirs::new()
        .ok_or_else(|| eyre!("directories_next failed to initialize"))?;
    let music_dir = music_dir
        .audio_dir()
        .ok_or_else(|| eyre!("directories_next failed to retrieve music dir"))?
        .to_path_buf();
    let title = args
        .value_of("title")
        .ok_or_else(|| eyre!("Song title is not given"))?;
    let search_options = SearchOptions::youtube(title).with_count(5);
    let ytsearch = YoutubeDl::search_for(&search_options)
        .socket_timeout("10")
        .run()?;

    match ytsearch {
        Playlist(playlist) => {
            let mut count = 1;
            let entries = playlist
                .entries
                .ok_or_else(|| eyre!("Can't get video entries"))?;
            for video in &entries {
                println!(
                    "{}. Title: {}, Channel:{}",
                    count,
                    video.title,
                    video.channel.as_ref().unwrap()
                );
                count += 1;
            }
            if !args.is_present("search-only") {
                println!("Choose a video to download from the list");
                let mut option_picked = String::new();
                std::io::stdin().read_line(&mut option_picked)?;
                let option = option_picked
                    .trim()
                    .parse::<usize>()
                    .wrap_err_with(|| eyre!("Can't convert input into usize"))?;

                let output_format = music_dir.join("%(title)s.%(ext)s");
                let video = &entries
                    .get(option - 1)
                    .ok_or_else(|| eyre!("Can't get entry number: {}", option))?;

                let mut video_title = video.title.replace("/", "_").replace(":", " -");
                video_title.push_str(".opus");

                let mut filename_opus = music_dir.join(&video_title);
                filename_opus.set_extension("opus");
                let filename_flac = filename_opus.with_extension("flac");

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
                    ffmpeg_convert_to_flac(&filename_opus, &filename_flac)?;
                } else {
                    // If opus file does not exist
                    println!("Song is already downloaded");
                }
            }
        }
        SingleVideo(video) => {
            println!("Title: {}, Channel:{}", video.title, video.channel.unwrap())
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
