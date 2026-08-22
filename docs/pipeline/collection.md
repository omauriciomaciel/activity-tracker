---
type: Pipeline
title: Collection Pipeline
description: The daemon's periodic flow that captures shell/windows/browser/git activity, applies privacy filters, appends typed entries to the day's JSONL log, and prunes stale entries.
tags: [pipeline, collection, daemon, capture, jsonl]
status: stable
---

# Collection Pipeline

The recurring flow executed by the [Daemon Service](../services/daemon.md) every `interval`
minutes (default 5). Triggered manually by `at collect` or `at start`.

## Steps

```text
┌────────────────────────────────────────────────────────────────────┐
│ 1. Load Config (blocked_patterns, ignored_git_paths)               │
│    └─ daemon::do_collect reloads Config::load() every tick          │
├────────────────────────────────────────────────────────────────────┤
│ 2. collect_all(log_dir, blocked, ignored_git_paths)                │
│    ├─ create log_dir if missing                                    │
│    ├─ date = today, ts = now, log_file = logs/YYYY-MM-DD.jsonl     │
│    ├─ capture_shell_history  -> Entry::Shell     (or warn + skip)  │
│    ├─ capture_open_windows   -> Entry::Apps      (or warn + skip)  │
│    ├─ capture_chrome_tabs    -> Entry::ChromeTabs (or warn + skip) │
│    └─ capture_git_context    -> Entry::Context   (or warn + skip)  │
├────────────────────────────────────────────────────────────────────┤
│ 3. Append entries to log_file (one JSON per line)                  │
│    └─ chmod 0o600 on Unix                                           │
├────────────────────────────────────────────────────────────────────┤
│ 4. clean_all_logs(log_dir)                                         │
│    └─ remove shell/git entries whose date != file stem date        │
└────────────────────────────────────────────────────────────────────┘
```

## Concurrency

The daemon wraps `collect_all` in `tokio::task::spawn_blocking` because the collector uses
blocking I/O (`std::fs`, `std::process::Command`, `reqwest::blocking`). The foreground loop
remains responsive via `tokio::select!` with `ctrl_c`.

## Failure Modes

Each capture is independent and non-fatal. A failing source prints `Aviso:<source>: <err>`
to stderr and is simply omitted from that run's entries. The log file is still written with
whatever sources succeeded.

## Data Sources

| Source | Doc |
|---|---|
| Shell history | [Shell History Capture](../data-sources/shell-history.md) |
| Open windows | [Open Windows Capture](../data-sources/open-windows.md) |
| Browser tabs | [Browser Tabs Capture](../data-sources/browser-tabs.md) |
| Git context | [Git Context Capture](../data-sources/git-context.md) |

## Output

One append per run to `~/.local/share/activity-tracker/logs/YYYY-MM-DD.jsonl`, containing up
to four [Log Entry](../data/log-entry.md) lines. See [Storage Layout](../infrastructure/storage-layout.md).

## Related

- Orchestrated by [Daemon Service](../services/daemon.md)
- Implemented by [Collector Service](../services/collector.md)
- Consumed downstream by the [Summary Generation Pipeline](summary-generation.md)
  and [Export Pipeline](export.md)
