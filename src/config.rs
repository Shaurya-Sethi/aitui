use std::env;
use std::process;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

const HELP: &str = "\
Lightweight terminal chat UI

Usage: aitui [OPTIONS]

Options:
      --url <URL>          OpenAI-compatible base URL
      --model <MODEL>      Model name
      --api-key <KEY>      API key
  -h, --help               Print help

Environment variables (used when flags are omitted):
  AITUI_BASE_URL   OpenAI-compatible base URL
  AITUI_MODEL      Model name
  AITUI_API_KEY    API key
";

pub fn parse_args() -> CliArgs {
    let mut args = env::args().skip(1);
    let mut out = CliArgs::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                process::exit(0);
            }
            "--url" => out.url = args.next(),
            "--model" => out.model = args.next(),
            "--api-key" => out.api_key = args.next(),
            other => {
                eprintln!("unknown argument: {other}\n\n{HELP}");
                process::exit(1);
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl Config {
    pub fn load(args: &CliArgs) -> Self {
        let base_url = env_or(args.url.clone(), "AITUI_BASE_URL")
            .unwrap_or_else(|| "http://localhost:11434/v1".into());

        let model = env_or(args.model.clone(), "AITUI_MODEL").unwrap_or_else(|| "llama3".into());

        let api_key = env_or(args.api_key.clone(), "AITUI_API_KEY");

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
    cli.or_else(|| env::var(key).ok())
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
        let rest = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))?;
        let host = rest.split('/').next()?;
        Some(host.split(':').next()?)
    }

    pub fn port_str(url: &str) -> Option<&str> {
        let rest = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))?;
        let host = rest.split('/').next()?;
        host.split(':').nth(1)
    }
}
