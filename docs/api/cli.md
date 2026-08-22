---
type: API
title: CLI API
description: The activity-tracker command surface defined with clap, including start, stop, status, collect, summary, export, tui, tag, clean-logs, update, and config subcommands.
tags: [cli, clap, commands, flags, interface]
status: stable
---

# CLI API

**Defined in**: `src/main.rs` via `clap` derive macros.
**Aliases** (set up by the installer): `at` -> `activity-tracker`, `ats` -> `activity-tracker summary`.

## Top-Level

```
activity-tracker [COMMAND] [--version]
```

## Subcommands

### `start` - run the daemon

| Flag | Type | Default | Description |
|---|---|---|---|
| `-i, --interval` | u64 | `5` | Collection interval in minutes |
| `--foreground` | flag | false | Run in foreground (used by systemd/launchd) |

Without `--foreground`, re-spawns itself detached. See [Daemon Service](../services/daemon.md).

### `stop` - stop the background daemon

Removes the PID file and sends `kill` to the recorded PID.

### `status` - show daemon status

Checks the PID file and `/proc/{pid}` (Linux) to report `rodando` / `parado`.

### `collect` - one-shot manual collection

Runs [Collection Pipeline](../pipeline/collection.md) once and prints the entry count.

### `summary` - generate an LLM summary (alias target of `ats`)

See [Summary Generation Pipeline](../pipeline/summary-generation.md).

| Flag | Type | Default | Description |
|---|---|---|---|
| `-d, --days` | u32 | `3` | Days back to summarize |
| `--date` | String | - | Specific date `YYYY-MM-DD` |
| `-m, --model` | String | config | Model override |
| `--provider` | String | config | `ollama\|openai\|anthropic\|groq\|gemini\|openrouter` |
| `--ollama-url` | String | config | Ollama URL (provider=ollama) |
| `--api-key` | String | config | API key override |
| `--lang` | String | config | Summary language |
| `--today` | flag | - | Shortcut for `--days 1` |
| `--week` | flag | - | Shortcut for `--days 7` |
| `--month` | flag | - | Shortcut for `--days 30` |
| `--search` | String | - | Filter logs by term before summarizing |
| `--send-notion` | flag | - | Send result to Notion |
| `--send-slack` | flag | - | Send result to Slack |

### `export` - export logs to CSV/JSON

See [Export Pipeline](../pipeline/export.md).

| Flag | Type | Default | Description |
|---|---|---|---|
| `-d, --days` | u32 | `1` | Days back |
| `--date` | String | - | Specific date |
| `--format` | String | `csv` | `csv\|json` |
| `-o, --output` | String | stdout | Output file |

### `tui` - interactive terminal UI

See [TUI Service](../services/tui.md). Accepts `--model`, `--provider`, `--ollama-url`,
`--api-key`, `--lang` overrides.

### `tag` - manual annotations

| Arg/Flag | Description |
|---|---|
| `[label]` | Tag text (omit to list today's tags) |
| `-l, --list` | List tags instead of adding |
| `--delete` | Delete the tag matching `label` |
| `--date` | Target date `YYYY-MM-DD` (default: today) |

### `clean-logs` - prune stale entries

Runs `clean_all_logs` (date mismatch) + `purge_ignored_git_repos` (ignored paths) over all
log files.

### `update` - self-update

See [Self-Update Pipeline](../pipeline/self-update.md).

### `config` - persistent configuration

Sub-actions (each calls `Config::save()`):

| Action | Purpose |
|---|---|
| `set-model <name>` | Default model |
| `set-provider <name>` | Provider (validated against the six supported) |
| `set-url <url>` | Ollama URL |
| `set-api-key <key>` | Cloud provider API key |
| `set-lang <lang>` | Summary language |
| `set-machine-name <name>` | Machine name for titles |
| `set-notion-token <token>` | Notion integration token |
| `set-notion-page <page_id>` | Notion parent page id |
| `set-slack-webhook <url>` | Slack incoming webhook |
| `add-block <pattern>` | Add privacy block pattern |
| `remove-block <pattern>` | Remove privacy block pattern |
| `list-blocks` | List active block patterns |
| `set-prompt <template>` | Custom LLM prompt (`{context}`, `{lang}` placeholders) |
| `clear-prompt` | Revert to default prompt |
| `show` | Print current config |

See [Config Data Model](../data/config.md).

## Related

- Dispatched from `main.rs` to [Daemon](../services/daemon.md), [Collector](../services/collector.md),
  [Summarizer](../services/summarizer.md), [TUI](../services/tui.md), and [Updater](../services/updater.md) services
- Aliases installed by [Installer](../infrastructure/installer.md)
