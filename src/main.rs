mod app;
mod audio;
mod cli;
mod config;
mod download;
mod model;
mod msg;
mod playlist;
mod task;
mod ui;
mod update;
mod watcher;
mod youtube;

use clap::Parser;
use cli::Args;

use crate::{app::App, audio::rodio::RodioBackend, config::Config, youtube::YoutubeService};

fn main() -> anyhow::Result<()> {
    let _args = Args::parse();

    // Download yt_dlp if not downloaded:
    let config_dir = config::Config::config_dir();
    download::download_ytdlp(&config_dir)?;

    // Youtube Download and Config path
    let config = Config::load();
    let music_download_dir = config.download_dir();
    let yt_service = smol::block_on(async_compat::Compat::new(YoutubeService::new(
        music_download_dir,
        &config_dir,
    )))?;

    let backend = RodioBackend::new()?;
    let mut app = App::new(backend, yt_service, config)?;

    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}
