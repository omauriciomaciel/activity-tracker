---
type: Service
title: Projects Service
description: Computes per-repository commit distribution and day-activity statistics over a rolling window, used for summary enrichment and the TUI Projects tab.
tags: [projects, statistics, commits, distribution, repos]
status: stable
---

# Projects Service

**Module**: `src/projects.rs` (~89 lines)

Computes how the user's git activity is distributed across repositories over a configurable
number of days. Used to enrich multi-day summaries and to render the TUI Projects tab.

## Data Structure

```rust
pub struct ProjectStat {
    pub name: String,       // repo directory basename
    pub _path: String,      // full repo path
    pub commits: usize,     // unique commit count
    pub days_active: usize, // distinct days with >=1 commit
    pub pct: f64,           // share of total commits (%)
}
```

See [Project Stat Data Model](../data/project-stat.md).

## API

```rust
pub fn load_stats(days: u32) -> Result<Vec<ProjectStat>>
pub fn format_context(stats: &[ProjectStat], days: u32) -> String
```

## How It Works

1. Iterates the last `days` days (today backwards).
2. For each day's JSONL log, reads `context` entries via `read_log_entries` (from the
   [Summarizer Service](summarizer.md)).
3. For each repo, accumulates unique commit strings into a `HashSet` and counts distinct
   active days. Falls back to `last_commit` when `commits` is empty.
4. Computes `pct = repo_commits / total_commits * 100`.
5. Sorts by commit count descending, then name.

## Context Formatting

`format_context` emits a `=== DISTRIBUIÇÃO DE PROJETOS (últimos N dias) ===` header followed
by up to 10 repos as `  {name}: {pct}% ({commits} commits, {days} dias ativos)`. This text is
appended to the LLM context for multi-day summaries.

## Usage

- Called by [Summarizer Service](summarizer.md) `run()` for multi-day (`days > 1`)
  summaries to enrich the LLM context and print a bar chart.
- Called by [TUI Service](tui.md) for the Projects tab (7-day default, `s`/`m` to
  switch to 7/30 days).

## Related

- Reads [Log Entry](../data/log-entry.md) `context` records via the summarizer
- Produces [Project Stat](../data/project-stat.md) records
- Consumed by [Summarizer Service](summarizer.md) and [TUI Service](tui.md)
