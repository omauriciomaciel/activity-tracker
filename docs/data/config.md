---
type: Data Model
title: Config
description: Persistent application configuration serialized as TOML at ~/.config/activity-tracker/config.toml, holding LLM provider settings, credentials, privacy patterns, and integration tokens.
tags: [config, toml, settings, credentials, privacy]
status: stable
---

# Config

**Module**: `src/config.rs`
**Format**: TOML
**Path**: `~/.config/activity-tracker/config.toml` (via `dirs::config_dir()`)

## Structure

```rust
pub struct Config {
    pub model: String,                  // e.g. "llama3.1", "gpt-4o-mini"
    pub provider: String,               // ollama | openai | anthropic | groq | gemini | openrouter
    pub ollama_url: String,             // e.g. "http://localhost:11434"
    pub api_key: Option<String>,        // for cloud providers
    pub lang: String,                   // pt-br | en | es | fr | de | ja | zh
    pub notion_token: Option<String>,   // Notion integration token
    pub notion_page_id: Option<String>, // Notion parent page id
    pub machine_name: Option<String>,   // overrides hostname in titles
    pub slack_webhook: Option<String>,  // Slack incoming webhook URL
    pub blocked_patterns: Vec<String>,  // privacy blocklist (case-insensitive substring)
    pub ignored_git_paths: Vec<String>, // git repo path prefixes to skip
    pub custom_prompt: Option<String>,  // custom LLM prompt template ({context}, {lang})
}
```

## Defaults

| Field | Default |
|---|---|
| `model` | `llama3.1` |
| `provider` | `ollama` |
| `ollama_url` | `http://localhost:11434` |
| `api_key` | `None` |
| `lang` | `pt-br` |
| all integrations | `None` |
| `blocked_patterns` | `[]` |
| `ignored_git_paths` | `[]` |
| `custom_prompt` | `None` |

## Lifecycle

- **Load**: `Config::load()` reads the TOML file if it exists; otherwise creates it with
  defaults and saves. Deserialization uses serde with `default =` attributes so missing fields
  in an older config file fall back to defaults.
- **Save**: `Config::save()` serializes with `toml::to_string_pretty` and writes back,
  creating the parent directory first.
- **Display**: `display()` renders a human-readable summary (provider, model, masked API key
  showing first 6 + last 4 chars, language, prompt preview, integration status, blocked
  patterns, ignored paths, log/config paths).

## Machine Name Resolution

`get_machine_name()` returns `machine_name` if set, otherwise runs `hostname` and trims it,
falling back to `"unknown"`. Used in Notion/Slack titles.

## Path Helpers

| Function | Returns |
|---|---|
| `base_dir()` | `<config_dir>/activity-tracker` |
| `config_path()` | `<base_dir>/config.toml` |
| `data_dir()` | `<data_local_dir>/activity-tracker` |
| `log_dir()` | `<data_dir>/logs` |
| `summary_dir()` | `<data_dir>/summaries` |
| `summary_path(date)` | `<summary_dir>/YYYY-MM-DD.md` |

See [Storage Layout](../infrastructure/storage-layout.md) for the full on-disk map.

## Mutation Surface

Config is mutated by the `config` CLI subcommands ([CLI API](../api/cli.md)) and the TUI Config
tab ([TUI Service](../services/tui.md)). Both call `Config::save()` after each change.

## Related

- Loaded by [Daemon Service](../services/daemon.md) on every tick
- Drives [LLM Provider APIs](../api/llm-providers.md) selection
- Credentials used by [Notion Integration](../services/notion.md) and [Slack Integration](../services/slack.md)
- Edited via [CLI API](../api/cli.md) and [TUI Service](../services/tui.md)
