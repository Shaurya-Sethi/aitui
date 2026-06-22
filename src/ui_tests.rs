#[cfg(test)]
mod ui_scroll_tests {
    use crate::app::App;
    use crate::config::Config;
    use crate::store::Message;
    use crate::ui;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn test_config() -> Config {
        Config {
            base_url: "http://localhost:11434/v1".into(),
            model: "test".into(),
            api_key: None,
        }
    }

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
    fn draw_honors_manual_scroll_offset() {
        let mut app = App::new(test_config());
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
        terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
        let top = chat_text(terminal.backend());
        assert!(top.contains("marker-00"));

        app.scroll = app.max_scroll(app.viewport_height);
        terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
        let bottom = chat_text(terminal.backend());
        assert!(bottom.contains("marker-39"));
        assert_ne!(top, bottom);
    }
}
