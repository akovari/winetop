use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{layout::Rect, Frame};

pub fn sparkline(history: &[f32], width: usize) -> String {
    const BARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if history.is_empty() || width == 0 {
        return String::new();
    }
    let slice = if history.len() > width {
        &history[history.len() - width..]
    } else {
        history
    };
    let max = slice.iter().cloned().fold(0.0_f32, f32::max).max(1.0);
    slice
        .iter()
        .map(|v| {
            let idx = ((*v / max) * (BARS.len() as f32 - 1.0)).round() as usize;
            BARS[idx.min(BARS.len() - 1)]
        })
        .collect()
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ratatui::layout::Constraint::Percentage(percent_y),
        ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    ratatui::layout::Layout::horizontal([
        ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ratatui::layout::Constraint::Percentage(percent_x),
        ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

pub fn draw_modal(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line>, style: Style) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style);
    let inner = centered_rect(70, 40, area);
    frame.render_widget(Clear, inner);
    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

pub fn key_hint(keys: &[(&str, &str)], style: Style) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, (k, desc)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", style));
        }
        spans.push(Span::styled(
            format!("{k} "),
            Style::default().fg(ratatui::style::Color::White),
        ));
        spans.push(Span::styled((*desc).to_string(), style));
    }
    Line::from(spans)
}
