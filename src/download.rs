use std::{
    fs::{create_dir_all, Permissions},
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::Arc,
};

use downloader::{progress::Reporter, Download, Downloader};
use indicatif::{ProgressBar, ProgressStyle};

struct DownloadProgress {
    pb: ProgressBar,
}

impl DownloadProgress {
    fn new(max_progress: u64) -> Self {
        let pb = ProgressBar::new(max_progress);

        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .unwrap()
                .progress_chars("#>-"));

        Self { pb }
    }
}

impl Reporter for DownloadProgress {
    fn done(&self) {
        self.pb.println("done!");
    }
    fn progress(&self, current: u64) {
        self.pb.set_position(current);
    }
    fn set_message(&self, message: &str) {
        self.pb.println(message);
    }
    fn setup(&self, max_progress: Option<u64>, _message: &str) {
        self.pb.set_length(max_progress.unwrap_or(0));
        self.pb.println("Downloading yt-dlp...");
    }
}

pub fn download_ytdlp(config_dir: &Path) -> anyhow::Result<()> {
    // Create config dir if not exists
    if !config_dir.exists() {
        create_dir_all(config_dir)?;
    }

    let ytdlp_path = config_dir.join("yt-dlp");
    if ytdlp_path.exists() {
        // yt-dlp already exists, no need to download
        return Ok(());
    }

    let mut downloader = Downloader::builder()
        .download_folder(config_dir)
        .parallel_requests(8)
        .build()?;

    let dl =
        Download::new("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux")
            .file_name(&ytdlp_path)
            .progress(Arc::new(DownloadProgress::new(0)));

    let result = downloader.download(&[dl])?;

    for r in result {
        match r {
            Err(e) => println!("Error: {e}"),
            Ok(s) => {
                std::fs::set_permissions(&ytdlp_path, Permissions::from_mode(0o0775))?;
                println!("Download Success: {}", &s);
            }
        };
    }

    Ok(())
}
