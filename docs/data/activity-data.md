---
type: Data Model
title: Activity Data
description: In-memory aggregated representation built from one or more daily log files, holding deduped commands, app counts, tabs, repos, tags, and the covered date range.
tags: [activity-data, aggregate, in-memory, dedupe]
status: stable
---

# Activity Data

**Defined in**: `src/summarizer/aggregate.rs`
**Built by**: `aggregate(files)` / `load_for_date(date)`

The in-memory result of parsing and aggregating one or more daily JSONL logs into a single
normalized structure ready for context building, search filtering, or export.

## Structure

```rust
pub struct ActivityData {
    pub dates: Vec<String>,              // "YYYY-MM-DD" per source file
    pub commands: Vec<String>,           // deduped shell commands (max 150)
    pub top_apps: Vec<(String, u32)>,    // window title -> count, top 15
    pub tabs: Vec<(String, String)>,     // (title, url), deduped by url, max 30
    pub repos: Vec<(String, Vec<String>)>, // (repo path, commits), date-filtered
    pub tags: Vec<(String, String)>,     // (HH:MM, label) sorted by time
}
```

## Construction (`aggregate`)

For each log file (oldest-to-newest), entries are processed newest-first within the file:

| Entry | Handling |
|---|---|
| `shell` | Trim, drop empty/noise (`is_noise_command`), dedupe via `HashSet`, append |
| `apps` | Strip hostname prefix, count occurrences in `HashMap` |
| `chrome_tabs` | Dedupe by URL via `HashSet`, push `(title, url)` |
| `context` | Merge commits per repo; drop commits whose date != the file's stem date |
| `tag` | Extract `HH:MM` from `ts[11..16]`, push `(hour, label)` |

Post-processing:
- `top_apps` sorted by count desc, truncated to 15.
- `tabs` truncated to 30.
- `repos` filtered to those with commits, sorted by latest commit desc.
- `tags` sorted by hour.
- `commands` truncated to 150.

## Condensing for Long Periods

`condense_for_period(data, days)` further trims when summarizing many days:
- `days > 20`: commands to 60
- `6 < days <= 20`: commands to 100
- `days > 6`: apps to 10, tabs to 20

## Search Filtering

`filter_by_search(data, query)` produces a new `ActivityData` keeping only items where the
query (lowercased) appears in the command, app name, tab title/url, repo path/commit, or tag
label. Repos are kept if the repo path matches or any commit matches.

## Context Serialization

`build_context(data)` turns the structure into the human-readable text block sent to the LLM
(see [Summarizer Service](../services/summarizer.md) for the section format).

## Related

- Built from [Log Entry](log-entry.md) records by [Summarizer Service](../services/summarizer.md)
- Loaded per-day by [TUI Service](../services/tui.md) (`load_for_date`)
- Enriched with [Project Stat](project-stat.md) for multi-day summaries
