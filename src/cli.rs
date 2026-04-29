use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tusic", author, version, about, long_about)]
#[command(version = "0.1.0")]
#[command(about = "A terminal music player", long_about = None)]
pub struct Args {
    #[arg(short, long, help = "Path to scan for music files")]
    pub path: Option<PathBuf>,

    #[arg(short, long, help = "Do not scan directories automatically")]
    pub no_scan: bool,

    #[arg(short, long, help = "Recursive scan depth (default: 10)")]
    pub depth: Option<usize>,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Verbose output")]
    pub verbose: bool,
}
