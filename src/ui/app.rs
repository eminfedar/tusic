use crate::audio::AudioBackend;
use crate::model::{ActivePanel, Model};
use crate::msg::Message;
use crate::ui::help::render_help;
use crate::ui::layout::calculate_layout;
use crate::ui::logs::render_logs;
use crate::ui::logs::LogsState;
use crate::ui::player::render_player;
use crate::ui::playlist::render_playlist;
use crate::ui::playlist::PlaylistState;
use crate::ui::search::SearchResultsState;
use crate::ui::search::{render_search_input, render_search_results};
use crate::update::{read_tracks, update};
use crate::watcher;
use crate::youtube::{get_download_dir, get_scan_paths, YoutubeService};
use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};

use std::sync::mpsc::{self, Receiver};

use crate::task::Task;

pub struct App<T: AudioBackend> {
    player: T,
    yt_service: YoutubeService,
    running: bool,
    playlist_state: PlaylistState,
    search_results_state: SearchResultsState,
    logs_state: LogsState,
    task: Task<Message>,
    task_rx: Receiver<Message>,
}

impl<T: AudioBackend> App<T> {
    pub fn new(backend: T) -> Result<Self> {
        // YouTube service
        let download_dir = get_download_dir();

        let yt_service = YoutubeService::new(download_dir.clone());
        let yt_service = smol::block_on(async_compat::Compat::new(yt_service))?;

        let (task_tx, task_rx) = mpsc::channel::<Message>();

        // File change watcher
        watcher::watch(get_scan_paths(), task_tx.clone())?;

        // Create
        Ok(Self {
            player: backend,
            yt_service,
            running: false,
            playlist_state: PlaylistState::default(),
            search_results_state: SearchResultsState::default(),
            logs_state: LogsState::default(),
            task: Task::new(task_tx),
            task_rx,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.running = true;

        let mut model = Model::default();

        let playlist = read_tracks();
        model.set_tracks(playlist);

        while self.running {
            // Draw
            terminal.draw(|f| self.render_frame(f, &model))?;

            // Input Events
            let msg = self.handle_events(&mut model)?;
            if matches!(msg, Message::Quit) {
                break;
            }

            // Update from Input Events
            update(
                &mut model,
                msg,
                &mut self.player,
                &self.yt_service,
                &self.task,
            )?;

            // Handle results from async Tasks
            while let Ok(m) = self.task_rx.try_recv() {
                update(
                    &mut model,
                    m,
                    &mut self.player,
                    &self.yt_service,
                    &self.task,
                )?;
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

        if in_logs {
            return match key.code {
                KeyCode::Up | KeyCode::Char('k') => Message::LogScrollUp,
                KeyCode::Down | KeyCode::Char('j') => Message::LogScrollDown,
                KeyCode::PageUp => Message::LogScrollUp,
                KeyCode::PageDown => Message::LogScrollDown,
                KeyCode::Home => Message::LogScrollTop,
                KeyCode::End => Message::LogScrollBottom,
                KeyCode::Char('l') => Message::ToggleLogs,
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

            (KeyCode::Char(' '), _) if active_playlist => {
                if self.player.is_playing() {
                    Message::Pause
                } else {
                    Message::Play
                }
            }

            (KeyCode::Char('l'), _) => Message::ToggleLogs,
            (KeyCode::Char('r'), _) if active_playlist => Message::CycleRepeat,
            (KeyCode::Char('s'), _) if active_playlist => Message::ToggleShuffle,
            (KeyCode::Char('?'), _) if active_playlist => Message::ToggleHelp,
            (KeyCode::Char('y'), _) => Message::ToggleYoutube,

            (KeyCode::Right, _) if has_ctrl && active_playlist => Message::Next,
            (KeyCode::Right, _) if active_playlist => Message::SeekForward,
            (KeyCode::Left, _) if has_ctrl && active_playlist => Message::Prev,
            (KeyCode::Left, _) if active_playlist => Message::SeekBackward,
            (KeyCode::Up, _) if active_playlist || active_search_results => Message::ScrollUp,
            (KeyCode::Down, _) if active_playlist || active_search_results => Message::ScrollDown,
            (KeyCode::PageUp, _) if active_playlist || active_search_results => {
                Message::ScrollUpHalf
            }
            (KeyCode::PageDown, _) if active_playlist || active_search_results => {
                Message::ScrollDownHalf
            }
            (KeyCode::Home, _) if active_playlist || active_search_results => Message::ScrollTop,
            (KeyCode::End, _) if active_playlist || active_search_results => Message::ScrollBottom,

            (KeyCode::Char('+'), _) if active_playlist => Message::IncreaseVolume,
            (KeyCode::Char('-'), _) if active_playlist => Message::DecreaseVolume,

            _ => Message::None,
        }
    }

    fn render_frame(&mut self, f: &mut Frame, model: &Model) {
        let areas = calculate_layout(f.area(), model);

        self.render_header(f, areas.header, model);

        render_player(f, areas.now_playing, model);
        render_playlist(f, areas.playlist, model, &mut self.playlist_state);

        if model.ui.show_youtube {
            render_search_input(f, areas.search_input, model);
            render_search_results(
                f,
                areas.search_results,
                model,
                &mut self.search_results_state,
            );
        }

        if model.ui.show_help {
            render_help(f, areas.help);
        }

        if model.ui.show_logs {
            if let Some(logs_area) = areas.logs {
                render_logs(f, logs_area, model, &mut self.logs_state);
            }
        }
    }

    fn render_header(&self, f: &mut Frame, area: Rect, model: &Model) {
        let title = " TUSIC2 ";

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
            ActivePanel::Playlist => " [Playlist]",
            ActivePanel::SearchInput => " [Search Input]",
            ActivePanel::SearchResults => " [Search Results]",
        };

        let header_text = Line::from(vec![
            Span::styled(title, Style::default().fg(Color::Cyan).bold()),
            Span::raw(track_info),
            Span::raw(active_panel_text),
            Span::styled(search_indicator, Style::default().fg(Color::Yellow)),
        ]);

        let p = Paragraph::new(header_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::NONE));

        f.render_widget(p, area);
    }
}
