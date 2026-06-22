use std::env;
use std::io::{self, BufRead, IsTerminal, Write};
use std::process;

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

Run `aitui` with no configuration to start interactive setup.
";

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let mut url = None;
        let mut model = None;
        let mut api_key = None;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    print!("{HELP}");
                    process::exit(0);
                }
                "--url" => url = args.next(),
                "--model" => model = args.next(),
                "--api-key" => api_key = args.next(),
                other => {
                    eprintln!("unknown argument: {other}\n\n{HELP}");
                    process::exit(1);
                }
            }
        }

        let base_url = env_or(url, "AITUI_BASE_URL");
        let model = env_or(model, "AITUI_MODEL");
        let api_key = env_or(api_key, "AITUI_API_KEY");

        let (base_url, model, api_key) = if base_url.is_none() || model.is_none() {
            if io::stdin().is_terminal() {
                interactive_setup(base_url, model, api_key)
            } else {
                eprintln!("configuration required. Set flags or env vars.\n\n{HELP}");
                process::exit(1);
            }
        } else {
            (base_url.unwrap(), model.unwrap(), api_key)
        };

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

#[cfg(test)]
impl Config {
    pub fn for_test() -> Self {
        Self {
            base_url: "http://localhost:11434/v1".into(),
            model: "test".into(),
            api_key: None,
        }
    }
}

fn env_or(cli: Option<String>, key: &str) -> Option<String> {
    cli.or_else(|| env::var(key).ok())
}

fn interactive_setup(
    base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
) -> (String, String, Option<String>) {
    eprintln!("aitui setup\n");

    let base_url = base_url.unwrap_or_else(|| prompt_required("Base URL"));
    let model = model.unwrap_or_else(|| prompt_required("Model"));
    let api_key = api_key.or_else(|| {
        eprint!("API key (leave empty for local): ");
        io::stderr().flush().ok();
        let mut line = String::new();
        io::stdin().lock().read_line(&mut line).ok();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    (base_url, model, api_key)
}

fn prompt_required(label: &str) -> String {
    loop {
        eprint!("{label}: ");
        io::stderr().flush().ok();
        let mut line = String::new();
        io::stdin().lock().read_line(&mut line).ok();
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        eprintln!("required");
    }
}

fn normalize_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_appends_v1_once() {
        assert_eq!(
            normalize_url("http://localhost:11434"),
            "http://localhost:11434/v1"
        );
        assert_eq!(
            normalize_url("http://localhost:11434/"),
            "http://localhost:11434/v1"
        );
        assert_eq!(
            normalize_url("http://localhost:11434/v1"),
            "http://localhost:11434/v1"
        );
        assert_eq!(
            normalize_url("http://localhost:11434/v1/"),
            "http://localhost:11434/v1"
        );
    }

    #[test]
    fn endpoint_host_includes_port() {
        let cfg = Config {
            base_url: "http://localhost:11434/v1".into(),
            model: "m".into(),
            api_key: None,
        };
        assert_eq!(cfg.endpoint_host(), "localhost:11434");
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
