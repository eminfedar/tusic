use crate::model::{ActivePanel, Model};
use crate::youtube::YoutubeTrack;
use ratatui::style::Modifier;
use ratatui::symbols;
use ratatui::text::Span;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Scrollbar, ScrollbarOrientation},
    Frame,
};
use tui_widget_list::{ListBuilder, ListState, ListView};

#[derive(Default)]
pub struct SearchResultsState {
    pub list_state: ListState,
}

pub fn render_search_input(f: &mut Frame, area: Rect, model: &Model) {
    let is_active = matches!(model.ui.active_panel, ActivePanel::SearchInput);

    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" YouTube Search ")
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let query = &model.search.query;

    let input_text = if query.is_empty() {
        Span::from("Search YouTube...").style(Style::new().dim())
    } else {
        Span::from(query)
    };

    let blinking_caret =
        Span::raw(symbols::block::FULL).style(Style::new().add_modifier(Modifier::RAPID_BLINK));
    let lines = if is_active {
        Line::from(vec![input_text, blinking_caret])
    } else {
        Line::from(vec![input_text])
    };

    let p = ratatui::widgets::Paragraph::new(lines)
        .style(Style::default())
        .block(Block::default());

    f.render_widget(p, inner);
}

pub fn render_search_results(
    f: &mut Frame,
    area: Rect,
    model: &Model,
    state: &mut SearchResultsState,
) {
    let is_active = matches!(model.ui.active_panel, ActivePanel::SearchResults);
    let results = &model.search.results;

    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title: String = if model.search.is_downloading {
        " Downloading... ".to_string()
    } else if model.search.is_loading {
        " Searching YouTube... ".to_string()
    } else {
        format!(" YouTube Results [{}] ", results.len())
    };

    let block = Block::default()
        .title(title)
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(border_style);

    if model.search.is_loading && results.is_empty() {
        f.render_widget(&block, area);
        let inner = block.inner(area);
        let p = ratatui::widgets::Paragraph::new("Searching YouTube for songs...")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default());
        f.render_widget(p, inner);
        return;
    }

    if results.is_empty() {
        f.render_widget(&block, area);
        let inner = block.inner(area);
        let hint = "Type a song name and press Enter to search";
        let p = ratatui::widgets::Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default());
        f.render_widget(p, inner);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

    let list = ListView::new(
        ListBuilder::new(move |context| {
            let idx = context.index;
            let track: &YoutubeTrack = &results[idx];
            let is_selected = context.is_selected;

            let mut style = if is_selected {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default()
            };

            if !is_active {
                style = style.dim()
            }

            let prefix = if is_selected { "> " } else { "  " };

            let duration = format_duration(track.duration_ms);
            let name = if track.channel.is_empty() {
                track.title.clone()
            } else {
                format!("{} - {}", track.title, track.channel)
            };

            // Ellipsize text if larger than width
            let char_indices: Vec<usize> = name.char_indices().map(|(i, _c)| i).collect();
            let width_limit = (inner.width - 2) as usize - duration.len();

            let name = if char_indices.len() > width_limit {
                match char_indices.get(width_limit - 2) {
                    // 3 is the appended length of spaces: " " + ".."
                    Some(&end) => {
                        format!("{}..", &name[..end])
                    }
                    None => name,
                }
            } else {
                name
            };

            let line = Line::from(vec![
                ratatui::text::Span::raw(prefix),
                ratatui::text::Span::styled(name, style),
                ratatui::text::Span::raw(duration),
            ]);
            (line, 1)
        }),
        results.len(),
    )
    .block(Block::default())
    .scrollbar(scrollbar)
    .infinite_scrolling(true);

    state.list_state.select(Some(model.ui.search_selected));

    f.render_stateful_widget(list, inner, &mut state.list_state);
}

fn format_duration(ms: u64) -> String {
    let minutes = (ms / 1000) / 60;
    let seconds = (ms / 1000) % 60;

    format!(" ({minutes:02}:{seconds:02})")
}
