---
type: Service
title: Collector Service
description: Captures shell history, open windows, browser tabs, and git context, applying privacy filters, then appends typed entries to the day's JSONL log.
tags: [collector, capture, shell, windows, browser, git, jsonl, privacy]
status: stable
---

# Collector Service

**Module**: `src/collector.rs` (~924 lines)

The collector is the heart of the data-gathering side of Activity Tracker. On each run it
captures four independent activity streams, filters them for noise and privacy, and appends
them as typed JSON lines to the current day's log file.

## Entry Point

```rust
pub fn collect_all(log_dir: &Path, blocked: &[String], ignored_git_paths: &[String]) -> Result<usize>
```

Runs all four captures, writes entries to `log_dir/YYYY-MM-DD.jsonl`, sets the file mode to
`0o600` on Unix, then calls [clean_all_logs](#log-cleaning) to prune stale entries. Returns
the number of entries saved.

The daemon invokes this on every tick via `spawn_blocking` (see [Daemon Service](daemon.md)).

## Capture Sources

Each source produces one [Log Entry](../data/log-entry.md). Failures are non-fatal (printed to
stderr as warnings).

| Source | Function | Entry type | Details |
|---|---|---|---|
| Shell history | `capture_shell_history` | `shell` | Bash, Zsh, Fish - incremental reads |
| Open windows | `capture_open_windows` | `apps` | Multi-platform fallback chain |
| Chrome/Brave tabs | `capture_chrome_tabs` | `chrome_tabs` | DevTools Protocol then SQLite |
| Git context | `capture_git_context` | `context` | Commits since 00:00 today |

See [Data Sources](../data-sources/index.md) for per-source deep dives.

## Privacy Filtering

`is_blocked(text, patterns)` performs a **case-insensitive substring** check against the
`blocked_patterns` list from [Config](../data/config.md). Any command, window title, tab title,
or URL containing a blocked term is dropped before it is ever written to disk.

Additionally, `scrub_secrets` (in the summarizer) redacts sensitive-looking tokens
(`password`, `secret`, `token`, `apikey`, `bearer`, `private_key`, ...) from commands at
summary time, replacing following values with `[REDACTED]`.

## Noise Filtering

Trivial shell commands are filtered out: `ls`, `cd`, `clear`, `pwd`, `exit`, `history`,
`ll`, `la`, `l`. Pure-numeric `#timestamp` lines (Bash HISTTIMEFORMAT markers) are also
stripped. See `is_noise_command` in the summarizer.

## Incremental Shell Reads

`read_incremental(file, marker, max)` remembers the last byte offset read in a marker file
(`.last_bash_pos`, `.last_zsh_pos`, `.last_fish_pos`) under the log dir. On the first run it
reads only the last ~4 KB. Subsequent runs read only newly appended bytes, capped at `max`
lines. Markers are also set to mode `0o600`.

## Log Cleaning

- `clean_all_logs(log_dir)` - reprocesses every `.jsonl` file, removing entries whose date
  does not match the file's stem date (shell commands and git commits are date-filtered).
- `purge_ignored_git_repos(log_dir, ignored_git_paths)` - removes git repo entries whose
  path starts with any ignored prefix.
- `clean_log_file(path, date)` - per-file implementation for shell/context entries.

These run automatically at the end of every `collect_all` and are also exposed via the
`clean-logs` CLI command.

## Manual Tags

Users can annotate a day with free-text notes (e.g. "reunião de planning"):

- `write_tag(log_dir, label, date_opt)` - appends a `tag` entry (ISO timestamp + label).
- `delete_tag(log_dir, label, date_opt)` - removes matching `tag` entries by case-insensitive label.

Exposed via the `tag` CLI command. Tags flow into [Activity Data](../data/activity-data.md) at
summary time.

## Security Notes

- Git discovery rejects any repo path that does not start with `$HOME` (symlink escape guard)
  and uses `find -P` (never follow symlinks).
- Chrome's locked SQLite DB is copied to a private user dir (not `/tmp`) with mode `0o600`
  and opened read-only; the temp copy is always cleaned up.
- All log/marker files are created with `0o600` permissions on Unix.

## Related

- Driven by [Daemon Service](daemon.md)
- Output consumed by [Summarizer Service](summarizer.md)
- Writes [Log Entry](../data/log-entry.md) records to the [Storage Layout](../infrastructure/storage-layout.md)
