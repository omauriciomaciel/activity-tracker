---
type: Data Model
title: Log Entry
description: The tagged-union JSONL record type stored one-per-line in daily log files, covering shell, apps, chrome_tabs, context, and tag entries.
tags: [log-entry, jsonl, serde, tagged-union, storage]
status: stable
---

# Log Entry

**Defined in**: `src/collector.rs` (write side), `src/summarizer/mod.rs` (read side)

Each line of a daily log file (`~/.local/share/activity-tracker/logs/YYYY-MM-DD.jsonl`) is a
JSON object with a `type` discriminator (serde `tag = "type"`). Log files are created with
mode `0o600` on Unix.

## Variants

### `shell`

```json
{ "type": "shell", "ts": "2026-08-22T14:30:00", "commands": ["cargo build", "git status"] }
```

Produced by [Shell History Capture](../data-sources/shell-history.md). `commands` is the
filtered, deduped-at-write set of new commands since the last collection.

### `apps`

```json
{ "type": "apps", "ts": "2026-08-22T14:30:00", "windows": ["Code", "Firefox"] }
```

Produced by [Open Windows Capture](../data-sources/open-windows.md). `windows` is the list of
captured window titles.

### `chrome_tabs`

```json
{ "type": "chrome_tabs", "ts": "2026-08-22T14:30:00",
  "tabs": [ { "title": "Rust docs", "url": "https://doc.rust-lang.org", "visited_at": "2026-08-22 14:25:01" } ] }
```

Produced by [Browser Tabs Capture](../data-sources/browser-tabs.md). `visited_at` is only
populated from the SQLite history path; the DevTools path leaves it `null` (skipped when
serializing via `skip_serializing_if = "Option::is_none"`).

### `context`

```json
{ "type": "context", "ts": "2026-08-22T14:30:00",
  "data": { "git_repos": [
    { "repo": "/home/user/activity-tracker", "commits": ["2026-08-22 14:05:12 +0000 feat: x"] }
  ] } }
```

Produced by [Git Context Capture](../data-sources/git-context.md). The read side (`GitEntry`)
also tolerates a legacy `last_commit: String` field (defaults to empty) for backward
compatibility.

### `tag`

```json
{ "type": "tag", "ts": "2026-08-22T10:00:00", "label": "reunião de planning" }
```

Written by `collector::write_tag` via the `at tag` command. Manual user annotations.

## Sub-structures

### `TabInfo` (write) / `TabEntry` (read)

| Field | Type | Notes |
|---|---|---|
| `title` | String | defaults to empty on read |
| `url` | String | defaults to empty on read |
| `visited_at` | Option<String> | skipped when `None` on write |

### `ContextData` / `CtxData`

| Field | Type |
|---|---|
| `git_repos` | Vec<GitRepoInfo / GitEntry> |

### `GitRepoInfo` (write) / `GitEntry` (read)

| Field | Type | Notes |
|---|---|---|
| `repo` | String | full path |
| `commits` | Vec<String> | `"%ai %s"` git log lines |
| `last_commit` | String | legacy, defaults empty (read only) |

## Parsing

`read_log_entries(path)` (in the summarizer) reads the file line-by-line, skips empty lines,
and deserializes each into `LogEntry` via serde, silently dropping unparseable lines. Used by
the aggregator, exporter, and projects module.

## Related

- Written by [Collector Service](../services/collector.md)
- Read into [Activity Data](activity-data.md) by [Summarizer Service](../services/summarizer.md)
- Storage details in [Storage Layout](../infrastructure/storage-layout.md)
