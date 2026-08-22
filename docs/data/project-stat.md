---
type: Data Model
title: Project Stat
description: Per-repository statistics (unique commits, active days, percentage share) computed over a rolling day window for summary enrichment and the TUI Projects tab.
tags: [project-stat, statistics, commits, distribution]
status: stable
---

# Project Stat

**Defined in**: `src/projects.rs`
**Built by**: `load_stats(days)`

A per-repository summary of git activity over the last N days, used to show how time is
distributed across projects.

## Structure

```rust
pub struct ProjectStat {
    pub name: String,        // repo directory basename (final path component)
    pub _path: String,       // full repo path (stored, currently unused externally)
    pub commits: usize,      // count of unique commit strings
    pub days_active: usize,  // distinct days with at least one commit
    pub pct: f64,            // commits / total_commits * 100.0
}
```

## Computation

1. For each of the last `days` days, read that day's JSONL log and extract `context` entries.
2. For each repo, accumulate unique commit strings into a `HashSet` (falls back to
   `last_commit` when `commits` is empty) and increment `days_active` once per day per repo.
3. `total_commits` = sum of unique commits across all repos.
4. `pct = commit_count / total_commits * 100` (0.0 if no commits).
5. Sort by commit count desc, then name asc.

## Rendering

### In the terminal summary

The [Summarizer Service](../services/summarizer.md) prints a bar chart (for multi-day runs):

```
  activity-tracker  ████████████████░░░░░░░░   67.3%  (12c, 5d)
  meu-projeto       ████████░░░░░░░░░░░░░░░░   32.7%  ( 6c, 3d)
```

Bar width is `(pct/100)*20` blocks; shows commits (`c`) and active days (`d`).

### In the LLM context

`format_context(stats, days)` appends a `=== DISTRIBUIÇÃO DE PROJETOS ===` section (top 10
repos) to the LLM context for multi-day summaries.

### In the TUI

The [TUI Service](../services/tui.md) Projects tab renders the same stats; `s` switches to a
7-day window, `m` to 30 days.

## Related

- Computed by [Projects Service](../services/projects.md)
- Reads [Log Entry](log-entry.md) `context` records
- Consumed by [Summarizer Service](../services/summarizer.md) and [TUI Service](../services/tui.md)
