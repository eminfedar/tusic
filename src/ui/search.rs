use crate::model::{ActivePanel, Model};
use crate::youtube::YoutubeTrack;
use ratatui::style::Stylize;
use ratatui::symbols::{self, shade};
use ratatui::text::Span;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Clear, Scrollbar, ScrollbarOrientation},
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

    let blinking_caret = Span::raw(symbols::block::FULL).rapid_blink();
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

    // Both an in-progress YouTube *search* and a *download* show the same
    // animated loading placeholder in this panel, with any stale results
    // cleared away. When the operation finishes the results list returns.
    if model.search.is_loading || model.search.is_downloading {
        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(Clear, inner);
        render_loading_blocks(f, inner, model.ui.anim_tick);
        return;
    }

    if results.is_empty() {
        f.render_widget(&block, area);
        let inner = block.inner(area);
        // Show the last search error (e.g. no network) in red, otherwise the
        // default hint in gray.
        let (text, color) = match &model.search.error {
            Some(e) => (e.as_str(), Color::Red),
            None => (
                "Type a song name and press Enter to search",
                Color::DarkGray,
            ),
        };
        let p = ratatui::widgets::Paragraph::new(text)
            .style(Style::default().fg(color))
            .wrap(ratatui::widgets::Wrap { trim: true })
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
                ratatui::text::Span::styled(duration, style),
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

/// Loading placeholder shown while a YouTube search or download is in progress.
/// Draws `BLOCKS` full-width bars that "breathe" by cycling through Unicode
/// block-shade glyphs (` `, `░`, `▒`, `█`), each drawn in `Color::Gray`. Every
/// bar leads the one below it by two animation frames, producing a top-to-bottom
/// ripple. `tick` is the model's free-running animation counter
/// (`model.ui.anim_tick`), advanced ~25 times per second.
fn render_loading_blocks(f: &mut Frame, area: Rect, tick: u64) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    // One pulse of the "breathing" animation expressed as an ordered sequence
    // of shade glyphs: invisible for a couple of frames, then ramping up ░▒█ and
    // back down. A bar blinks by stepping through this list one frame at a time.
    const ANIMATION: [&str; 7] = [
        shade::EMPTY,
        shade::EMPTY,
        shade::LIGHT,
        shade::MEDIUM,
        shade::FULL,
        shade::MEDIUM,
        shade::LIGHT,
    ];
    // Ticks spent on each animation frame (~25 ticks/sec ⇒ ~0.6s per pulse).
    const STEP_TICKS: u64 = 2;
    const BLOCKS: u16 = 3;

    let frame = tick / STEP_TICKS;

    // Reuse a single buffer for all bars instead of allocating one per row every
    // frame. Shade glyphs are 3 bytes each.
    let mut bar = String::with_capacity(area.width as usize * 3);

    for row in 0..BLOCKS {
        if row >= area.height {
            break;
        }

        // `BLOCKS - row` is 3, 2, 1 — never underflows. A larger phase means the
        // bar is further ahead in the cycle, so the pulse ripples downward.
        let phase = (BLOCKS - row) as u64 * 2;
        let glyph = ANIMATION[((frame + phase) % ANIMATION.len() as u64) as usize];

        bar.clear();
        for _ in 0..area.width {
            bar.push_str(glyph);
        }

        let line = Line::from(Span::styled(bar.as_str(), Style::default().fg(Color::Gray)));
        let row_area = Rect::new(area.x, area.y + row, area.width, 1);
        f.render_widget(ratatui::widgets::Paragraph::new(line), row_area);
    }
}

fn format_duration(ms: u64) -> String {
    let minutes = (ms / 1000) / 60;
    let seconds = (ms / 1000) % 60;

    format!(" ({minutes:02}:{seconds:02})")
}
