---
type: Pipeline
title: Export Pipeline
description: Reads daily log files and exports raw activity rows (shell, app, tab, git, tag) to CSV or JSON, to stdout or a file.
tags: [pipeline, export, csv, json]
status: stable
---

# Export Pipeline

Triggered by `at export`. Implemented in `summarizer::export_cmd` / `export_raw`
([Summarizer Service](../services/summarizer.md)).

## Steps

1. Resolve target files: a specific `--date` or the last `--days` (default 1) log files
   (reuses `find_log_files`).
2. For each file, read entries via `read_log_entries` and flatten into rows of
   `(date, type, content)`:
   - `shell` -> `("date", "shell", "<command>")`
   - `apps` -> `("date", "app", "<window title>")`
   - `chrome_tabs` -> `("date", "tab", "<title> | <url>")` (or just URL if title empty/equals URL)
   - `context` -> `("date", "git", "<repo> | <commit>")`
   - `tag` -> `("date", "tag", "<label>")`
3. Dedupe within each file via a `HashSet` keyed by `"{type}:{content}"`.
4. Apply noise/hostname filtering (`is_noise_command`, `strip_hostname_prefix`).
5. Serialize:
   - **csv** (default): header `date,type,content`, content CSV-quoted.
   - **json**: array of `{"date","type","content"}`, pretty-printed.
6. Output to stdout, or to `-o <file>` if provided.

## CLI Flags

| Flag | Default | Purpose |
|---|---|---|
| `--days N` | `1` | Days back to export |
| `--date YYYY-MM-DD` | - | Export a single date |
| `--format csv\|json` | `csv` | Output format |
| `-o <path>` | stdout | Write to file |

## Example

```bash
at export                       # today as CSV to stdout
at export --days 7              # last 7 days as CSV
at export --format json         # today as JSON
at export --days 7 -o week.csv  # last 7 days to file
```

## Related

- Shares log reading with [Summarizer Service](../services/summarizer.md)
- Reads [Log Entry](../data/log-entry.md) records from the [Collection Pipeline](collection.md)
- Exposed via [CLI API](../api/cli.md) `export`
