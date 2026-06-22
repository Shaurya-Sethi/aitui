# aitui

<p align="center">
  <img src="assets/demo.gif" alt="aitui demo" width="800" />
</p>

A minimal terminal chat UI for local models. OpenAI-compatible API, streaming, session history. Under 2k lines of Rust, ~3 MB RAM.

## Why

Most UIs like Open WebUI are heavy and run in the browser, which also uses a lot of memory. Terminal tools like `aichat` are often more feature-rich than I need. I wanted something lightweight and modern I could run in the terminal to chat with my local models.

This project intentionally leaves agent harnesses, tools, skills, system instructions, and similar out of scope. For that, use something like [pi](https://github.com/badlogic/pi-mono) or [opencode](https://github.com/anomalyco/opencode).

## Install

```sh
git clone https://github.com/Shaurya-Sethi/aitui.git
cd aitui
cargo install --path .
```

## Run

```sh
aitui
```

With no flags or env vars, `aitui` runs interactive setup and asks for base URL, model, and API key (API key can be left empty for local models).

```sh
aitui --url http://localhost:8080/v1 --model lmstudio-community/Qwen3.6-27B-MLX-4bit
```

Or set `AITUI_BASE_URL`, `AITUI_MODEL`, and optionally `AITUI_API_KEY`.
