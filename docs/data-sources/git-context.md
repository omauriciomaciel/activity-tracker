---
type: Data Source
title: Git Context Capture
description: Discovers git repositories up to 4 levels below $HOME and collects each repo's commits for the current day, with symlink-escape and ignored-path guards.
tags: [git, commits, repos, discovery, data-source]
status: stable
---

# Git Context Capture

**Source of**: `context` [Log Entry](../data/log-entry.md) records
**Implemented in**: `capture_git_context` ([Collector Service](../services/collector.md))

Discovers git repositories under the user's home directory and records each repo's commits
made since 00:00 of the current day. Produces a single `context` entry per collection run.

## Repository Discovery

```sh
find -P "$HOME" -maxdepth 4 -name .git -type d
```

- `-P` never follows symlinks.
- `-maxdepth 4` limits traversal depth.
- Only directories named `.git` are matched.

## Safety Guards

- **Symlink escape guard**: any discovered repo path that does not start with `$HOME` is
  rejected.
- **Ignored paths**: repo paths starting with any prefix in [Config](../data/config.md)
  `ignored_git_paths` are skipped (e.g. personal or client folders).
- **Cap**: stops after 15 repos with commits.

## Commit Collection

For each repo:

```sh
git -C "$repo" log --since="YYYY-MM-DD 00:00:00" --format="%ai %s"
```

Records the author date and subject for every commit today. Repos with no commits today are
not included. Commits are stored as strings like `2026-08-22 14:05:12 +0000 feat: add TUI`.

## Output

```json
{ "type": "context", "ts": "2026-08-22T14:30:00",
  "data": { "git_repos": [
    { "repo": "/home/user/activity-tracker", "commits": ["2026-08-22 14:05:12 +0000 feat: add TUI"] }
  ] } }
```

See [ContextData / GitRepoInfo](../data/log-entry.md).

## Log Purging

Repos matching `ignored_git_paths` can be retroactively removed from existing logs via
`purge_ignored_git_repos` (run by `clean-logs` and the TUI Config `P` key). Stale repos
without today's commits are pruned by `clean_log_file`.

## Related

- Part of the [Collection Pipeline](../pipeline/collection.md)
- Repo stats computed by [Projects Service](../services/projects.md)
- ContextData structure in [Log Entry](../data/log-entry.md)
