use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::model::Model;

#[derive(Clone)]
pub struct LayoutRegions {
    pub now_playing: Rect,
    pub playlist: Rect,
    pub search_input: Rect,
    pub search_results: Rect,
    pub help: Rect,
    pub header: Rect,
    pub logs: Option<Rect>,
}

pub fn calculate_layout(area: Rect, model: &Model) -> LayoutRegions {
    let help_height = if model.ui.show_help { 1 } else { 0 };
    let header_height = 1;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(3),
            Constraint::Length(help_height),
        ])
        .split(area);

    let main_area = chunks[1];

    let logs_area = if model.ui.show_logs {
        let logs_height = (area.height as usize / 2).max(10) as u16;
        let logs_width = (area.width as usize - 4).max(40) as u16;
        let logs_x = (area.width - logs_width) / 2;
        let logs_y = (area.height - logs_height) / 2;
        Some(Rect::new(logs_x, logs_y, logs_width, logs_height))
    } else {
        None
    };

    let left_right = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_area);

    let left_column = if model.ui.show_youtube {
        left_right[0]
    } else {
        chunks[1]
    };
    let right_column = left_right[1];

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(3)])
        .split(left_column);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(right_column);

    LayoutRegions {
        header: chunks[0],
        now_playing: left_chunks[0],
        playlist: left_chunks[1],
        search_input: right_chunks[0],
        search_results: right_chunks[1],
        help: chunks[2],
        logs: logs_area,
    }
}
