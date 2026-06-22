use ratatui::prelude::Stylize;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Block;

pub const BG: Color = Color::Rgb(30, 30, 46);
const OVERLAY_BG: Color = Color::Rgb(24, 24, 37);
const TEXT: Color = Color::Rgb(205, 214, 244);
const USER_BG: Color = Color::Rgb(49, 50, 68);
const ACCENT: Color = Color::Rgb(137, 180, 250);
const ASSISTANT_BORDER: Color = Color::Rgb(69, 71, 90);
const CODE_FG: Color = Color::Rgb(166, 227, 161);
const CODE_BORDER: Color = Color::Rgb(88, 91, 112);
const MUTED: Color = Color::Rgb(108, 112, 134);
const ERROR: Color = Color::Rgb(243, 139, 168);
const THINKING: Color = Color::Rgb(148, 156, 187);
const INLINE_CODE: Color = Color::Rgb(245, 194, 231);

pub fn bg() -> Color {
    BG
}

pub fn style_user() -> Style {
    Style::default().fg(TEXT).bg(USER_BG)
}

pub fn style_assistant() -> Style {
    Style::default().fg(TEXT).bg(BG)
}

pub fn style_user_border() -> Style {
    Style::default().fg(ACCENT)
}

pub fn style_assistant_border() -> Style {
    Style::default().fg(ASSISTANT_BORDER)
}

pub fn style_code() -> Style {
    Style::default().fg(CODE_FG).bg(USER_BG)
}

pub fn style_code_border() -> Style {
    Style::default().fg(CODE_BORDER)
}

pub fn style_muted() -> Style {
    Style::default().fg(MUTED)
}

pub fn style_accent() -> Style {
    Style::default().fg(ACCENT)
}

pub fn style_error() -> Style {
    Style::default().fg(ERROR)
}

pub fn style_streaming() -> Style {
    Style::default().fg(CODE_FG)
}

pub fn style_thinking() -> Style {
    Style::default()
        .fg(THINKING)
        .bg(BG)
        .add_modifier(Modifier::ITALIC)
}

pub fn style_inline_code() -> Style {
    Style::default().fg(INLINE_CODE).bg(USER_BG)
}

pub fn block_header<'a>(title: &'a str) -> Block<'a> {
    Block::bordered()
        .border_style(style_assistant_border())
        .title(title.bold())
        .style(Style::default().bg(BG))
}

pub fn block_input<'a>(title: &'a str) -> Block<'a> {
    Block::bordered()
        .border_style(style_accent())
        .title(title)
        .style(Style::default().bg(BG))
}

pub fn block_overlay<'a>(title: &'a str) -> Block<'a> {
    Block::bordered()
        .border_style(style_accent())
        .title(title.bold())
        .style(Style::default().bg(OVERLAY_BG))
}
