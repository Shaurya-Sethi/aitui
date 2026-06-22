use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub thinking: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub updated_at: u64,
}

pub fn list() -> Result<Vec<SessionMeta>> {
    let dir = sessions_dir()?;
    let index_path = dir.join("index.json");
    if !index_path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&index_path)?;
    match serde_json::from_str::<Vec<SessionMeta>>(&raw) {
        Ok(mut metas) => {
            metas.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(metas)
        }
        Err(_) => {
            let metas = rebuild_index_from_disk()?;
            let _ = write_index(&dir, &metas);
            Ok(metas)
        }
    }
}

pub fn load(id: &str) -> Result<Session> {
    let path = sessions_dir()?.join(format!("{id}.json"));
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("load session {}", path.display()))?;
    serde_json::from_str(&raw).context("parse session")
}

pub fn save(session: &Session) -> Result<()> {
    let dir = sessions_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", session.id));
    fs::write(&path, serde_json::to_string_pretty(session)?)?;

    let mut metas = list()?;
    metas.retain(|m| m.id != session.id);
    metas.push(SessionMeta {
        id: session.id.clone(),
        title: session.title.clone(),
        updated_at: session.updated_at,
    });
    metas.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    write_index(&dir, &metas)?;
    Ok(())
}

pub fn delete(id: &str) -> Result<()> {
    let dir = sessions_dir()?;
    let path = dir.join(format!("{id}.json"));
    if path.exists() {
        fs::remove_file(&path)?;
    }
    let mut metas = list()?;
    metas.retain(|m| m.id != id);
    if dir.exists() {
        write_index(&dir, &metas)?;
    }
    Ok(())
}

fn rebuild_index_from_disk() -> Result<Vec<SessionMeta>> {
    let dir = sessions_dir()?;
    let mut metas = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if name == "index" {
            continue;
        }
        if let Ok(session) = load(name) {
            metas.push(SessionMeta {
                id: session.id,
                title: session.title,
                updated_at: session.updated_at,
            });
        }
    }
    metas.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(metas)
}

fn write_index(dir: &std::path::Path, metas: &[SessionMeta]) -> Result<()> {
    let index_path = dir.join("index.json");
    let tmp_path = dir.join("index.json.tmp");
    fs::write(&tmp_path, serde_json::to_string_pretty(metas)?)?;
    fs::rename(tmp_path, index_path)?;
    Ok(())
}

pub fn new_session_id() -> String {
    format!(
        "{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn title_from_messages(messages: &[Message]) -> String {
    messages
        .iter()
        .find(|m| m.role == "user")
        .map(|m| m.content.chars().take(60).collect())
        .unwrap_or_else(|| "New chat".into())
}

fn sessions_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("AITUI_TEST_SESSIONS_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let dirs = ProjectDirs::from("", "", "aitui").context("resolve data dir")?;
    Ok(dirs.data_dir().join("sessions"))
}

pub fn relative_time(ts: u64) -> String {
    let now = now_secs();
    let diff = now.saturating_sub(ts);
    if diff < 60 {
        "just now".into()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

#[cfg(test)]
pub(crate) static TEST_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_truncates_first_user_message() {
        let msgs = vec![Message {
            role: "user".into(),
            content: "a".repeat(80),
            thinking: String::new(),
        }];
        assert_eq!(title_from_messages(&msgs).len(), 60);
    }

    #[test]
    fn delete_removes_session_file_and_index_entry() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("aitui-test-{}", new_session_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("AITUI_TEST_SESSIONS_DIR", &tmp);

        let session = Session {
            id: "test-session".into(),
            title: "Test".into(),
            created_at: 1,
            updated_at: 1,
            messages: vec![Message {
                role: "user".into(),
                content: "hi".into(),
                thinking: String::new(),
            }],
        };
        save(&session).unwrap();
        assert_eq!(list().unwrap().len(), 1);

        delete("test-session").unwrap();
        assert!(list().unwrap().is_empty());
        assert!(!tmp.join("test-session.json").exists());

        std::env::remove_var("AITUI_TEST_SESSIONS_DIR");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn save_rebuilds_index_when_corrupt() {
        let _guard = TEST_DIR_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("aitui-test-{}", new_session_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("AITUI_TEST_SESSIONS_DIR", &tmp);

        for (id, title) in [("s1", "one"), ("s2", "two")] {
            save(&Session {
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
        assert_eq!(list().unwrap().len(), 2);

        std::fs::write(tmp.join("index.json"), "{not json").unwrap();
        save(&Session {
            id: "s1".into(),
            title: "one".into(),
            created_at: 1,
            updated_at: 2,
            messages: vec![Message {
                role: "user".into(),
                content: "one".into(),
                thinking: String::new(),
            }],
        })
        .unwrap();

        let metas = list().unwrap();
        assert_eq!(metas.len(), 2);
        assert!(metas.iter().any(|m| m.id == "s1"));
        assert!(metas.iter().any(|m| m.id == "s2"));

        std::env::remove_var("AITUI_TEST_SESSIONS_DIR");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
