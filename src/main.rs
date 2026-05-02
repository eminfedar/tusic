mod audio;
mod cli;
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

use crate::{audio::rodio::RodioBackend, ui::App};

fn main() -> anyhow::Result<()> {
    let _args = Args::parse();

    ratatui::run(|terminal| {
        let backend = RodioBackend::new()?;
        let mut app = App::new(backend)?;
        app.run(terminal)
    })?;

    Ok(())
}
