---
type: Data Source
title: Shell History Capture
description: Incrementally reads new commands from Bash, Zsh, and Fish history files, filters trivial and blocked commands, and date-filters by embedded timestamps.
tags: [shell, bash, zsh, fish, history, incremental, data-source]
status: stable
---

# Shell History Capture

**Source of**: `shell` [Log Entry](../data/log-entry.md) records
**Implemented in**: `capture_shell_history` ([Collector Service](../services/collector.md))

Reads shell history incrementally (only new lines since the last collection) from up to three
shells, filters out trivial and privacy-blocked commands, and produces a single `shell` entry
per collection run.

## Supported Shells

| Shell | History file | Marker | Format |
|---|---|---|---|
| Bash | `~/.bash_history` | `.last_bash_pos` | Optional `#timestamp` lines (HISTTIMEFORMAT) |
| Zsh | `~/.zsh_history` | `.last_zsh_pos` | `: <epoch>:0;<command>` |
| Fish | `~/.local/share/fish/fish_history` | `.last_fish_pos` | `- cmd: <command>` |

## Incremental Reading

`read_incremental(file, marker, max)`:

- Reads the file size and compares to the byte offset stored in the marker file.
- On the **first run** (no marker), reads only the last ~4 KB (`current_size - 4096`).
- On subsequent runs, seeks to `last_pos` and reads only the new bytes.
- Caps at `max` lines (400 for bash, 100 for zsh, 200 for fish).
- Updates the marker to the new file size (mode `0o600` on Unix).

## Date Filtering

Only commands from **today** are kept:

- **Bash**: `filter_by_date` walks `#timestamp` markers; if no timestamps are present in the
  batch, all (already-incremental) lines are kept. Skips pure-numeric `#` marker lines.
- **Zsh**: parses the `: <epoch>:0;...` prefix, converts epoch to a `NaiveDate`, and skips
  commands not from today. Lines without a parseable timestamp are kept.
- **Fish**: no per-command timestamps, so the incremental read itself guarantees recency.

## Filtering

For each command, the first whitespace-delimited token is checked against a trivial-command
blocklist (`ls`, `cd`, `clear`, `pwd`, `exit`, `history`, `ll`, `la`, `l`) and the whole
command is checked against [Config](../data/config.md) `blocked_patterns` (case-insensitive
substring). Blocked/trivial commands are dropped before persistence.

## Output

```json
{ "type": "shell", "ts": "2026-08-22T14:30:00", "commands": ["cargo build --release", "git status"] }
```

## Related

- Part of the [Collection Pipeline](../pipeline/collection.md)
- Privacy filtering shared with [Collector Service](../services/collector.md)
- Consumed by [Summarizer Service](../services/summarizer.md) (with `scrub_secrets` at summary time)
