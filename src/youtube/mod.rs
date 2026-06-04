use crate::playlist::Track;
use anyhow::Error;
use rusty_ytdl::search::{SearchOptions, SearchResult};
use std::path::{Path, PathBuf};
use yt_dlp::{
    client::{Libraries, LibraryInstaller},
    Downloader, VideoSelection,
};

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
    /// Set when the last search failed (e.g. no network). Shown in the results
    /// panel and cleared when a new search starts or succeeds.
    pub error: Option<String>,
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
    pub async fn new(download_dir: PathBuf, config_dir: &Path) -> anyhow::Result<Self> {
        // Install yt-dlp:
        let ytdlp_path = config_dir.join("yt-dlp");
        if !ytdlp_path.exists() {
            let installer = LibraryInstaller::new(config_dir.to_path_buf());
            println!(
                "Downloading yt-dlp binary, please wait (this may take a minute, size: 36MB~)"
            );
            installer.install_youtube(None).await?;
        }

        let libraries = Libraries::new(ytdlp_path, PathBuf::from("ffmpeg"));

        // The builder's output_dir is just an initial default; each download
        // targets an explicit absolute path (see `download_track`), so the
        // directory can change at runtime without rebuilding the downloader.
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

    pub async fn download_track(&self, track: &YoutubeTrack, dir: &Path) -> anyhow::Result<Track> {
        let safe_title = sanitize_filename(&track.title);
        let safe_channel = sanitize_filename(&track.channel);
        // Name the file "Title - Channel" so it reads nicely in the library.
        let mut filename = format!("{} - {}.m4a", safe_title, safe_channel);

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
            filename = format!("{} - {}.mp3", safe_title, safe_channel);
        }

        match format {
            Some(f) => {
                // Download straight into the configured directory.
                let target = dir.join(filename);
                let video_path = self.downloader.download_format_to_path(f, &target).await?;

                Ok(track.to_track(video_path))
            }
            None => Err(Error::msg("Can't find m4a or mp3 format")),
        }
    }
}
