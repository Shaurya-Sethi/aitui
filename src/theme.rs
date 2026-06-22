use ratatui::prelude::Stylize;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Block;

#[derive(Debug, Clone)]
pub struct Theme;

impl Theme {
    pub fn bg(&self) -> Color {
        Color::Rgb(30, 30, 46)
    }

    pub fn style_user(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(205, 214, 244))
            .bg(Color::Rgb(49, 50, 68))
    }

    pub fn style_assistant(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(205, 214, 244))
            .bg(Color::Rgb(30, 30, 46))
    }

    pub fn style_user_border(&self) -> Style {
        Style::default().fg(Color::Rgb(137, 180, 250))
    }

    pub fn style_assistant_border(&self) -> Style {
        Style::default().fg(Color::Rgb(69, 71, 90))
    }

    pub fn style_code(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(166, 227, 161))
            .bg(Color::Rgb(49, 50, 68))
    }

    pub fn style_code_border(&self) -> Style {
        Style::default().fg(Color::Rgb(88, 91, 112))
    }

    pub fn style_muted(&self) -> Style {
        Style::default().fg(Color::Rgb(108, 112, 134))
    }

    pub fn style_accent(&self) -> Style {
        Style::default().fg(Color::Rgb(137, 180, 250))
    }

    pub fn style_error(&self) -> Style {
        Style::default().fg(Color::Rgb(243, 139, 168))
    }

    pub fn style_streaming(&self) -> Style {
        Style::default().fg(Color::Rgb(166, 227, 161))
    }

    pub fn style_thinking(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(148, 156, 187))
            .bg(Color::Rgb(30, 30, 46))
            .add_modifier(Modifier::ITALIC)
    }

    pub fn style_inline_code(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(245, 194, 231))
            .bg(Color::Rgb(49, 50, 68))
    }

    pub fn block_header<'a>(&self, title: &'a str) -> Block<'a> {
        Block::bordered()
            .border_style(self.style_assistant_border())
            .title(title.bold())
            .style(Style::default().bg(self.bg()))
    }

    pub fn block_input<'a>(&self, title: &'a str) -> Block<'a> {
        Block::bordered()
            .border_style(self.style_accent())
            .title(title)
            .style(Style::default().bg(self.bg()))
    }

    pub fn block_overlay<'a>(&self, title: &'a str) -> Block<'a> {
        Block::bordered()
            .border_style(self.style_accent())
            .title(title.bold())
            .style(Style::default().bg(Color::Rgb(24, 24, 37)))
    }
}
