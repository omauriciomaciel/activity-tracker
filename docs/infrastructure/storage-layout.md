---
type: Infrastructure
title: Storage Layout
description: On-disk file layout for configuration, daily JSONL logs, cached summaries, the daemon PID file, and shell-history read markers, all under user-specific XDG-style directories.
tags: [storage, filesystem, xdg, logs, config, paths]
status: stable
---

# Storage Layout

Activity Tracker stores all state under user-specific directories provided by the `dirs`
crate (`config_dir` and `data_local_dir`), following XDG conventions on Linux and the
standard Library locations on macOS.

## Directory Map

```
~/.config/activity-tracker/                  (base_dir / config_dir)
└── config.toml                              configuration (see Config)

~/.local/share/activity-tracker/             (data_dir / data_local_dir)
├── daemon.pid                               daemon PID file (Daemon Service)
├── daemon.log                               launchd stdout/stderr (macOS)
├── logs/                                    (log_dir)
│   ├── YYYY-MM-DD.jsonl                     daily activity log (Log Entry)
│   ├── .last_bash_pos                       bash history read marker
│   ├── .last_zsh_pos                        zsh history read marker
│   └── .last_fish_pos                       fish history read marker
├── summaries/                               (summary_dir)
│   └── YYYY-MM-DD.md                        cached LLM summary (markdown)
└── chrome_tmp_{pid}.db                      transient Chrome DB copy (cleaned up)
```

## Path Helpers (`src/config.rs`)

| Function | Linux path | macOS path |
|---|---|---|
| `base_dir()` | `~/.config/activity-tracker` | `~/Library/Application Support/activity-tracker`* |
| `config_path()` | `<base_dir>/config.toml` | `<base_dir>/config.toml` |
| `data_dir()` | `~/.local/share/activity-tracker` | `~/Library/Application Support/activity-tracker`* |
| `log_dir()` | `<data_dir>/logs` | `<data_dir>/logs` |
| `summary_dir()` | `<data_dir>/summaries` | `<data_dir>/summaries` |
| `summary_path(date)` | `<summary_dir>/YYYY-MM-DD.md` | `<summary_dir>/YYYY-MM-DD.md` |

\* On macOS, `dirs::config_dir` and `dirs::data_local_dir` both resolve to
`~/Library/Application Support`, so `base_dir` and `data_dir` coincide. On Linux they are
distinct (`~/.config` vs `~/.local/share`).

## Permissions

All sensitive files are created with mode `0o600` on Unix:

- Daily `.jsonl` log files
- Shell-history read markers (`.last_*_pos`)
- Transient Chrome DB copies (`chrome_tmp_{pid}.db`)
- The installed binary itself is `0o755`

## File Formats

| File | Format | Doc |
|---|---|---|
| `config.toml` | TOML | [Config](../data/config.md) |
| `YYYY-MM-DD.jsonl` | JSON Lines (one [Log Entry](../data/log-entry.md) per line) | [Log Entry](../data/log-entry.md) |
| `YYYY-MM-DD.md` | Markdown | [Summary Generation Pipeline](../pipeline/summary-generation.md) |
| `daemon.pid` | plain integer | [Daemon Service](../services/daemon.md) |
| `.last_*_pos` | plain integer (byte offset) | [Shell History Capture](../data-sources/shell-history.md) |

## Related

- Paths defined in `src/config.rs` ([Config Data Model](../data/config.md))
- Logs written by [Collection Pipeline](../pipeline/collection.md)
- Summaries cached by [Summary Generation Pipeline](../pipeline/summary-generation.md)
- Autostart wired by [Installer](installer.md)
