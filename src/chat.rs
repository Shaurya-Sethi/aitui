use crate::config::Config;
use crate::store::Message;
use anyhow::{Context, Result};
use futures_util::StreamExt;
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
            let _ = tx.send(ChatEvent::Error(e.to_string())).await;
        }
    });
    rx
}

async fn do_stream(
    config: &Config,
    messages: &[Message],
    tx: &mpsc::Sender<ChatEvent>,
) -> Result<()> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .context("HTTP client")?;
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

    let response = req.send().await.context("request failed")?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {status}: {text}");
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("stream read")?;
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

async fn emit_sse_line(line: &str, tx: &mpsc::Sender<ChatEvent>) -> Result<bool> {
    if line.is_empty() {
        return Ok(false);
    }
    for event in parse_sse_line(line) {
        let done = matches!(event, ChatEvent::Done);
        let _ = tx.send(event).await;
        if done {
            return Ok(true);
        }
    }
    Ok(false)
}

fn parse_sse_line(line: &str) -> Vec<ChatEvent> {
    let line = line.trim();
    if line.is_empty() {
        return Vec::new();
    }

    // OpenAI SSE uses "data: …"; some providers stream raw NDJSON lines.
    let payload = line.strip_prefix("data: ").unwrap_or(line);
    if payload == "[DONE]" {
        return vec![ChatEvent::Done];
    }

    let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) else {
        return Vec::new();
    };

    if json.get("done").and_then(|v| v.as_bool()) == Some(true) {
        return vec![ChatEvent::Done];
    }

    let delta = &json["choices"][0]["delta"];
    let mut events = Vec::new();
    for key in ["reasoning_content", "reasoning", "thinking"] {
        if let Some(text) = delta[key].as_str() {
            if !text.is_empty() {
                events.push(ChatEvent::ThinkingToken(text.to_string()));
                break;
            }
        }
    }
    if let Some(content) = delta["content"].as_str() {
        if !content.is_empty() {
            events.push(ChatEvent::Token(content.to_string()));
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_token_chunk() {
        let line = r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#;
        let events = parse_sse_line(line);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], ChatEvent::Token(t) if t == "hi"));
    }

    #[test]
    fn parse_done() {
        let events = parse_sse_line("data: [DONE]");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ChatEvent::Done));
    }

    #[test]
    fn parse_raw_ndjson_token() {
        let line = r#"{"choices":[{"delta":{"content":"hi"}}]}"#;
        let events = parse_sse_line(line);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], ChatEvent::Token(t) if t == "hi"));
    }

    #[test]
    fn parse_reasoning_token() {
        let line = r#"data: {"choices":[{"delta":{"reasoning_content":"hmm"}}]}"#;
        let events = parse_sse_line(line);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], ChatEvent::ThinkingToken(t) if t == "hmm"));
    }

    #[test]
    fn parse_reasoning_and_content_same_delta() {
        let line = r#"data: {"choices":[{"delta":{"reasoning_content":"hmm","content":"hi"}}]}"#;
        let events = parse_sse_line(line);
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], ChatEvent::ThinkingToken(t) if t == "hmm"));
        assert!(matches!(&events[1], ChatEvent::Token(t) if t == "hi"));
    }
}
