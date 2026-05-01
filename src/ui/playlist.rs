use crate::model::{ActivePanel, Model};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Scrollbar, ScrollbarOrientation},
    Frame,
};
use tui_widget_list::{ListBuilder, ListState, ListView};

#[derive(Default)]
pub struct PlaylistState {
    pub list_state: ListState,
}

pub fn render_playlist(f: &mut Frame, area: Rect, model: &Model, state: &mut PlaylistState) {
    let tracks = model.playlist.tracks();
    let current_index = model.current_index;

    let is_active = matches!(model.ui.active_panel, ActivePanel::Playlist);
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" Playlist [{} tracks] ", tracks.len());

    let block = Block::default()
        .title(title)
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(border_style);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

    let list = ListView::new(
        ListBuilder::new(move |context| {
            let idx = context.index;
            let track = &tracks[idx];
            let is_current = current_index == Some(idx);
            let has_error = is_current && model.ui.playback_error.is_some();

            let (prefix, mut style) = if has_error {
                ("⚠️".to_string(), Style::default().fg(Color::Red))
            } else if is_current {
                ("▸ ".to_string(), Style::default().fg(Color::Green).bold())
            } else if context.is_selected {
                ("  ".to_string(), Style::default().fg(Color::Yellow).bold())
            } else {
                ("  ".to_string(), Style::default())
            };

            if !is_active {
                style = style.dim()
            }

            let name = track.display_name();

            let line = Line::from(vec![
                ratatui::text::Span::raw(prefix),
                ratatui::text::Span::styled(name, style),
            ]);
            (line, 1)
        }),
        tracks.len(),
    )
    .block(block)
    .scrollbar(scrollbar)
    .infinite_scrolling(true);

    state.list_state.select(Some(model.ui.selected));

    f.render_stateful_widget(list, area, &mut state.list_state);
}
