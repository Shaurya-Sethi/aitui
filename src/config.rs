use anyhow::{Context, Result};
use clap::Parser;
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "aitui", about = "Lightweight terminal chat UI")]
pub struct Cli {
    #[arg(long, help = "OpenAI-compatible base URL")]
    pub url: Option<String>,

    #[arg(long, help = "Model name")]
    pub model: Option<String>,

    #[arg(long, help = "API key")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
}

impl Config {
    pub fn load(cli: &Cli) -> Result<Self> {
        let path = config_path()?;
        let file = if path.exists() {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            toml::from_str(&raw).context("parse config.toml")?
        } else {
            write_default_config(&path)?;
            FileConfig::default()
        };

        let base_url = env_or(cli.url.clone(), "AITUI_BASE_URL")
            .or(file.base_url)
            .unwrap_or_else(|| "http://localhost:11434/v1".into());

        let model = env_or(cli.model.clone(), "AITUI_MODEL")
            .or(file.model)
            .unwrap_or_else(|| "llama3".into());

        let api_key = env_or(cli.api_key.clone(), "AITUI_API_KEY").or(file.api_key);

        Ok(Self {
            base_url: normalize_url(&base_url),
            model,
            api_key,
        })
    }

    pub fn endpoint_host(&self) -> String {
        url::host_str(&self.base_url)
            .map(|h| {
                if let Some(port) = url::port_str(&self.base_url) {
                    format!("{h}:{port}")
                } else {
                    h.to_string()
                }
            })
            .unwrap_or_else(|| self.base_url.clone())
    }
}

fn config_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "aitui").context("resolve config dir")?;
    Ok(dirs.config_dir().join("config.toml"))
}

fn write_default_config(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let template = r#"# aitui configuration
# base_url = "http://localhost:11434/v1"
# model = "llama3"
# api_key = "sk-..."
"#;
    fs::write(path, template)?;
    Ok(())
}

fn env_or(cli: Option<String>, key: &str) -> Option<String> {
    cli.or_else(|| std::env::var(key).ok())
}

fn normalize_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

// ponytail: tiny url parse without extra crate
mod url {
    pub fn host_str(url: &str) -> Option<&str> {
        let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://"))?;
        let host = rest.split('/').next()?;
        Some(host.split(':').next()?)
    }

    pub fn port_str(url: &str) -> Option<&str> {
        let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://"))?;
        let host = rest.split('/').next()?;
        host.split(':').nth(1)
    }
}
