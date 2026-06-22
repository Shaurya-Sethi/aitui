use crate::config::Config;
use crate::store::Message;
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ChatEvent {
    Token(String),
    ThinkingToken(String),
    Done,
    Error(String),
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ApiMessage<'a>],
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ApiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

pub fn stream_completion(
    config: &Config,
    messages: &[Message],
) -> mpsc::Receiver<ChatEvent> {
    let (tx, rx) = mpsc::channel(256);
    let config = config.clone();
    let messages = messages.to_vec();
    tokio::spawn(async move {
        if let Err(e) = do_stream(&config, &messages, &tx).await {
            let _ = tx.send(ChatEvent::Error(e)).await;
        }
    });
    rx
}

async fn do_stream(
    config: &Config,
    messages: &[Message],
    tx: &mpsc::Sender<ChatEvent>,
) -> Result<(), String> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;
    let url = format!("{}/chat/completions", config.base_url);
    let api_messages: Vec<ApiMessage<'_>> = messages
        .iter()
        .filter(|m| !m.content.is_empty())
        .map(|m| ApiMessage {
            role: &m.role,
            content: &m.content,
        })
        .collect();
    let body = ChatRequest {
        model: &config.model,
        messages: &api_messages,
        stream: true,
    };

    let mut req = client.post(&url).json(&body);
    if let Some(key) = &config.api_key {
        req = req.bearer_auth(key);
    }

    let mut response = req
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {text}"));
    }

    let mut buffer = String::new();

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("stream read: {e}"))?
    {
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();
            if emit_sse_line(line.trim(), tx).await? {
                return Ok(());
            }
        }
    }

    if !buffer.trim().is_empty() {
        if emit_sse_line(buffer.trim(), tx).await? {
            return Ok(());
        }
    }

    let _ = tx.send(ChatEvent::Done).await;
    Ok(())
}

async fn emit_sse_line(line: &str, tx: &mpsc::Sender<ChatEvent>) -> Result<bool, String> {
    if line.is_empty() {
        return Ok(false);
    }
    let mut events = parse_sse_line(line);
    while let Some(event) = events.next() {
        let done = matches!(event, ChatEvent::Done);
        let _ = tx.send(event).await;
        if done {
            return Ok(true);
        }
    }
    Ok(false)
}

fn parse_sse_line(line: &str) -> SseEvents {
    let mut out = SseEvents::default();
    let line = line.trim();
    if line.is_empty() {
        return out;
    }

    // OpenAI SSE uses "data: …"; some providers stream raw NDJSON lines.
    let payload = line.strip_prefix("data: ").unwrap_or(line);
    if payload == "[DONE]" {
        out.push(ChatEvent::Done);
        return out;
    }

    let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) else {
        return out;
    };

    if json.get("done").and_then(|v| v.as_bool()) == Some(true) {
        out.push(ChatEvent::Done);
        return out;
    }

    let delta = &json["choices"][0]["delta"];
    for key in ["reasoning_content", "reasoning", "thinking"] {
        if let Some(text) = delta[key].as_str() {
            if !text.is_empty() {
                out.push(ChatEvent::ThinkingToken(text.to_string()));
                break;
            }
        }
    }
    if let Some(content) = delta["content"].as_str() {
        if !content.is_empty() {
            out.push(ChatEvent::Token(content.to_string()));
        }
    }

    out
}

#[derive(Default)]
struct SseEvents {
    events: [Option<ChatEvent>; 2],
    len: usize,
}

impl SseEvents {
    fn push(&mut self, event: ChatEvent) {
        if self.len < self.events.len() {
            self.events[self.len] = Some(event);
            self.len += 1;
        }
    }

    fn next(&mut self) -> Option<ChatEvent> {
        if self.len == 0 {
            return None;
        }
        let event = self.events[0].take()?;
        for i in 1..self.len {
            self.events[i - 1] = self.events[i].take();
        }
        self.events[self.len - 1] = None;
        self.len -= 1;
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_events(line: &str) -> Vec<ChatEvent> {
        let mut events = parse_sse_line(line);
        let mut out = Vec::new();
        while let Some(event) = events.next() {
            out.push(event);
        }
        out
    }

    #[test]
    fn parse_token_chunk() {
        let line = r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#;
        let events = collect_events(line);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], ChatEvent::Token(t) if t == "hi"));
    }

    #[test]
    fn parse_done() {
        let events = collect_events("data: [DONE]");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ChatEvent::Done));
    }

    #[test]
    fn parse_raw_ndjson_token() {
        let line = r#"{"choices":[{"delta":{"content":"hi"}}]}"#;
        let events = collect_events(line);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], ChatEvent::Token(t) if t == "hi"));
    }

    #[test]
    fn parse_reasoning_token() {
        let line = r#"data: {"choices":[{"delta":{"reasoning_content":"hmm"}}]}"#;
        let events = collect_events(line);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], ChatEvent::ThinkingToken(t) if t == "hmm"));
    }

    #[test]
    fn parse_reasoning_and_content_same_delta() {
        let line = r#"data: {"choices":[{"delta":{"reasoning_content":"hmm","content":"hi"}}]}"#;
        let events = collect_events(line);
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], ChatEvent::ThinkingToken(t) if t == "hmm"));
        assert!(matches!(&events[1], ChatEvent::Token(t) if t == "hi"));
    }
}
