use crate::app::App;
use crate::store;
use crate::theme;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::prelude::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(ratatui::style::Style::default().bg(theme::bg())),
        area,
    );

    let [header, chat, input, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(5),
        Constraint::Length(1),
    ])
    .areas(area);

    draw_header(frame, app, header);
    draw_chat(frame, app, chat);
    draw_input(frame, app, input);
    draw_footer(frame, app, footer);

    if app.resume_overlay.is_some() {
        draw_resume_overlay(frame, app);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let host = app.config.endpoint_host();
    let model = &app.config.model;

    let stream_indicator = if app.streaming {
        Line::from(vec![
            Span::styled(" ● ", theme::style_streaming()),
            Span::styled("streaming", theme::style_streaming().italic()),
        ])
    } else {
        Line::from(Span::styled(" ready ", theme::style_muted()))
    };

    let title = format!(" {model} · {host} ");
    let block = theme::block_header(&title)
        .title_alignment(ratatui::layout::Alignment::Left);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = Layout::horizontal([Constraint::Fill(1), Constraint::Length(14)]).split(inner);
    frame.render_widget(Paragraph::new(""), cols[0]);
    frame.render_widget(Paragraph::new(stream_indicator), cols[1]);
}

fn draw_chat(frame: &mut Frame, app: &mut App, area: Rect) {
    let width = area.width;
    app.rebuild_chat_lines(width);

    let block = Block::bordered()
        .border_style(theme::style_assistant_border())
        .style(ratatui::style::Style::default().bg(theme::bg()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let viewport = inner.height as usize;
    app.viewport_height = viewport.max(1);
    app.sync_scroll(viewport);

    let scroll = app.scroll.min(app.max_scroll(viewport));
    let paragraph = Paragraph::new(app.chat_lines.clone())
        .style(ratatui::style::Style::default().bg(theme::bg()))
        .scroll((scroll as u16, 0));

    frame.render_widget(paragraph, inner);

    if app.chat_lines.len() > viewport {
        let content_len = app.chat_lines.len();
        let mut state = ScrollbarState::new(content_len)
            .position(scrollbar_position(scroll, content_len, viewport))
            .viewport_content_length(viewport);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_symbol("█")
            .track_symbol(Some("│"));
        frame.render_stateful_widget(
            scrollbar,
            inner.inner(Margin {
                vertical: 0,
                horizontal: 0,
            }),
            &mut state,
        );
    }
}

/// Ratatui scrollbar position tops out at `content_len - 1`, but paragraph scroll
/// tops out at `content_len - viewport`. Scale so the thumb reaches the track end.
fn scrollbar_position(scroll: usize, content_len: usize, viewport: usize) -> usize {
    let max_scroll = content_len.saturating_sub(viewport.max(1));
    if max_scroll == 0 {
        return 0;
    }
    scroll.saturating_mul(content_len.saturating_sub(1)) / max_scroll
}

#[cfg(test)]
mod tests {
    use super::scrollbar_position;
    use crate::app::App;
    use crate::config::Config;
    use crate::store::Message;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn chat_text(backend: &TestBackend) -> String {
        let buf = backend.buffer();
        let mut out = String::new();
        for y in 3..18 {
            for x in 0..buf.area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    out.push_str(cell.symbol());
                }
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn scrollbar_position_reaches_end_at_bottom() {
        let content_len = 100;
        let viewport = 20;
        let max_scroll = content_len - viewport;
        assert_eq!(
            scrollbar_position(max_scroll, content_len, viewport),
            content_len - 1
        );
    }

    #[test]
    fn scrollbar_position_starts_at_top() {
        assert_eq!(scrollbar_position(0, 100, 20), 0);
    }

    #[test]
    fn draw_honors_manual_scroll_offset() {
        let mut app = App::new(Config::for_test());
        let mut content = String::new();
        for i in 0..40 {
            content.push_str(&format!("marker-{i:02}\n"));
        }
        app.messages.push(Message {
            role: "assistant".into(),
            content,
            thinking: String::new(),
        });
        app.auto_scroll = false;
        app.scroll = 0;

        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| super::draw(frame, &mut app)).unwrap();
        let top = chat_text(terminal.backend());
        assert!(top.contains("marker-00"));

        app.scroll = app.max_scroll(app.viewport_height);
        terminal.draw(|frame| super::draw(frame, &mut app)).unwrap();
        let bottom = chat_text(terminal.backend());
        assert!(bottom.contains("marker-39"));
        assert_ne!(top, bottom);
    }
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(&app.input, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let hints = " ↑↓ scroll · Enter send · Shift+Enter/⌥Enter newline · /new · /resume · /quit ";
    let mut spans = vec![Span::styled(hints, theme::style_muted())];

    if let Some(status) = &app.status {
        spans.push(Span::styled(format!(" │ {status}"), theme::style_error()));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_resume_overlay(frame: &mut Frame, app: &App) {
    let overlay = app.resume_overlay.as_ref().unwrap();
    let area = centered_rect(70, 70, frame.area());
    frame.render_widget(Clear, area);

    let block = theme::block_overlay(" Resume session ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = inner.height as usize;
    let start = overlay
        .selected
        .saturating_sub(visible.saturating_sub(1));

    let lines: Vec<Line> = overlay
        .sessions
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(i, s)| {
            let idx = start + i;
            let prefix = if idx == overlay.selected {
                "▸ "
            } else {
                "  "
            };
            let time = store::relative_time(s.updated_at);
            Line::from(vec![
                Span::styled(prefix, theme::style_accent()),
                Span::styled(s.title.clone(), theme::style_assistant()),
                Span::styled(format!("  {time}"), theme::style_muted()),
            ])
        })
        .collect();

    let help = Line::from(Span::styled(
        " ↑↓ select · Enter resume · Delete remove · Esc cancel ",
        theme::style_muted(),
    ));

    let [list_area, help_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }),
        list_area,
    );
    frame.render_widget(Paragraph::new(help), help_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
