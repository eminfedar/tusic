use crate::audio::AudioBackend;
use crate::config::Config;
use crate::model::{ActivePanel, Model, SettingsField};
use crate::msg::Message;
use crate::ui::layout::calculate_layout;
use crate::ui::logs::{self, LogsState};
use crate::ui::playlist::{self, PlaylistState};
use crate::ui::search::SearchResultsState;
use crate::ui::{help, popup, settings};
use crate::ui::{player, search};
use crate::update::{read_tracks, update};
use crate::watcher::Watcher;
use crate::youtube::YoutubeService;
use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata};

use std::sync::mpsc::{self, Receiver, Sender};

use crate::task::Task;

pub struct App<T: AudioBackend> {
    player: T,
    yt_service: YoutubeService,
    running: bool,
    config: Config,
    playlist_state: PlaylistState,
    search_results_state: SearchResultsState,
    logs_state: LogsState,
    task: Task<Message>,
    task_rx: Receiver<Message>,
    media_controls: Option<MediaControls>,
    watcher: Watcher,
}

impl<T: AudioBackend> App<T> {
    pub fn new(backend: T, yt_service: YoutubeService, config: Config) -> Result<Self> {
        let (task_tx, task_rx) = mpsc::channel::<Message>();

        // File change watcher, pointed at the configured music directories.
        let watcher = Watcher::new(config.resolved_dirs(), task_tx.clone())?;

        // Media controls are best-effort: on platforms without a session/D-Bus
        // they fail to initialize and we simply run without them.
        let media_controls = Self::init_media_controls(task_tx.clone());

        // Create
        Ok(Self {
            player: backend,
            yt_service,
            running: false,
            config,
            playlist_state: PlaylistState::default(),
            search_results_state: SearchResultsState::default(),
            logs_state: LogsState::default(),
            task: Task::new(task_tx),
            task_rx,
            media_controls,
            watcher,
        })
    }

    fn init_media_controls(task_tx: Sender<Message>) -> Option<MediaControls> {
        let mut media_controls = MediaControls::new(souvlaki::PlatformConfig {
            dbus_name: "tusic",
            display_name: "Tusic",
            hwnd: None,
        })
        .ok()?;

        // The closure must be Send and have a static lifetime
        media_controls
            .attach(move |event: MediaControlEvent| {
                let msg = match event {
                    MediaControlEvent::Next => Message::Next,
                    MediaControlEvent::Previous => Message::Prev,
                    MediaControlEvent::Toggle => Message::PlayPause,
                    MediaControlEvent::Play => Message::Play,
                    MediaControlEvent::Pause => Message::Pause,
                    MediaControlEvent::Stop => Message::Pause,

                    _ => Message::None,
                };

                let _ = task_tx.send(msg);
            })
            .ok()?;

        media_controls.set_metadata(MediaMetadata::default()).ok()?;

        Some(media_controls)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.running = true;

        let mut model = Model::new(self.config.clone());

        let playlist = read_tracks(&model.config);
        model.set_tracks(playlist);

        while self.running {
            // Keep scroll viewports in sync with the real rendered layout so
            // paging/scrolling is correct at any terminal size.
            let size = terminal.size()?;
            let regions = calculate_layout(Rect::new(0, 0, size.width, size.height), &model);
            // Each scrollable panel is wrapped in a bordered Block (inner = height - 2).
            model.ui.playlist_viewport = regions.playlist.height.saturating_sub(2) as usize;
            model.ui.search_viewport = regions.search_results.height.saturating_sub(2) as usize;
            model.ui.log_viewport = regions
                .logs
                .map(|r| r.height.saturating_sub(2) as usize)
                .unwrap_or(0);

            // Draw
            terminal.draw(|f| self.render_frame(f, &model))?;

            // Input Events
            let mut msg = self.handle_events(&mut model)?;
            if matches!(msg, Message::Quit) {
                break;
            }

            // Update from Input Events
            msg = update(
                &mut model,
                msg,
                &mut self.player,
                &self.yt_service,
                &self.task,
                &mut self.media_controls,
                &mut self.watcher,
            )?;

            while !matches!(msg, Message::None | Message::Tick) {
                // Handle other messages come from previous messages
                msg = update(
                    &mut model,
                    msg,
                    &mut self.player,
                    &self.yt_service,
                    &self.task,
                    &mut self.media_controls,
                    &mut self.watcher,
                )?;
            }

            // Handle results from async Tasks
            while let Ok(m) = self.task_rx.try_recv() {
                let mut msg2 = m;

                while !matches!(msg2, Message::None | Message::Tick) {
                    // Handle other messages come from previous messages
                    msg2 = update(
                        &mut model,
                        msg2,
                        &mut self.player,
                        &self.yt_service,
                        &self.task,
                        &mut self.media_controls,
                        &mut self.watcher,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn handle_events(&mut self, model: &mut Model) -> Result<Message> {
        if event::poll(std::time::Duration::from_millis(40))? {
            if let Event::Key(key) = event::read()? {
                match key.kind {
                    event::KeyEventKind::Press => return Ok(self.handle_key(key, model)),
                    event::KeyEventKind::Repeat => return Ok(Message::None),
                    event::KeyEventKind::Release => return Ok(Message::None),
                }
            }

            /*
            if let Event::Mouse(mouse_event) = event::read()? {
                return Ok(self.handle_mouse(mouse_event, model));
            }
            */
        }

        Ok(Message::Tick)
    }

    fn handle_key(&mut self, key: KeyEvent, model: &mut Model) -> Message {
        let active_panel = model.ui.active_panel.clone();
        let in_logs = model.ui.show_logs;
        let active_playlist = matches!(active_panel, ActivePanel::Playlist);
        let active_search_input = matches!(active_panel, ActivePanel::SearchInput);
        let active_search_results = matches!(active_panel, ActivePanel::SearchResults);
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // The delete-confirmation popup captures all input while open.
        if model.ui.confirm_delete.is_some() {
            return match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    Message::ConfirmDeleteTrack
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    Message::CancelDeleteTrack
                }
                _ => Message::None,
            };
        }

        // The Settings popup captures all input while open.
        if model.ui.show_settings {
            let field = model.ui.settings.field.clone();
            return match field {
                // Text input for adding a new directory. Arrows still navigate
                // between settings; printable chars edit the input.
                SettingsField::NewDir => match key.code {
                    KeyCode::Esc => Message::ToggleSettings,
                    KeyCode::Up => Message::SettingsNavUp,
                    KeyCode::Down => Message::SettingsNavDown,
                    // Enter adds the typed directory (or saves if the box is empty).
                    KeyCode::Enter if model.ui.settings.new_dir.trim().is_empty() => {
                        Message::SettingsSave
                    }
                    KeyCode::Enter => Message::SettingsAddDir,
                    KeyCode::Backspace => Message::SettingsBackspace,
                    KeyCode::Char(c) => Message::SettingsInput(c),
                    _ => Message::None,
                },
                // Editing an existing entry in place captures all text input.
                SettingsField::DirList if model.ui.settings.editing.is_some() => match key.code {
                    KeyCode::Esc => Message::SettingsCancelEdit,
                    KeyCode::Enter => Message::SettingsCommitEdit,
                    KeyCode::Backspace => Message::SettingsEditBackspace,
                    KeyCode::Char(c) => Message::SettingsEditInput(c),
                    _ => Message::None,
                },
                SettingsField::DirList => match key.code {
                    KeyCode::Esc => Message::ToggleSettings,
                    KeyCode::Enter => Message::SettingsSave,
                    KeyCode::Up => Message::SettingsNavUp,
                    KeyCode::Down => Message::SettingsNavDown,
                    KeyCode::Char('e') => Message::SettingsStartEdit,
                    KeyCode::Char('d') | KeyCode::Delete => Message::SettingsRemoveDir,
                    KeyCode::Char('p') => Message::SettingsMakePrimary,
                    _ => Message::None,
                },
                SettingsField::UseCurrentDir => match key.code {
                    KeyCode::Esc => Message::ToggleSettings,
                    KeyCode::Enter => Message::SettingsSave,
                    KeyCode::Char(' ') => Message::SettingsToggleCwd,
                    KeyCode::Up => Message::SettingsNavUp,
                    KeyCode::Down => Message::SettingsNavDown,
                    _ => Message::None,
                },
            };
        }

        if in_logs {
            return match key.code {
                KeyCode::Up | KeyCode::Char('k') => Message::LogScrollUp,
                KeyCode::Down | KeyCode::Char('j') => Message::LogScrollDown,
                KeyCode::PageUp => Message::LogScrollUp,
                KeyCode::PageDown => Message::LogScrollDown,
                KeyCode::Home => Message::LogScrollTop,
                KeyCode::End => Message::LogScrollBottom,
                KeyCode::Char('l') => Message::ToggleLogs,
                KeyCode::Char('q') => Message::Quit,
                _ => Message::None,
            };
        }

        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => Message::Escape,

            (KeyCode::Tab, _) => Message::ToggleActivePanel,

            (KeyCode::Enter, _) => {
                if active_search_results {
                    Message::DownloadYoutube(model.ui.search_selected)
                } else if active_search_input {
                    if !model.search.query.is_empty() {
                        Message::DoYoutubeSearch(model.search.query.clone())
                    } else {
                        Message::Enter
                    }
                } else {
                    Message::Enter
                }
            }

            _ if active_search_input => match key.code {
                KeyCode::Backspace => Message::SearchBackspace,
                KeyCode::Char(c) => Message::SearchInput(c),
                _ => Message::None,
            },

            (KeyCode::Char('q'), _) => Message::Quit,

            (KeyCode::Char(' '), _) if active_playlist => Message::PlayPause,

            (KeyCode::Char('l'), _) => Message::ToggleLogs,
            (KeyCode::Char('c'), _) => Message::ToggleSettings,
            (KeyCode::Char('r'), _) if active_playlist => Message::CycleRepeat,
            (KeyCode::Char('s'), _) if active_playlist => Message::ToggleShuffle,
            (KeyCode::Char('d'), _) if has_ctrl && (active_playlist || active_search_results) => {
                Message::ScrollDownHalf
            }
            (KeyCode::Char('d'), _) if active_playlist => Message::RequestDeleteTrack,
            (KeyCode::Delete, _) if active_playlist => Message::RequestDeleteTrack,
            (KeyCode::Char('?'), _) if active_playlist => Message::ToggleHelp,
            (KeyCode::Char('y'), _) => Message::ToggleYoutube,

            (KeyCode::Right, _) if has_ctrl && active_playlist => Message::Next,
            (KeyCode::Right, _) if active_playlist => Message::SeekForward,
            (KeyCode::Left, _) if has_ctrl && active_playlist => Message::Prev,
            (KeyCode::Left, _) if active_playlist => Message::SeekBackward,
            (KeyCode::Up, _) if active_playlist || active_search_results => Message::ScrollUp,
            (KeyCode::Down, _) if active_playlist || active_search_results => Message::ScrollDown,
            (KeyCode::Char('k'), _) if active_playlist || active_search_results => Message::ScrollUp,
            (KeyCode::Char('j'), _) if active_playlist || active_search_results => {
                Message::ScrollDown
            }
            (KeyCode::PageUp, _) if active_playlist || active_search_results => {
                Message::ScrollUpHalf
            }
            (KeyCode::PageDown, _) if active_playlist || active_search_results => {
                Message::ScrollDownHalf
            }
            (KeyCode::Char('u'), _) if has_ctrl && (active_playlist || active_search_results) => {
                Message::ScrollUpHalf
            }
            (KeyCode::Home, _) if active_playlist || active_search_results => Message::ScrollTop,
            (KeyCode::End, _) if active_playlist || active_search_results => Message::ScrollBottom,
            (KeyCode::Char('g'), _) if active_playlist || active_search_results => Message::ScrollTop,
            (KeyCode::Char('G'), _) if active_playlist || active_search_results => {
                Message::ScrollBottom
            }

            (KeyCode::Char('+'), _) if active_playlist => Message::IncreaseVolume,
            (KeyCode::Char('-'), _) if active_playlist => Message::DecreaseVolume,

            _ => Message::None,
        }
    }

    fn render_frame(&mut self, f: &mut Frame, model: &Model) {
        let areas = calculate_layout(f.area(), model);

        self.render_header(f, areas.header, model);

        player::render_player(f, areas.now_playing, model);
        playlist::render_playlist(f, areas.playlist, model, &mut self.playlist_state);

        if model.ui.show_youtube {
            search::render_search_input(f, areas.search_input, model);
            search::render_search_results(
                f,
                areas.search_results,
                model,
                &mut self.search_results_state,
            );
        }

        if model.ui.show_help {
            help::render_help(f, areas.help);
        }

        if model.ui.show_logs {
            if let Some(logs_area) = areas.logs {
                logs::render_logs(f, logs_area, model, &mut self.logs_state);
            }
        }

        if model.ui.show_settings {
            settings::render_settings(f, f.area(), model);
        }

        if model.ui.confirm_delete.is_some() {
            popup::render_delete_confirm(f, f.area(), model);
        }
    }

    fn render_header(&self, f: &mut Frame, area: Rect, model: &Model) {
        let title = format!("[ TUSIC v{} ] ", env!("CARGO_PKG_VERSION"));

        let track_info = if let Some(idx) = model.current_index {
            let total = model.playlist.len();
            format!(" [{}/{}] ", idx + 1, total)
        } else {
            String::new()
        };

        let search_indicator = if model.search.is_loading {
            " [Searching...]"
        } else if model.search.is_downloading {
            " [Downloading...]"
        } else {
            ""
        };

        let active_panel_text = match model.ui.active_panel {
            ActivePanel::Playlist => "[Playlist]",
            ActivePanel::SearchInput => "[Search Input]",
            ActivePanel::SearchResults => "[Search Results]",
        };

        let header_text = Line::from(vec![
            Span::styled(title, Style::default().fg(Color::Cyan).bold()),
            Span::raw(track_info),
            Span::raw(active_panel_text),
            Span::styled(search_indicator, Style::default().fg(Color::Yellow)),
        ]);

        let p = Paragraph::new(header_text)
            .style(Style::default())
            .block(Block::default().borders(Borders::NONE));

        f.render_widget(p, area);
    }
}
