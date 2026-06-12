# Activity Tracker

A Rust daemon that captures system activity (terminal history, open windows, Chrome/Brave tabs, Git repositories) and generates intelligent summaries via LLM. Supports Ollama (local), OpenAI, Anthropic, Groq, Gemini, and OpenRouter. Optionally sends summaries to [Notion](https://notion.so) or a [Slack](https://slack.com) channel via webhook.

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/omauriciomaciel/activity-tracker/main/install.sh | sh
```

The script detects your OS/architecture, downloads the pre-built binary from the latest GitHub release, installs it to `~/.local/bin`, configures autostart, and adds shell aliases:

- **macOS** — creates a LaunchAgent under `~/Library/LaunchAgents/` and opens the required permission screens
- **Linux (systemd)** — creates and enables a systemd user service

After installation, two aliases become available:

```
at   →  activity-tracker
ats  →  activity-tracker summary
```

Custom install directory:

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/omauriciomaciel/activity-tracker/main/install.sh | sh
```

Or build from source:

```bash
cargo build --release
./target/release/activity-tracker --version
```

**Prerequisites:**
- **LLM** — local Ollama (`ollama serve`) or an API key from a cloud provider
- Optional: `wmctrl` for X11 window capture (`sudo apt install wmctrl`)

## LLM Providers

| Provider | Example models | Requires |
|---|---|---|
| `ollama` | llama3.1, mistral, gemma2 | Ollama running locally |
| `openai` | gpt-4o, gpt-4o-mini | OpenAI API key |
| `anthropic` | claude-sonnet-4-6, claude-haiku-4-5-20251001 | Anthropic API key |
| `groq` | llama-3.1-8b-instant, mixtral-8x7b | Groq API key |
| `gemini` | gemini-2.0-flash, gemini-1.5-pro | Google API key |
| `openrouter` | any available model | OpenRouter API key |

Default is `ollama`. To switch provider:

```bash
at config set-provider openai
at config set-api-key sk-...
at config set-model gpt-4o-mini
```

## Usage

```bash
# Background daemon
at start                     # start in background
at start --interval 10       # custom interval in minutes
at stop                      # stop the daemon
at status                    # check if running

# Single manual collection
at collect

# Summary for the last N days
ats
ats --days 7
ats --today                  # shortcut for today
ats --week                   # shortcut for --days 7
ats --month                  # shortcut for --days 30
ats --model gpt-4o-mini
ats --lang en
ats --verbose                # display raw data before the summary

# Filter logs by keyword before summarizing
ats --search docker           # filters entries containing "docker" and summarizes
ats --search "cargo build"

# Summary for a specific date (YYYY-DD-MM format)
ats --date 2026-06-06

# Override provider for a single session (without changing config)
ats --provider anthropic --api-key sk-ant-... --model claude-haiku-4-5-20251001
ats --provider groq --model llama-3.1-8b-instant

# Send summary to Notion
ats --today --send-notion

# Send summary to Slack
ats --today --send-slack

# Send to both at once
ats --week --send-notion --send-slack

# Interactive TUI
at tui                       # opens the interface, today as initial date
at tui --provider openai --model gpt-4o-mini

# Manual annotations
at tag "planning meeting"        # adds a note to today's log
at tag "code review"
at tag                           # lists today's notes
at tag --list --date 2026-10-06  # lists notes for a specific date
at tag "sprint planning" --date 2026-10-06  # note on a specific date

# Export logs as CSV or JSON
at export                    # today as CSV (stdout)
at export --days 7           # last 7 days
at export --format json      # JSON format
at export --days 7 -o week.csv   # save to file
at export --date 2026-06-10  # specific date

# Persistent configuration
at config set-provider ollama       # ollama | openai | anthropic | groq | gemini | openrouter
at config set-model llama3.1
at config set-api-key <key>         # API key for cloud providers
at config set-url http://localhost:11434  # Ollama URL (provider=ollama)
at config set-lang en               # pt-br | en | es
at config set-notion-token secret_xxx
at config set-notion-page <page_id>
at config set-slack-webhook https://hooks.slack.com/services/...
at config set-machine-name "MacBook Pro"  # optional, uses hostname if omitted
at config set-prompt "You are a product manager. {lang}\n\n{context}\n\nWrite a bullet-point report."
at config clear-prompt              # restore default prompt
at config show
```

## Interactive TUI

`at tui` opens a full terminal interface:

```
┌──────────────────── ◄ 2026-06-12 (today) ► ────────────────────┐
│  Activities  │  Summary  │  Projects  │  Config                 │
├────────────────────────────────────────────────────────────────┤
│ ■ SHELL  (42 commands)                                         │
│ ───────────────────────────────────────────────────────────    │
│   $ cargo build --release                                      │
│   $ git commit -m "feat: interactive tui"                      │
│   $ vim src/tui.rs                                             │
│   ...                                                          │
│                                                                │
│ ■ GIT  (2 repos, 5 commits)                                    │
│ ───────────────────────────────────────────────────────────    │
│   activity-tracker                                             │
│     ↳ 2026-06-12 feat: interactive tui                        │
├────────────────────────────────────────────────────────────────┤
│ ←/→ day  Tab tab  ↑↓ scroll  r summary  q quit                │
└────────────────────────────────────────────────────────────────┘
```

The **Projects** tab shows time distribution per repository:

```
  activity-tracker  ████████████████░░░░░░░░   67.3%  (12c, 5d)
  my-project        ████████░░░░░░░░░░░░░░░░   32.7%  ( 6c, 3d)
```

The **Config** tab lets you edit all settings without leaving the TUI. Provider and language fields cycle with `←`/`→`; other fields open a text-edit mode. Privacy patterns can be added and removed directly.

| Key | Action |
|---|---|
| `←` / `→` or `h` / `l` | Navigate between days |
| `Tab` / `1` / `2` / `3` / `4` | Switch between Activities, Summary, Projects, and Config tabs |
| `↑` / `↓` or `j` / `k` | Scroll content / navigate fields (Config) |
| `PgUp` / `PgDn` | Fast scroll |
| `Home` | Go to top |
| `r` | Generate LLM summary (switches to Summary tab) |
| `s` | In Projects tab: 7-day window |
| `m` | In Projects tab: 30-day window |
| `Enter` / `e` | In Config tab: edit selected field |
| `←` / `→` | In Config tab: cycle provider or language |
| `d` / `Delete` | In Config tab: remove privacy pattern |
| `R` | In Config tab: reload config from disk |
| `q` / `Esc` | Quit (or cancel edit in Config tab) |

Accepts the same provider flags as `summary` (`--provider`, `--model`, `--api-key`, `--lang`).

## Autostart on Login

### macOS

`install.sh` creates and loads a LaunchAgent automatically:

```bash
launchctl list | grep activity-tracker     # status
launchctl unload ~/Library/LaunchAgents/com.activity-tracker.plist  # stop
launchctl load -w ~/Library/LaunchAgents/com.activity-tracker.plist # start

tail -f ~/.local/share/activity-tracker/daemon.log  # logs
```

### Linux (systemd)

```bash
systemctl --user status activity-tracker   # status
systemctl --user stop activity-tracker     # stop
systemctl --user start activity-tracker    # start
systemctl --user disable activity-tracker  # remove from autostart

journalctl --user -u activity-tracker -f   # live logs
```

> For autostart to work after reboot without an active graphical session (Linux):
> ```bash
> loginctl enable-linger $USER
> ```

## macOS Permissions

`install.sh` opens the permission screens automatically. If you need to configure manually, add the binary (`~/.local/bin/activity-tracker`) to:

- **System Settings → Privacy & Security → Full Disk Access** — to read Chrome/Brave history
- **System Settings → Privacy & Security → Accessibility** — to capture window titles via `osascript`

## Notion Integration

Create an [Internal Integration](https://www.notion.so/my-integrations) in Notion, share a page with it, and configure:

```bash
at config set-notion-token secret_xxx
at config set-notion-page <page_id>   # ID after the last / in the page URL
```

From then on, any summary can be sent with `--send-notion`:

```bash
ats --today --send-notion
```

The summary is created as a sub-page of the configured page. The title uses the date and machine name, e.g. `2026-06-12 — MacBook Pro`.

## Slack Integration

Create an [Incoming Webhook](https://api.slack.com/messaging/webhooks) in Slack (at **api.slack.com/apps → your app → Incoming Webhooks**) and configure:

```bash
at config set-slack-webhook https://hooks.slack.com/services/T.../B.../...
```

From then on, any summary can be sent with `--send-slack`:

```bash
ats --today --send-slack
ats --week --send-slack
```

The summary is sent as a formatted Block Kit message (header + sections). Works with any channel the webhook has access to. Can be combined with `--send-notion`:

```bash
ats --today --send-notion --send-slack
```

## What Gets Captured

| Source | Details |
|---|---|
| **Shell history** | Bash, Zsh and Fish — incremental reads (only new commands since the last collection) |
| **Open windows** | `wmctrl` (X11) → `xdotool` → `osascript` (macOS) → `ps` (Linux fallback) |
| **Chrome / Brave** | DevTools Protocol (port 9222) or SQLite history DB (last 2 hours) |
| **Git** | All commits for the day per repo, found up to 4 levels below home (max 15 repos) |

Trivial commands are filtered automatically (`ls`, `cd`, `clear`, `pwd`, etc.).

## Storage

| File | Path |
|---|---|
| Daily logs (JSONL) | `~/.local/share/activity-tracker/logs/YYYY-MM-DD.jsonl` |
| Daemon PID | `~/.local/share/activity-tracker/daemon.pid` |
| Configuration | `~/.config/activity-tracker/config.toml` |

Each line in the `.jsonl` file is a typed entry (`shell`, `apps`, `chrome_tabs`, or `context`).

## Default Configuration

```toml
provider   = "ollama"
model      = "llama3.1"
ollama_url = "http://localhost:11434"
lang       = "pt-br"
# api_key         = "sk-..."                              # optional, for cloud providers
# notion_token    = "secret_xxx"                          # optional
# notion_page_id  = "abc123..."                           # optional
# slack_webhook   = "https://hooks.slack.com/services/..."  # optional
# machine_name    = "MacBook Pro"                         # optional, defaults to system hostname
```

## Summary Structure

1. **General Summary** — what was done, in 2-3 paragraphs
2. **Identified Projects** — detected ongoing tasks
3. **Most Used Tools** — most frequent apps and CLIs
4. **Sites & Research** — what was read or searched online
5. **Suggestions** — productivity observations

## Chrome with DevTools Protocol

To capture tabs in real time (without relying on the SQLite history), launch Chrome with:

```bash
google-chrome --remote-debugging-port=9222
```
