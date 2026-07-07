use std::{
    fs::{create_dir_all, read_to_string},
    path::Path,
    sync::Arc,
};

use anyhow::Context;
use downloader::{progress::Reporter, Download, Downloader};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};

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

fn parse_checksum(content: &str, filename: &str) -> Option<String> {
    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').map(|s| s.trim()).collect();
        if parts.len() == 2 && parts[1].to_lowercase() == filename.to_lowercase() {
            return Some(parts[0].to_string());
        }
    }
    None
}

fn sha256_of_file(path: &Path) -> anyhow::Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Could not read file: {}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(hex::encode(digest))
}

fn verify_checksum(
    checksum_path: &Path,
    yt_dlp_path: &Path,
    filename: &str,
) -> anyhow::Result<String> {
    let content = read_to_string(checksum_path)
        .with_context(|| format!("Could not read checksum file: {}", checksum_path.display()))?;
    let expected = parse_checksum(&content, filename)
        .with_context(|| format!("Could not find checksum for {}", filename))?;

    let got = sha256_of_file(yt_dlp_path)?;
    if got != expected {
        anyhow::bail!("Checksum mismatch: expected {} got {}", expected, got);
    }
    Ok(got)
}

pub fn ytdlp_path(config_dir: &Path) -> std::path::PathBuf {
    config_dir.join(ytdlp_file_name())
}

pub fn download_ytdlp(config_dir: &Path) -> anyhow::Result<()> {
    // Create config dir if not exists
    if !config_dir.exists() {
        create_dir_all(config_dir)?;
    }

    let ytdlp_sha256checksum_path = config_dir.join("SHA2-256SUMS");
    let ytdlp_path = ytdlp_path(config_dir);

    if ytdlp_path.exists() {
        // yt-dlp already exists, no need to download
        return Ok(());
    }

    let mut downloader = Downloader::builder()
        .download_folder(config_dir)
        .parallel_requests(8)
        .build()?;

    let yt_dlp_release_url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/";
    let yt_dlp_sha256checksum = "SHA2-256SUMS";
    let yt_dlp_binary = ytdlp_release_asset();

    let yt_dlp_url = format!("{}{}", yt_dlp_release_url, yt_dlp_binary);
    let yt_dlp_sha256_url = format!("{}{}", yt_dlp_release_url, yt_dlp_sha256checksum);

    // download for yt-dlp binary
    let dl = Download::new(&yt_dlp_url)
        .file_name(&ytdlp_path)
        .progress(Arc::new(DownloadProgress::new(0)));

    // download for yt-dlp SHA2-256 checksum
    let dl_sha256 = Download::new(&yt_dlp_sha256_url)
        .file_name(&ytdlp_sha256checksum_path)
        .progress(Arc::new(DownloadProgress::new(0)));

    let result = downloader.download(&[dl, dl_sha256])?;

    let hash = verify_checksum(&ytdlp_sha256checksum_path, &ytdlp_path, yt_dlp_binary)
        .inspect_err(|_| {
            // remove the downloaded file if the checksum verification fails
            let _ = std::fs::remove_file(&ytdlp_path);
        })
        .context("Checksum verification failed. Removed potentially corrupt binary.")?;

    println!("Checksum verified: {hash}");

    // helper function to convert path to file name string
    fn file_name_str(path: &Path) -> &str {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
    }

    for r in result {
        match r {
            Err(e) => println!("Error: {e}"),
            Ok(s) => {
                println!("Download Success: {}", &s);

                let name_from_downloader = file_name_str(&s.file_name);
                let yt_dlp_name = file_name_str(&ytdlp_path);
                println!(
                    "Name from downloader: {} == {}",
                    &name_from_downloader, &yt_dlp_name
                );
                if name_from_downloader == yt_dlp_name {
                    make_executable(&ytdlp_path)?;
                }
            }
        };
    }

    Ok(())
}

fn ytdlp_file_name() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

fn ytdlp_release_asset() -> &'static str {
    if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp_linux"
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> anyhow::Result<()> {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, Permissions::from_mode(0o775))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}
