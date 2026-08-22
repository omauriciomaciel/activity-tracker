---
type: Pipeline
title: Summary Generation Pipeline
description: The end-to-end flow of loading log files, aggregating activity, building LLM context, calling the configured provider, rendering the markdown summary, caching it, and dispatching to Notion/Slack.
tags: [pipeline, summary, llm, aggregate, render, dispatch]
status: stable
---

# Summary Generation Pipeline

Triggered by `at summary` (alias `ats`) or the TUI `r` key. Orchestrated by
`summarizer::run` ([Summarizer Service](../services/summarizer.md)).

## Steps

```text
1. Resolve target
   ├─ --date YYYY-MM-DD -> single file            (specific date)
   └─ --days N (default 3) -> last N log files    (--today=1, --week=7, --month=30)

2. aggregate(files) -> ActivityData
   └─ dedupe commands, count apps, dedupe tabs, merge+date-filter repos, collect tags

3. condense_for_period(data, days)
   └─ trim volumes for long ranges (>6 days)

4. (optional) filter_by_search(data, query) + print matches
   └─ abort if everything empty

5. (multi-day only) projects::load_stats(days) -> Vec<ProjectStat>
   └─ append format_context() to the LLM context

6. build_context(data) -> human-readable text block

7. call_llm(provider, url, api_key, model, context, lang, custom_prompt)
   └─ dispatch to ollama | openai | anthropic | groq | gemini | openrouter
   └─ 300s timeout

8. Render to terminal
   ├─ project bar chart (if multi-day)
   └─ summary via termimad MadSkin (markdown)

9. Cache (single-day or specific-date only)
   └─ save_summary(date, text) -> summaries/YYYY-MM-DD.md

10. Dispatch (optional)
    ├─ --send-notion -> notion::send_page(token, page_id, title, summary)
    └─ --send-slack  -> slack::send_message(webhook, title, summary)
    └─ title = "{date label} - {machine name}"
```

## Title / Date Label

- Single date: `"YYYY-MM-DD - {machine}"`
- Range: `"{oldest} a {newest} - {machine}"`
- `machine` comes from [Config](../data/config.md) `get_machine_name()`.

## Shortcut Flags

| Flag | Effect |
|---|---|
| `--today` | `days=1`, clears `--date` |
| `--week` | `days=7`, clears `--date` |
| `--month` | `days=30`, clears `--date` |
| `--search <term>` | Filter before summarizing |
| `--provider/--model/--api-key/--lang/--ollama-url` | Per-session overrides (config untouched) |
| `--send-notion` / `--send-slack` | Post-generation dispatch |

## Credential Validation

Before running, if `--send-notion` is set but `notion_token`/`notion_page_id` are missing,
the CLI exits with a helpful setup message. Same for `--send-slack` without `slack_webhook`.

## Caching

Summaries are only cached for single-day or specific-date runs (not ranges), written to
`~/.local/share/activity-tracker/summaries/YYYY-MM-DD.md`. The TUI reads these via
`load_summary` to show a `Cached` state without re-calling the LLM.

## Related

- Orchestrated by [Summarizer Service](../services/summarizer.md)
- Reads [Log Entry](../data/log-entry.md) logs from the [Collection Pipeline](collection.md)
- Calls [LLM Provider APIs](../api/llm-providers.md)
- Dispatches to [Notion Integration](../services/notion.md) and [Slack Integration](../services/slack.md)
- Invoked from [CLI API](../api/cli.md) `summary` and [TUI Service](../services/tui.md)
