use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "aitui",
    about = "Lightweight terminal chat UI",
    after_help = "Environment variables (used when flags are omitted):\n  AITUI_BASE_URL   OpenAI-compatible base URL\n  AITUI_MODEL      Model name\n  AITUI_API_KEY    API key"
)]
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

impl Config {
    pub fn load(cli: &Cli) -> Self {
        let base_url = env_or(cli.url.clone(), "AITUI_BASE_URL")
            .unwrap_or_else(|| "http://localhost:11434/v1".into());

        let model = env_or(cli.model.clone(), "AITUI_MODEL").unwrap_or_else(|| "llama3".into());

        let api_key = env_or(cli.api_key.clone(), "AITUI_API_KEY");

        Self {
            base_url: normalize_url(&base_url),
            model,
            api_key,
        }
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

mod url {
    pub fn host_str(url: &str) -> Option<&str> {
        let rest = url.strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))?;
        let host = rest.split('/').next()?;
        Some(host.split(':').next()?)
    }

    pub fn port_str(url: &str) -> Option<&str> {
        let rest = url.strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))?;
        let host = rest.split('/').next()?;
        host.split(':').nth(1)
    }
}
