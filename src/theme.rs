use ratatui::prelude::Stylize;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Block;

pub fn bg() -> Color {
    Color::Rgb(30, 30, 46)
}

pub fn style_user() -> Style {
    Style::default()
        .fg(Color::Rgb(205, 214, 244))
        .bg(Color::Rgb(49, 50, 68))
}

pub fn style_assistant() -> Style {
    Style::default()
        .fg(Color::Rgb(205, 214, 244))
        .bg(Color::Rgb(30, 30, 46))
}

pub fn style_user_border() -> Style {
    Style::default().fg(Color::Rgb(137, 180, 250))
}

pub fn style_assistant_border() -> Style {
    Style::default().fg(Color::Rgb(69, 71, 90))
}

pub fn style_code() -> Style {
    Style::default()
        .fg(Color::Rgb(166, 227, 161))
        .bg(Color::Rgb(49, 50, 68))
}

pub fn style_code_border() -> Style {
    Style::default().fg(Color::Rgb(88, 91, 112))
}

pub fn style_muted() -> Style {
    Style::default().fg(Color::Rgb(108, 112, 134))
}

pub fn style_accent() -> Style {
    Style::default().fg(Color::Rgb(137, 180, 250))
}

pub fn style_error() -> Style {
    Style::default().fg(Color::Rgb(243, 139, 168))
}

pub fn style_streaming() -> Style {
    Style::default().fg(Color::Rgb(166, 227, 161))
}

pub fn style_thinking() -> Style {
    Style::default()
        .fg(Color::Rgb(148, 156, 187))
        .bg(Color::Rgb(30, 30, 46))
        .add_modifier(Modifier::ITALIC)
}

pub fn style_inline_code() -> Style {
    Style::default()
        .fg(Color::Rgb(245, 194, 231))
        .bg(Color::Rgb(49, 50, 68))
}

pub fn block_header<'a>(title: &'a str) -> Block<'a> {
    Block::bordered()
        .border_style(style_assistant_border())
        .title(title.bold())
        .style(Style::default().bg(bg()))
}

pub fn block_input<'a>(title: &'a str) -> Block<'a> {
    Block::bordered()
        .border_style(style_accent())
        .title(title)
        .style(Style::default().bg(bg()))
}

pub fn block_overlay<'a>(title: &'a str) -> Block<'a> {
    Block::bordered()
        .border_style(style_accent())
        .title(title.bold())
        .style(Style::default().bg(Color::Rgb(24, 24, 37)))
}
