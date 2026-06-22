use crate::theme;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::prelude::Stylize;
use ratatui::text::{Line, Span};

pub fn wrap_line(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let cur_len = current.chars().count();
        if current.is_empty() {
            if word_len > width {
                push_long_word(&mut lines, word, width);
            } else {
                current = word.to_string();
            }
        } else if cur_len + 1 + word_len <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            if word_len > width {
                push_long_word(&mut lines, word, width);
            } else {
                current = word.to_string();
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn push_long_word(lines: &mut Vec<String>, word: &str, width: usize) {
    let mut chunk = String::new();
    for ch in word.chars() {
        if chunk.chars().count() >= width {
            lines.push(chunk);
            chunk = String::new();
        }
        chunk.push(ch);
    }
    if !chunk.is_empty() {
        lines.push(chunk);
    }
}

pub fn render(md: &str, width: u16) -> Vec<Line<'static>> {
    let wrap_width = width.saturating_sub(4).max(10) as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut strikethrough = false;
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_lines: Vec<String> = Vec::new();
    let mut list_index = 0u64;

    let options = Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(md, options);

    let flush_inline = |spans: &mut Vec<Span<'static>>, out: &mut Vec<Line<'static>>| {
        if spans.is_empty() {
            return;
        }
        for chunk in wrap_spans(spans, wrap_width) {
            out.push(Line::from(chunk));
        }
        spans.clear();
    };

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                flush_inline(&mut current_spans, &mut lines);
                bold = true;
            }
            Event::End(TagEnd::Heading(..)) => {
                flush_inline(&mut current_spans, &mut lines);
                bold = false;
                lines.push(Line::from(""));
            }
            Event::Start(Tag::Paragraph) => {
                flush_inline(&mut current_spans, &mut lines);
            }
            Event::End(TagEnd::Paragraph) => {
                flush_inline(&mut current_spans, &mut lines);
                lines.push(Line::from(""));
            }
            Event::Start(Tag::Strong) => bold = true,
            Event::End(TagEnd::Strong) => bold = false,
            Event::Start(Tag::Emphasis) => italic = true,
            Event::End(TagEnd::Emphasis) => italic = false,
            Event::Start(Tag::Strikethrough) => strikethrough = true,
            Event::End(TagEnd::Strikethrough) => strikethrough = false,
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_inline(&mut current_spans, &mut lines);
                in_code_block = true;
                code_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    _ => String::new(),
                };
                code_lines.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if !code_lang.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("┌ ", theme::style_code_border()),
                        Span::styled(code_lang.clone(), theme::style_muted().italic()),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled("┌", theme::style_code_border())));
                }
                for cl in &code_lines {
                    let wrapped = wrap_line(cl, wrap_width.saturating_sub(4));
                    if wrapped.is_empty() {
                        lines.push(Line::from(vec![
                            Span::styled("│ ", theme::style_code_border()),
                            Span::styled(" ", theme::style_code()),
                        ]));
                    } else {
                        for part in wrapped {
                            lines.push(Line::from(vec![
                                Span::styled("│ ", theme::style_code_border()),
                                Span::styled(part, theme::style_code()),
                            ]));
                        }
                    }
                }
                lines.push(Line::from(Span::styled("└", theme::style_code_border())));
                lines.push(Line::from(""));
                code_lang.clear();
                code_lines.clear();
            }
            Event::Start(Tag::List(start)) => {
                flush_inline(&mut current_spans, &mut lines);
                list_index = start.unwrap_or(1);
            }
            Event::End(TagEnd::List(_)) => {
                lines.push(Line::from(""));
            }
            Event::Start(Tag::Item) => {
                flush_inline(&mut current_spans, &mut lines);
                let bullet = if list_index > 0 {
                    let b = format!("{list_index}. ");
                    list_index += 1;
                    b
                } else {
                    "• ".to_string()
                };
                current_spans.push(Span::styled(bullet, theme::style_muted()));
            }
            Event::End(TagEnd::Item) => {
                flush_inline(&mut current_spans, &mut lines);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                current_spans.push(Span::styled(
                    format!(" ({dest_url})"),
                    theme::style_muted(),
                ));
            }
            Event::End(TagEnd::Link) => {}
            Event::Rule => {
                flush_inline(&mut current_spans, &mut lines);
                let rule = "─".repeat(wrap_width.min(40));
                lines.push(Line::from(Span::styled(rule, theme::style_muted())));
                lines.push(Line::from(""));
            }
            Event::Code(text) => {
                if in_code_block {
                    for part in text.split('\n') {
                        code_lines.push(part.to_string());
                    }
                } else {
                    current_spans.push(Span::styled(
                        text.to_string(),
                        theme::style_inline_code(),
                    ));
                }
            }
            Event::Text(text) => {
                if in_code_block {
                    code_lines.push(text.to_string());
                } else {
                    let style = base_style(bold, italic, strikethrough);
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_inline(&mut current_spans, &mut lines);
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                current_spans.push(Span::styled(
                    format!("[image: {dest_url}]"),
                    theme::style_muted(),
                ));
            }
            _ => {}
        }
    }

    flush_inline(&mut current_spans, &mut lines);

    while lines.last().map(|l| l.spans.is_empty()) == Some(true) {
        lines.pop();
    }

    lines
}

fn base_style(bold: bool, italic: bool, strikethrough: bool) -> ratatui::style::Style {
    let mut style = theme::style_assistant();
    if bold {
        style = style.bold();
    }
    if italic {
        style = style.italic();
    }
    if strikethrough {
        style = style.crossed_out();
    }
    style
}

fn wrap_spans(spans: &[Span<'static>], width: usize) -> Vec<Vec<Span<'static>>> {
    let plain: String = spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect::<Vec<_>>()
        .join("");

    if plain.is_empty() {
        return vec![];
    }

    if plain.len() <= width {
        return vec![spans.to_vec()];
    }

    wrap_line(&plain, width)
        .into_iter()
        .map(|w| {
            vec![Span::styled(
                w,
                spans.first().map(|s| s.style).unwrap_or_default(),
            )]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    #[test]
    fn wrap_line_breaks_at_word_boundary() {
        let lines = wrap_line("hello world foo bar", 10);
        assert_eq!(lines, vec!["hello", "world foo", "bar"]);
    }

    #[test]
    fn fenced_code_block_renders_with_gutter() {
        let md = "```rust\nfn main() {}\n```";
        let lines = render(md, 60);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text.contains("rust"));
        assert!(text.contains("fn main()"));
        assert!(text.contains("│"));
    }

    #[test]
    fn strikethrough_renders_crossed_out() {
        let lines = render("~~gone~~", 40);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text.contains("gone"));
        assert!(lines[0].spans[0].style.add_modifier.contains(Modifier::CROSSED_OUT));
    }
}
