---
type: Data Source
title: Browser Tabs Capture
description: Captures open Chrome/Brave tabs via the DevTools Protocol (port 9222) or, as fallback, the last 2 hours of SQLite history.
tags: [chrome, brave, devtools-protocol, sqlite, browser, tabs, data-source]
status: stable
---

# Browser Tabs Capture

**Source of**: `chrome_tabs` [Log Entry](../data/log-entry.md) records
**Implemented in**: `capture_chrome_tabs` ([Collector Service](../services/collector.md))

Captures browser tabs from Chrome or Brave/Chromium. Tries a live DevTools Protocol query
first; if unavailable, falls back to reading the SQLite history database.

## Method 1: DevTools Protocol (preferred)

`fetch_devtools_tabs` issues a blocking HTTP GET to `http://localhost:9222/json/list`, parses
the JSON array, and keeps entries where `type == "page"`, mapping `title` and `url` into
[TabInfo](../data/log-entry.md).

Enable it by launching Chrome with remote debugging:

```bash
google-chrome --remote-debugging-port=9222
```

This captures **currently open** tabs in real time without touching disk history.

## Method 2: SQLite History DB (fallback)

`read_chrome_history_db` only runs if the DevTools query returned nothing. It checks a list
of candidate DB paths for Chrome, Chromium, and Brave on both Linux and macOS:

| Browser | Linux path | macOS path |
|---|---|---|
| Google Chrome | `~/.config/google-chrome/Default/History` | `~/Library/Application Support/Google/Chrome/Default/History` |
| Chromium | `~/.config/chromium/Default/History` | `~/Library/Application Support/Chromium/Default/History` |
| Brave | `~/.config/BraveSoftware/Brave-Browser/Default/History` | `~/Library/Application Support/BraveSoftware/Brave-Browser/Default/History` |

### Safe DB Access

Because Chrome locks its `History` SQLite file:

1. Copy the DB to a **private user dir** (`dirs::data_local_dir()/activity-tracker/`), not
   `/tmp`, named `chrome_tmp_{pid}.db`, with mode `0o600`.
2. Open it read-only (`SQLITE_OPEN_READ_ONLY`) via `rusqlite`.
3. Query the last **2 hours** of visits:

```sql
SELECT title, url,
       datetime(last_visit_time/1000000 - 11644473600, 'unixepoch', 'localtime')
FROM urls
WHERE last_visit_time > ((strftime('%s','now') + 11644473600) * 1000000 - 7200000000)
ORDER BY last_visit_time DESC
LIMIT 40
```

(Chrome stores `last_visit_time` as microseconds since 1601-01-01; the `11644473600` offset
converts to Unix epoch.)

4. Always remove the temp copy afterward (even on copy failure).

## Filtering

Both methods retain only tabs whose `url` and `title` pass the
[Config](../data/config.md) `blocked_patterns` check (case-insensitive substring).

## Output

```json
{ "type": "chrome_tabs", "ts": "2026-08-22T14:30:00",
  "tabs": [ { "title": "Rust docs", "url": "https://doc.rust-lang.org", "visited_at": "2026-08-22 14:25:01" } ] }
```

`TabInfo` has `title`, `url`, and optional `visited_at` (only populated from the SQLite path).

## Platform Permissions

- **macOS**: requires **Full Disk Access** to read the Chrome/Brave history DB (configured by
  the [Installer](../infrastructure/installer.md)).

## Related

- Part of the [Collection Pipeline](../pipeline/collection.md)
- TabInfo structure defined in [Log Entry](../data/log-entry.md)
- Aggregated/deduped by [Summarizer Service](../services/summarizer.md)
