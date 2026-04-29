use crate::playlist::Track;
use anyhow::Error;
use rusty_ytdl::search::{SearchOptions, SearchResult};
use std::path::PathBuf;
use yt_dlp::{
    client::{Libraries, LibraryInstaller},
    Downloader, VideoSelection,
};

pub fn get_download_dir() -> PathBuf {
    if let Some(mut dir) = dirs::audio_dir() {
        dir.push("tusic");
        if !dir.exists() {
            std::fs::create_dir_all(&dir).ok();
        }
        dir
    } else if let Ok(cwd) = std::env::current_dir() {
        let dir = cwd.join("downloads");
        if !dir.exists() {
            std::fs::create_dir_all(&dir).ok();
        }
        dir
    } else {
        PathBuf::from(".")
    }
}

pub fn get_scan_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Default music dir
    if let Some(music_dir) = dirs::audio_dir() {
        if music_dir.exists() {
            paths.push(music_dir);
        }
    }

    // Downloaded songs dir
    paths.push(get_download_dir());

    paths
}

#[derive(Clone, Debug, PartialEq)]
pub struct YoutubeTrack {
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub duration_ms: u64,
}

#[derive(Debug, Default)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<YoutubeTrack>,
    pub is_loading: bool,
    pub is_downloading: bool,
}

impl YoutubeTrack {
    pub fn to_track(&self, path: PathBuf) -> Track {
        let mut track = Track::new(path);
        track.title = self.title.clone();
        track.artist = self.channel.clone();
        track.duration_ms = self.duration_ms;
        track
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '|' | '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' => '_',
            _ => c,
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct YoutubeService {
    downloader: Downloader,
}

impl YoutubeService {
    pub async fn new(download_dir: PathBuf) -> anyhow::Result<Self> {
        let config_base = dirs::config_dir().unwrap_or(".".into()).join("tusic");

        // Install yt-dlp:
        let ytdlp_path = config_base.clone().join("yt-dlp");
        if !ytdlp_path.exists() {
            let installer = LibraryInstaller::new(config_base);
            println!(
                "Downloading yt-dlp binary, please wait (this may take a minute, size: 36MB~)"
            );
            installer.install_youtube(None).await?;
        }

        let libraries = Libraries::new(ytdlp_path, PathBuf::from("ffmpeg"));

        let downloader = Downloader::builder(libraries, download_dir).build().await?;

        Ok(Self { downloader })
    }

    pub async fn search(&self, query: &str, limit: u64) -> anyhow::Result<Vec<YoutubeTrack>> {
        // Access YouTube-specific features
        let youtube = rusty_ytdl::search::YouTube::new().unwrap();

        let result = youtube
            .search(
                query,
                Some(&SearchOptions {
                    limit,
                    search_type: rusty_ytdl::search::SearchType::Video,
                    safe_search: true,
                }),
            )
            .await?;

        let tracks = result
            .iter()
            .filter_map(|e| match e {
                SearchResult::Video(v) => Some(YoutubeTrack {
                    channel: v.channel.name.clone(),
                    video_id: v.id.clone(),
                    title: v.title.clone(),
                    duration_ms: v.duration,
                }),
                _ => None,
            })
            .collect();

        Ok(tracks)
    }

    pub async fn download_track(&self, track: &YoutubeTrack) -> anyhow::Result<Track> {
        let safe_title = sanitize_filename(&track.title);
        let mut filename = format!("{}.m4a", safe_title);

        let video = self
            .downloader
            .fetch_video_infos(&format!(
                "https://www.youtube.com/watch?v={}",
                track.video_id
            ))
            .await?;

        let mut format = video.select_audio_format(
            yt_dlp::model::AudioQuality::Best,
            yt_dlp::model::AudioCodecPreference::AAC,
        );

        if format.is_none() {
            format = video.select_audio_format(
                yt_dlp::model::AudioQuality::High,
                yt_dlp::model::AudioCodecPreference::MP3,
            );
            filename = format!("{}.mp3", safe_title);
        }

        match format {
            Some(f) => {
                let video_path = self
                    .downloader
                    .download_format(f, filename)
                    //.download_audio_stream(&video, &filename)
                    .await?;

                Ok(track.to_track(video_path))
            }
            None => Err(Error::msg("Can't find m4a or mp3 format")),
        }
    }
}
