---
type: System Overview
title: Activity Tracker - System Overview
description: A Rust daemon/CLI that captures system activity and generates LLM-powered summaries of what the user did.
tags: [activity-tracker, rust, daemon, cli, llm, productivity, overview]
status: stable
---

# System Overview

**Activity Tracker** is a single-binary Rust application (edition 2024, v1.13.0) that runs as
a background daemon, periodically snapshots user activity across the system, persists it as
daily JSONL logs, and can generate natural-language summaries via an LLM. Summaries may be
sent to Notion (as a sub-page) or Slack (as a Block Kit message).

## What It Does

1. **Captures** activity from four sources on a fixed interval:
   - Shell history (Bash, Zsh, Fish) - incremental, only new commands
   - Open windows (wmctrl / xdotool / osascript / ps / WSL PowerShell)
   - Chrome / Brave tabs (DevTools Protocol on port 9222, or SQLite history DB)
   - Git context (commits from the current day, up to 4 levels below `$HOME`, max 15 repos)
2. **Stores** each day as a JSONL file under `~/.local/share/activity-tracker/logs/`.
3. **Summarizes** aggregated activity through a pluggable LLM provider.
4. **Distributes** summaries to Notion and/or Slack on demand.
5. **Presents** an interactive terminal UI (ratatui) for browsing days, summaries, project stats, and editing config.

## High-Level Architecture

```
        ┌─────────────┐    every N min     ┌──────────────────┐
  user  │   daemon    │ ─────────────────▶ │    collector     │
        │ (foreground │                    │  shell/windows/  │
        │  or bg proc)│ ◀── entries ────── │  tabs/git        │
        └─────────────┘                    └──────────────────┘
               │                                    │
               │ PID file                           │ append
               ▼                                    ▼
        ┌─────────────┐                    ┌──────────────────┐
        │   config    │                    │  daily JSONL log │
        │ config.toml │                    │  YYYY-MM-DD.jsonl│
        └─────────────┘                    └──────────────────┘
               │                                    │
               │                                    │ aggregate
               ▼                                    ▼
        ┌─────────────┐   prompt          ┌──────────────────┐
        │ summarizer  │ ───────────────▶  │   LLM provider   │
        │ (aggregate) │ ◀── summary ────── │ ollama/openai/...│
        └─────────────┘                    └──────────────────┘
               │
               ├──▶ terminal (termimad markdown)
               ├──▶ Notion API (sub-page)
               ├──▶ Slack webhook (Block Kit)
               └──▶ summaries/YYYY-MM-DD.md (cached)
```

## Components at a Glance

| Component | Module | Role |
|---|---|---|
| CLI entry | `src/main.rs` | clap command dispatch |
| Daemon | [Daemon Service](../services/daemon.md) | interval loop, PID, lifecycle |
| Collector | [Collector Service](../services/collector.md) | capture + persist activity |
| Summarizer | [Summarizer Service](../services/summarizer.md) | aggregate, call LLM, export |
| TUI | [TUI Service](../services/tui.md) | interactive terminal interface |
| Projects | [Projects Service](../services/projects.md) | repo time-distribution stats |
| Updater | [Updater Service](../services/updater.md) | self-update via GitHub |
| Notion | [Notion Integration](../services/notion.md) | send summary as Notion page |
| Slack | [Slack Integration](../services/slack.md) | send summary to Slack channel |

## Configuration

Persisted as TOML at `~/.config/activity-tracker/config.toml`. See
[Config Data Model](../data/config.md). Supports six LLM providers, privacy block-lists,
ignored git paths, custom prompts, and Notion/Slack credentials.

## External Dependencies

- **Language runtime**: Rust (edition 2024), async via `tokio`.
- **Key crates**: `clap` (CLI), `reqwest` (HTTP), `rusqlite` (Chrome history),
  `ratatui` + `crossterm` (TUI), `serde`/`serde_json` (serialization), `flate2`/`tar`/`zip`
  (self-update extraction), `termimad`/`colored` (output), `toml` (config).
- **System tools** (probed at runtime): `find`, `git`, `wmctrl`, `xdotool`, `osascript`,
  `ps`, `powershell.exe` (WSL).
- **External services**: LLM providers ([LLM Provider APIs](../api/llm-providers.md)),
  Notion API, Slack webhooks, GitHub Releases API (updates).

## Supported Platforms

- **macOS** (x86_64, aarch64) - LaunchAgent autostart; requires Full Disk Access + Accessibility.
- **Linux** (x86_64, aarch64) - systemd user service autostart; optional `wmctrl` for X11.

## Distribution

Installed via `install.sh` (downloads pre-built binary from GitHub Releases) or built from
source with `cargo build --release`. Release artifacts (.deb, .tar.gz, .dmg) are produced by
the [CI/CD Pipeline](../infrastructure/ci-cd.md).
