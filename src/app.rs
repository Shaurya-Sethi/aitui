use crate::chat::{stream_completion, ChatEvent};
use crate::config::Config;
use crate::store::{
    self, Message, Session, SessionMeta, new_session_id, now_secs, title_from_messages,
};
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::prelude::Stylize;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::collections::HashMap;
use tui_textarea::{Input, TextArea};
use tokio::sync::mpsc;


pub struct ResumeOverlay {
    pub sessions: Vec<SessionMeta>,
    pub selected: usize,
}

pub struct App {
    pub config: Config,
    pub messages: Vec<Message>,
    pub session_id: String,
    session_created: Option<u64>,
    pub input: TextArea<'static>,
    pub scroll: usize,
    pub auto_scroll: bool,
    pub streaming: bool,
    pub stream_rx: Option<mpsc::Receiver<ChatEvent>>,
    pub status: Option<String>,
    pub resume_overlay: Option<ResumeOverlay>,
    pub chat_width: u16,
    pub md_cache: HashMap<usize, ((String, u16), Vec<Line<'static>>)>,
    pub chat_lines: Vec<Line<'static>>,
    pub viewport_height: usize,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut app = Self {
            config,
            messages: Vec::new(),
            session_id: String::new(),
            session_created: None,
            input: new_input(),
            scroll: 0,
            auto_scroll: true,
            streaming: false,
            stream_rx: None,
            status: None,
            resume_overlay: None,
            chat_width: 80,
            md_cache: HashMap::new(),
            chat_lines: Vec::new(),
            viewport_height: 1,
        };
        app.start_new_session();
        app
    }

    fn start_new_session(&mut self) {
        self.session_id = new_session_id();
        self.session_created = None;
        self.messages.clear();
        self.md_cache.clear();
        self.chat_lines.clear();
        self.scroll = 0;
        self.auto_scroll = true;
        self.streaming = false;
        self.stream_rx = None;
        self.status = None;
        self.input = new_input();
    }

    /// Returns `true` when the app should quit.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.resume_overlay.is_some() {
            return self.handle_resume_key(key);
        }

        if is_newline_key(&key) {
            self.input.insert_newline();
            return false;
        }

        match key.code {
            KeyCode::Enter => {
                if self.send_input() {
                    return true;
                }
            }
            KeyCode::Up if input_is_empty(&self.input) => self.scroll_by(-1),
            KeyCode::Down if input_is_empty(&self.input) => self.scroll_by(1),
            KeyCode::PageUp if input_is_empty(&self.input) => self.scroll_by(-5),
            KeyCode::PageDown if input_is_empty(&self.input) => self.scroll_by(5),
            _ => {
                self.input.input_without_shortcuts(Input::from(key));
            }
        }
        false
    }

    fn handle_resume_key(&mut self, key: KeyEvent) -> bool {
        if matches!(key.code, KeyCode::Delete | KeyCode::Backspace)
            && key.modifiers.is_empty()
        {
            return self.delete_selected_session();
        }

        let Some(overlay) = self.resume_overlay.as_mut() else {
            return false;
        };
        match key.code {
            KeyCode::Esc => {
                self.resume_overlay = None;
            }
            KeyCode::Up => {
                overlay.selected = overlay.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                if overlay.selected + 1 < overlay.sessions.len() {
                    overlay.selected += 1;
                }
            }
            KeyCode::Enter => {
                let id = overlay.sessions.get(overlay.selected).map(|m| m.id.clone());
                self.resume_overlay = None;
                if let Some(id) = id {
                    self.load_session(&id);
                }
            }
            _ => {}
        }
        false
    }

    fn delete_selected_session(&mut self) -> bool {
        let Some(overlay) = self.resume_overlay.as_mut() else {
            return false;
        };
        let Some(session) = overlay.sessions.get(overlay.selected) else {
            return false;
        };
        let id = session.id.clone();
        let title = session.title.clone();

        match store::delete(&id) {
            Ok(()) => {
                overlay.sessions.retain(|s| s.id != id);
                let deleted_active = id == self.session_id;
                if overlay.sessions.is_empty() {
                    self.resume_overlay = None;
                    self.status = Some("No saved sessions".into());
                } else {
                    overlay.selected = overlay.selected.min(overlay.sessions.len() - 1);
                    self.status = Some(format!("Deleted: {title}"));
                }
                if deleted_active {
                    let status = self.status.clone();
                    self.start_new_session();
                    self.status = status;
                }
            }
            Err(e) => self.status = Some(e),
        }
        false
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.resume_overlay.is_some() {
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_by(-3),
            MouseEventKind::ScrollDown => self.scroll_by(3),
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        let mut done = false;
        let mut error: Option<String> = None;

        let mut tokens = Vec::new();
        let mut thinking_tokens = Vec::new();
        if let Some(rx) = &mut self.stream_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    ChatEvent::Token(t) => tokens.push(t),
                    ChatEvent::ThinkingToken(t) => thinking_tokens.push(t),
                    ChatEvent::Done => done = true,
                    ChatEvent::Error(e) => {
                        error = Some(e);
                        done = true;
                    }
                }
            }
        }
        if !tokens.is_empty() || !thinking_tokens.is_empty() {
            if let Some(last) = self.messages.last_mut() {
                if last.role == "assistant" {
                    for t in thinking_tokens {
                        last.thinking.push_str(&t);
                    }
                    for t in tokens {
                        last.content.push_str(&t);
                    }
                    let idx = self.messages.len() - 1;
                    self.invalidate_cache(idx);
                }
            }
        }

        if done {
            self.streaming = false;
            self.stream_rx = None;
            if let Some(e) = error {
                self.status = Some(e);
            } else {
                self.status = None;
                let _ = self.save_session();
            }
        }
    }

    fn scroll_by(&mut self, delta: i32) {
        let max = self.max_scroll(self.viewport_height);
        if delta < 0 {
            self.auto_scroll = false;
            self.scroll = self.scroll.saturating_sub((-delta) as usize);
        } else {
            self.scroll = (self.scroll + delta as usize).min(max);
            if self.scroll >= max {
                self.auto_scroll = true;
            }
        }
    }

    pub fn sync_scroll(&mut self, viewport_height: usize) {
        if self.auto_scroll {
            self.scroll = self.max_scroll(viewport_height);
        }
    }

    pub fn rebuild_chat_lines(&mut self, width: u16) {
        if width != self.chat_width {
            self.chat_width = width;
            self.md_cache.clear();
        }

        let inner = width.saturating_sub(6);
        let mut lines: Vec<Line<'static>> = Vec::new();

        for idx in 0..self.messages.len() {
            if idx > 0 {
                lines.push(Line::from(""));
            }

            if self.messages[idx].role == "user" {
                lines.push(Line::from(vec![
                    Span::styled("╭─ ", theme::style_user_border()),
                    Span::styled("You ", theme::style_user_border().bold()),
                ]));
                for wrapped in wrap_text(&self.messages[idx].content, inner) {
                    lines.push(Line::from(vec![
                        Span::styled("│ ", theme::style_user_border()),
                        Span::styled(wrapped.clone(), theme::style_user()),
                    ]));
                }
                lines.push(Line::from(Span::styled(
                    "╰",
                    theme::style_user_border(),
                )));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("╭─ ", theme::style_assistant_border()),
                    Span::styled(
                        "Assistant ",
                        theme::style_assistant_border().bold(),
                    ),
                ]));
                if !self.messages[idx].thinking.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("│ ", theme::style_assistant_border()),
                        Span::styled("thinking ", theme::style_thinking().bold()),
                    ]));
                    for wrapped in wrap_text(&self.messages[idx].thinking, inner) {
                        lines.push(Line::from(vec![
                            Span::styled("│ ", theme::style_assistant_border()),
                            Span::styled(wrapped.clone(), theme::style_thinking()),
                        ]));
                    }
                }
                for rendered in self.cached_render(idx, width) {
                    let mut spans = vec![Span::styled("│ ", theme::style_assistant_border())];
                    spans.extend(rendered.spans);
                    lines.push(Line::from(spans));
                }
                lines.push(Line::from(Span::styled(
                    "╰",
                    theme::style_assistant_border(),
                )));
            }
        }

        self.chat_lines = lines;
    }

    fn cached_render(&mut self, idx: usize, width: u16) -> Vec<Line<'static>> {
        let msg = &self.messages[idx];
        let key = (format!("{}\0{}", msg.thinking, msg.content), width);
        if let Some((cached_key, cached)) = self.md_cache.get(&idx) {
            if cached_key == &key {
                return cached.clone();
            }
        }
        let rendered = crate::markdown::render(&msg.content, width.saturating_sub(6));
        self.md_cache.insert(idx, (key, rendered.clone()));
        rendered
    }

    fn invalidate_cache(&mut self, idx: usize) {
        self.md_cache.remove(&idx);
    }

    /// Returns `true` when the app should quit.
    fn send_input(&mut self) -> bool {
        let text = self.input.lines().join("\n").trim().to_string();
        if text.is_empty() {
            return false;
        }

        if text == "/quit" {
            self.input = new_input();
            return true;
        }

        if self.streaming {
            return false;
        }

        self.input = new_input();

        if text.trim() == "/resume" {
            self.open_resume();
            return false;
        }

        if text.trim() == "/new" {
            let _ = self.save_session();
            self.start_new_session();
            self.status = Some("New session".into());
            return false;
        }

        self.messages.push(Message {
            role: "user".into(),
            content: text,
            thinking: String::new(),
        });
        self.messages.push(Message {
            role: "assistant".into(),
            content: String::new(),
            thinking: String::new(),
        });

        self.streaming = true;
        self.auto_scroll = true;
        self.status = None;
        self.stream_rx = Some(stream_completion(&self.config, &self.messages));
        false
    }

    fn open_resume(&mut self) {
        match store::list() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    self.status = Some("No saved sessions".into());
                } else {
                    self.resume_overlay = Some(ResumeOverlay {
                        sessions,
                        selected: 0,
                    });
                }
            }
            Err(e) => self.status = Some(e),
        }
    }

    fn load_session(&mut self, id: &str) {
        match store::load(id) {
            Ok(session) => {
                self.session_id = session.id;
                self.session_created = Some(session.created_at);
                self.messages = session.messages;
                self.md_cache.clear();
                self.auto_scroll = false;
                self.scroll = 0;
                self.status = Some(format!("Resumed: {}", session.title));
            }
            Err(e) => self.status = Some(e),
        }
    }

    pub fn save_session(&mut self) -> Result<(), String> {
        if self.messages.is_empty() {
            return Ok(());
        }
        let now = now_secs();
        let created_at = self.session_created.unwrap_or(now);
        if self.session_created.is_none() {
            self.session_created = Some(created_at);
        }
        let session = Session {
            id: self.session_id.clone(),
            title: title_from_messages(&self.messages),
            created_at,
            updated_at: now,
            messages: self.messages.clone(),
        };
        store::save(&session)?;
        Ok(())
    }

    pub fn max_scroll(&self, viewport_height: usize) -> usize {
        self.chat_lines
            .len()
            .saturating_sub(viewport_height.max(1))
    }
}

fn input_is_empty(input: &TextArea<'static>) -> bool {
    input.lines().join("\n").trim().is_empty()
}

fn is_newline_key(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => key
            .modifiers
            .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT),
        KeyCode::Char('j' | 'J' | '\n' | '\r') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

fn new_input() -> TextArea<'static> {
    let mut input = TextArea::default();
    input.set_placeholder_text("Type a message… (/new · /resume · /quit)");
    input.set_cursor_line_style(Style::default());
    input.set_block(
        theme::block_input(" Message ")
            .style(Style::default().bg(theme::bg())),
    );
    input
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

    fn app_with_long_chat() -> App {
        let mut app = App::new(Config::for_test());
        let mut content = String::new();
        for i in 0..40 {
            content.push_str(&format!("chat-line-{i:02}\n"));
        }
        app.messages.push(Message {
            role: "user".into(),
            content: "hello".into(),
            thinking: String::new(),
        });
        app.messages.push(Message {
            role: "assistant".into(),
            content,
            thinking: String::new(),
        });
        app.rebuild_chat_lines(80);
        app.viewport_height = 10;
        app
    }

    #[test]
    fn page_up_disables_auto_scroll_and_moves_up() {
        let mut app = app_with_long_chat();
        app.auto_scroll = true;
        app.sync_scroll(app.viewport_height);
        let at_bottom = app.scroll;
        assert!(at_bottom > 0);

        let key = KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty());
        app.handle_key(key);

        assert!(!app.auto_scroll);
        assert!(app.scroll < at_bottom);
        app.sync_scroll(app.viewport_height);
        assert!(app.scroll < at_bottom);
    }

    #[test]
    fn mouse_scroll_up_disables_auto_scroll() {
        let mut app = app_with_long_chat();
        app.auto_scroll = true;
        app.sync_scroll(app.viewport_height);
        let at_bottom = app.scroll;

        app.handle_mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        });

        assert!(!app.auto_scroll);
        assert!(app.scroll < at_bottom);
    }

    #[test]
    fn load_session_starts_at_top_without_auto_pin() {
        let _guard = crate::store::TEST_DIR_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("aitui-test-{}", new_session_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("AITUI_TEST_SESSIONS_DIR", &tmp);

        let mut content = String::new();
        for i in 0..40 {
            content.push_str(&format!("saved-line-{i:02}\n"));
        }
        store::save(&Session {
            id: "saved-long".into(),
            title: "saved".into(),
            created_at: 1,
            updated_at: 1,
            messages: vec![
                Message {
                    role: "user".into(),
                    content: "hello".into(),
                    thinking: String::new(),
                },
                Message {
                    role: "assistant".into(),
                    content,
                    thinking: String::new(),
                },
            ],
        })
        .unwrap();

        let mut app = app_with_long_chat();
        app.auto_scroll = true;
        app.sync_scroll(app.viewport_height);
        assert!(app.scroll > 0);

        app.load_session("saved-long");

        assert!(!app.auto_scroll);
        assert_eq!(app.scroll, 0);
        assert_eq!(app.session_id, "saved-long");
        assert_eq!(app.messages.len(), 2);
        assert!(app.messages[1].content.contains("saved-line-39"));

        std::env::remove_var("AITUI_TEST_SESSIONS_DIR");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn type_chars(app: &mut App, chars: &[char]) {
        for c in chars {
            app.handle_key(KeyEvent::new(KeyCode::Char(*c), KeyModifiers::empty()));
        }
    }

    #[test]
    fn modified_enter_inserts_newline_without_sending() {
        struct Case {
            before: &'static [char],
            newline: KeyEvent,
            after: &'static [char],
            expected: &'static [&'static str],
        }

        let cases = [
            Case {
                before: &['x'],
                newline: KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT),
                after: &[],
                expected: &["x", ""],
            },
            Case {
                before: &['a', 'b'],
                newline: KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
                after: &[],
                expected: &["ab", ""],
            },
            Case {
                before: &['f', 'o', 'o'],
                newline: KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT),
                after: &['b', 'a', 'r'],
                expected: &["foo", "bar"],
            },
        ];

        for case in cases {
            let mut app = App::new(Config::for_test());
            type_chars(&mut app, case.before);
            app.handle_key(case.newline);
            type_chars(&mut app, case.after);
            assert_eq!(app.input.lines(), case.expected);
            assert!(app.messages.is_empty());
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn plain_enter_sends_message() {
        let mut app = App::new(Config::for_test());
        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

        assert_eq!(app.messages.len(), 2);
        assert_eq!(app.messages[0].role, "user");
        assert_eq!(app.messages[0].content, "hi");
    }

    #[test]
    fn up_arrow_scrolls_chat_when_input_empty() {
        let mut app = app_with_long_chat();
        app.auto_scroll = true;
        app.sync_scroll(app.viewport_height);
        let at_bottom = app.scroll;

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));

        assert!(!app.auto_scroll);
        assert!(app.scroll < at_bottom);
    }

    #[test]
    fn start_new_session_clears_messages_and_assigns_new_id() {
        let mut app = App::new(Config::for_test());
        let old_id = app.session_id.clone();
        app.messages.push(Message {
            role: "user".into(),
            content: "hello".into(),
            thinking: String::new(),
        });
        app.status = Some("old".into());

        app.start_new_session();

        assert_ne!(app.session_id, old_id);
        assert!(app.messages.is_empty());
        assert!(app.status.is_none());
    }

    #[test]
    fn delete_selected_session_clamps_selection() {
        let _guard = crate::store::TEST_DIR_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("aitui-test-{}", new_session_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("AITUI_TEST_SESSIONS_DIR", &tmp);

        for (id, title) in [("s1", "one"), ("s2", "two"), ("s3", "three")] {
            store::save(&Session {
                id: id.into(),
                title: title.into(),
                created_at: 1,
                updated_at: 1,
                messages: vec![Message {
                    role: "user".into(),
                    content: title.into(),
                    thinking: String::new(),
                }],
            })
            .unwrap();
        }

        let mut app = App::new(Config::for_test());
        app.resume_overlay = Some(ResumeOverlay {
            sessions: vec![
                SessionMeta {
                    id: "s1".into(),
                    title: "one".into(),
                    updated_at: 1,
                },
                SessionMeta {
                    id: "s2".into(),
                    title: "two".into(),
                    updated_at: 1,
                },
                SessionMeta {
                    id: "s3".into(),
                    title: "three".into(),
                    updated_at: 1,
                },
            ],
            selected: 2,
        });

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::empty()));

        let overlay = app.resume_overlay.as_ref().unwrap();
        assert_eq!(overlay.sessions.len(), 2);
        assert_eq!(overlay.selected, 1);
        assert!(app.status.as_ref().unwrap().contains("three"));

        std::env::remove_var("AITUI_TEST_SESSIONS_DIR");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

fn wrap_text(text: &str, width: u16) -> Vec<String> {
    let w = width.max(10) as usize;
    text.lines()
        .flat_map(|line| {
            if line.is_empty() {
                vec![String::new()]
            } else {
                crate::markdown::wrap_line(line, w)
            }
        })
        .collect()
}
