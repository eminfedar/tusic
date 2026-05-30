mod app;
mod audio;
mod cli;
mod config;
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

    let config = Config::load();

    // Build the YouTube service *before* taking over the terminal: the first
    // run downloads the yt-dlp binary (~36MB) and prints progress, which would
    // be invisible (and look like a freeze) once the TUI owns the screen.
    let download_dir = config.download_dir();
    let yt_service = smol::block_on(async_compat::Compat::new(YoutubeService::new(download_dir)))?;

    let backend = RodioBackend::new()?;
    let mut app = App::new(backend, yt_service, config)?;

    ratatui::run(|terminal| app.run(terminal))?;

    Ok(())
}
