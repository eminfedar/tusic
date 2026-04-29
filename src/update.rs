use std::fs;

use crate::audio::AudioBackend;
use crate::model::{ActivePanel, Model, PlaybackStatus};
use crate::msg::Message;
use crate::playlist::{RepeatMode, Track};
use crate::task::Task;
use crate::youtube::{get_scan_paths, YoutubeService};
use anyhow::Result;

pub fn update<T: AudioBackend>(
    model: &mut Model,
    msg: Message,
    player: &mut T,
    yt_service: &YoutubeService,
    task: &Task<Message>,
) -> Result<()> {
    match msg {
        Message::Play => {
            if model.playback.status != PlaybackStatus::Playing
                && model.playback.position_ms < model.playback.duration_ms
            {
                player.play()?;
                model.playback.status = PlaybackStatus::Playing;
            }
        }

        Message::Pause => {
            if model.playback.status == PlaybackStatus::Playing {
                player.pause();
                model.playback.position_ms = player.get_position() - 100;
                model.playback.status = PlaybackStatus::Paused;
            }
        }

        Message::Next => {
            let next_idx =
                model
                    .playlist
                    .next_index(model.current_index, model.repeat.clone(), model.shuffle);
            if let Some(idx) = next_idx {
                play_track(idx, model, player);
            }
        }

        Message::Prev => {
            let prev_idx = model
                .playlist
                .prev_index(model.current_index, model.repeat.clone());

            if let Some(idx) = prev_idx {
                play_track(idx, model, player);
            }
        }

        Message::SeekForward => {
            let new_pos = model.playback.position_ms + 5000;
            let total_duration = model.playback.duration_ms;

            // if is there a song loaded, total_duration should be higher than 100
            if total_duration > 100 {
                let target = new_pos.clamp(0, total_duration.saturating_sub(100));

                if let Err(e) = player.seek_to(target) {
                    model.add_log(&format!("Seek error: {}", e));
                }

                model.add_log(&format!("player.is_paused:{}", player.is_playing()));

                model.playback.position_ms = target;
                model.add_log(&format!("Seek to: {}", target));
            }
        }

        Message::SeekBackward => {
            let new_pos = model.playback.position_ms.saturating_sub(5000);

            if let Err(e) = player.seek_to(new_pos) {
                model.add_log(&format!("Seek error: {}", e));
            }

            model.playback.position_ms = new_pos;
            model.add_log(&format!("Seek to: {}", new_pos));
        }

        Message::IncreaseVolume => {
            let vol = (model.volume + 5).clamp(0, 100);
            player.set_volume(vol);
            model.volume = vol;
        }
        Message::DecreaseVolume => {
            let vol = (model.volume - 5).clamp(0, 100);
            player.set_volume(vol);
            model.volume = vol;
        }

        Message::ToggleShuffle => {
            model.shuffle = !model.shuffle;
        }

        Message::CycleRepeat => {
            model.repeat = model.repeat.next();
        }

        Message::ScrollUp => {
            match model.ui.active_panel {
                ActivePanel::Playlist => {
                    if model.ui.selected > 0 {
                        model.ui.selected -= 1;
                        adjust_scroll(model);
                    }
                }
                ActivePanel::SearchInput => {
                    // Do nothing - navigation only in search results
                }
                ActivePanel::SearchResults => {
                    if model.ui.search_selected > 0 {
                        model.ui.search_selected -= 1;
                        adjust_search_scroll(model);
                    }
                }
            }
        }

        Message::ScrollDown => {
            match model.ui.active_panel {
                ActivePanel::Playlist => {
                    if model.ui.selected < model.playlist.len().saturating_sub(1) {
                        model.ui.selected += 1;
                        adjust_scroll(model);
                    }
                }
                ActivePanel::SearchInput => {
                    // Do nothing - navigation only in search results
                }
                ActivePanel::SearchResults => {
                    if model.ui.search_selected < model.search.results.len().saturating_sub(1) {
                        model.ui.search_selected += 1;
                        adjust_search_scroll(model);
                    }
                }
            }
        }

        Message::ScrollUpHalf => {
            match model.ui.active_panel {
                ActivePanel::Playlist => {
                    let new_selected = model.ui.selected.saturating_sub(10);
                    model.ui.selected = new_selected;
                    adjust_scroll(model);
                }
                ActivePanel::SearchInput => {
                    // Do nothing
                }
                ActivePanel::SearchResults => {
                    let new_selected = model.ui.search_selected.saturating_sub(10);
                    model.ui.search_selected = new_selected;
                    adjust_search_scroll(model);
                }
            }
        }

        Message::ScrollDownHalf => {
            match model.ui.active_panel {
                ActivePanel::Playlist => {
                    let new_selected =
                        (model.ui.selected + 10).min(model.playlist.len().saturating_sub(1));
                    model.ui.selected = new_selected;
                    adjust_scroll(model);
                }
                ActivePanel::SearchInput => {
                    // Do nothing
                }
                ActivePanel::SearchResults => {
                    let new_selected = (model.ui.search_selected + 10)
                        .min(model.search.results.len().saturating_sub(1));
                    model.ui.search_selected = new_selected;
                    adjust_search_scroll(model);
                }
            }
        }

        Message::ScrollTop => {
            match model.ui.active_panel {
                ActivePanel::Playlist => {
                    model.ui.selected = 0;
                    model.ui.scroll = 0;
                }
                ActivePanel::SearchInput => {
                    // Do nothing
                }
                ActivePanel::SearchResults => {
                    model.ui.search_selected = 0;
                    model.ui.search_scroll = 0;
                }
            }
        }

        Message::ScrollBottom => {
            match model.ui.active_panel {
                ActivePanel::Playlist => {
                    let list_len = model.playlist.len();
                    model.ui.selected = list_len.saturating_sub(1);
                    model.ui.scroll = model.ui.selected.saturating_sub(19);
                }
                ActivePanel::SearchInput => {
                    // Do nothing
                }
                ActivePanel::SearchResults => {
                    let results_len = model.search.results.len();
                    model.ui.search_selected = results_len.saturating_sub(1);
                    model.ui.search_scroll = model.ui.search_selected.saturating_sub(19);
                }
            }
        }

        Message::Enter => {
            if matches!(model.ui.active_panel, ActivePanel::SearchInput) {
                if !model.search.query.is_empty() {
                    model.search.is_loading = true;
                }
            } else {
                match model.ui.active_panel {
                    ActivePanel::Playlist => {
                        play_track(model.ui.selected, model, player);
                    }
                    ActivePanel::SearchInput => {}
                    ActivePanel::SearchResults => {
                        if model.ui.search_selected < model.search.results.len() {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Message::Escape => {
            if matches!(
                model.ui.active_panel,
                ActivePanel::SearchInput | ActivePanel::SearchResults
            ) {
                model.ui.active_panel = ActivePanel::Playlist;
            } else {
                model.ui.selected = model.current_index.unwrap_or(0);
            }
        }

        Message::ToggleHelp => {
            model.ui.show_help = !model.ui.show_help;
        }

        Message::ToggleLogs => {
            model.ui.show_logs = !model.ui.show_logs;
        }

        Message::ToggleYoutube => {
            model.ui.show_youtube = !model.ui.show_youtube;
            if !model.ui.show_youtube {
                model.ui.active_panel = ActivePanel::Playlist;
            }
        }

        Message::ToggleActivePanel => {
            if model.ui.show_youtube {
                match model.ui.active_panel {
                    ActivePanel::Playlist => model.ui.active_panel = ActivePanel::SearchInput,
                    ActivePanel::SearchInput => model.ui.active_panel = ActivePanel::SearchResults,
                    ActivePanel::SearchResults => model.ui.active_panel = ActivePanel::Playlist,
                }
            } else {
                model.ui.active_panel = ActivePanel::Playlist;
            }
        }

        Message::SearchInput(c) => {
            model.search.query.push(c);
        }

        Message::SearchBackspace => {
            model.search.query.pop();
        }

        Message::DoYoutubeSearch(query) => {
            if !query.is_empty() {
                model.search.is_loading = true;
                model.add_log(&format!("Searching YouTube: {}", &query));
                model.ui.active_panel = ActivePanel::SearchInput;

                let service = yt_service.clone();

                task.spawn(async move {
                    let result = service.search(&query, 10).await;

                    Message::YoutubeSearchResult(result)
                });
            }
        }
        Message::YoutubeSearchResult(result) => {
            model.search.is_loading = false;

            match result {
                Ok(tracks) => {
                    model.search.results = tracks;
                    model.ui.active_panel = ActivePanel::SearchResults;

                    model.add_log(&format!("{:?}", model.search.results));
                    model.add_log(&format!("Found {} tracks", model.search.results.len()));
                }
                Err(e) => model.add_log(&format!("Search error: {e:?}")),
            }
        }

        Message::DownloadYoutube(idx) => {
            if idx < model.search.results.len() {
                if model.search.is_downloading {
                    model.add_log("Already downloading, please wait...");
                    return Ok(());
                }

                model.search.is_downloading = true;

                let track = model.search.results[idx].clone();
                let track_title = track.title.clone();

                let service = yt_service.clone();

                task.spawn(async move {
                    let result = service.download_track(&track).await;

                    Message::YoutubeDownloadResult(result)
                });

                model.add_log(&format!("Downloading: {}", track_title));
            }
        }

        Message::YoutubeDownloadResult(result) => {
            model.search.is_downloading = false;

            match result {
                Ok(track) => {
                    let idx = model.playlist.push(track.clone());

                    model.current_index = Some(idx);
                    play_track(idx, model, player);

                    model.add_log(&format!("Downloaded: {}", track.display_name()));
                }
                Err(e) => model.add_log(&format!("Download error: {e}")),
            }
        }

        Message::LogScrollUp => {
            if model.ui.show_logs && model.ui.log_selected > 0 {
                model.ui.log_selected -= 1;
                adjust_log_scroll(model);
            }
        }

        Message::LogScrollDown => {
            if model.ui.show_logs {
                let log_count = model.ui.log_messages.len();
                if model.ui.log_selected < log_count.saturating_sub(1) {
                    model.ui.log_selected += 1;
                    adjust_log_scroll(model);
                }
            }
        }

        Message::LogScrollTop => {
            if model.ui.show_logs {
                model.ui.log_selected = 0;
                model.ui.log_scroll = 0;
            }
        }

        Message::LogScrollBottom => {
            if model.ui.show_logs {
                model.ui.log_selected = model.ui.log_messages.len().saturating_sub(1);
                adjust_log_scroll(model);
            }
        }

        Message::FileChanged(e) => {
            model.add_log(&format!("{e:?}"));

            let playlist = read_tracks();
            model.set_tracks(playlist);
        }

        Message::Tick | Message::None => {
            if model.playback.status == PlaybackStatus::Playing {
                let pos = player.get_position();
                model.playback.position_ms = pos;

                // let max_pos = model.playback.duration_ms;
                // model.add_log(&format!("Player pos : {pos} / {max_pos}"));

                if model.playback.position_ms >= model.playback.duration_ms.saturating_sub(100) {
                    if model.repeat == RepeatMode::One {
                        model.add_log("Loop mode: restarting same track");
                        play_track(model.current_index.unwrap_or(0), model, player);
                    } else {
                        model.add_log("Song ended, playing next track");
                        let next_idx = model.playlist.next_index(
                            model.current_index,
                            model.repeat.clone(),
                            model.shuffle,
                        );
                        if let Some(idx) = next_idx {
                            play_track(idx, model, player);
                        } else {
                            model.playback.status = PlaybackStatus::Paused;
                            player.pause();
                        }
                    }
                }
            }
        }

        Message::Quit => {}
    }

    Ok(())
}

fn play_track<T: AudioBackend>(idx: usize, model: &mut Model, player: &mut T) {
    let track = match model.playlist.get(idx) {
        Some(t) => t.clone(),
        None => return,
    };

    model.current_index = Some(idx);
    model.ui.playback_error = None;
    model.playback.position_ms = 0;

    match player.load_track(&track.path) {
        Ok(_) => (),
        Err(e) => {
            let err_msg = format!("Failed to load track: {}", e);
            model.add_log(&err_msg);
            model.ui.playback_error = Some(err_msg);

            player.stop();
            model.playback.status = PlaybackStatus::Stopped;

            return;
        }
    }

    model.playback.duration_ms = player.get_duration();
    player.set_volume(model.volume);

    match player.play() {
        Ok(_) => {
            model.playback.status = PlaybackStatus::Playing;
        }
        Err(e) => {
            let err_msg = format!("Failed to play: {}", e);
            model.add_log(&err_msg);
            model.ui.playback_error = Some(err_msg);

            player.stop();
            model.playback.status = PlaybackStatus::Stopped;
        }
    }
}

fn adjust_scroll(model: &mut Model) {
    if model.ui.selected >= model.ui.scroll + 20 {
        model.ui.scroll = model.ui.selected.saturating_sub(19);
    } else if model.ui.selected < model.ui.scroll {
        model.ui.scroll = model.ui.selected;
    }
}

fn adjust_search_scroll(model: &mut Model) {
    if model.ui.search_selected >= model.ui.search_scroll + 20 {
        model.ui.search_scroll = model.ui.search_selected.saturating_sub(19);
    } else if model.ui.search_selected < model.ui.search_scroll {
        model.ui.search_scroll = model.ui.search_selected;
    }
}

fn adjust_log_scroll(model: &mut Model) {
    if model.ui.log_selected >= model.ui.log_scroll + 20 {
        model.ui.log_scroll = model.ui.log_selected.saturating_sub(19);
    } else if model.ui.log_selected < model.ui.log_scroll {
        model.ui.log_scroll = model.ui.log_selected;
    }
}

pub fn read_tracks() -> Vec<Track> {
    use crate::playlist::is_audio_file;

    let mut tracks = Vec::new();

    let paths = get_scan_paths();

    for p in paths {
        for entry in fs::read_dir(p).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            if is_audio_file(&path) && !tracks.iter().any(|t: &Track| t.path == path) {
                tracks.push(crate::playlist::Track::new(path));
            }
        }
    }

    tracks.sort_by(|a, b| {
        a.display_name()
            .to_lowercase()
            .cmp(&b.display_name().to_lowercase())
    });

    tracks
}
