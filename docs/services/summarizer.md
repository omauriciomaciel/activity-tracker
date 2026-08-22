---
type: Service
title: Summarizer Service
description: Aggregates daily JSONL logs into activity data, builds an LLM context string, calls the configured provider, renders the markdown summary, persists it, and optionally dispatches to Notion/Slack.
tags: [summarizer, aggregate, llm, context, export, markdown]
status: stable
---

# Summarizer Service

**Module**: `src/summarizer/` (mod.rs, aggregate.rs, llm.rs, export.rs - ~1231 lines total)

The summarizer is the read/analysis side of Activity Tracker. It loads log files, deduplicates
and condenses activity, builds a textual context for the LLM, requests a summary, renders it,
caches it to disk, and optionally forwards it to integrations.

## Public API

```rust
pub struct RunOptions<'a> { days, date, model, provider, ollama_url, api_key, lang,
                            machine_name, notion, slack, search, custom_prompt }
pub async fn run(opts: RunOptions<'_>) -> Result<()>
pub fn load_for_date(date: NaiveDate) -> Result<ActivityData>
pub fn save_summary(date: NaiveDate, text: &str)
pub fn load_summary(date: NaiveDate) -> Option<String>
pub fn export_cmd(days, date, format, output) -> Result<()>
pub const DEFAULT_PROMPT_TEMPLATE: &str
pub async fn call_llm(provider, ollama_url, api_key, model, context, lang, custom_prompt) -> Result<String>
```

## Sub-modules

| Sub-module | Responsibility |
|---|---|
| `aggregate.rs` | Parse JSONL into [Activity Data](../data/activity-data.md), dedupe, condense, search-filter, build context text |
| `llm.rs` | Prompt construction + dispatch to all six [LLM Provider APIs](../api/llm-providers.md) |
| `export.rs` | Export raw log rows to CSV or JSON |
| `mod.rs` | Shared parsing types ([Log Entry](../data/log-entry.md)), `run()` orchestration, summary caching |

## Main Flow (`run`)

1. Resolve target files: a specific `--date` or the last `days` log files (`find_log_files`).
2. `aggregate(files)` -> `ActivityData` (dedupes commands/URLs, counts apps, merges repos).
3. `condense_for_period(data, days)` - for >6 days, trims commands to 60-100, apps to 10, tabs to 20.
4. If `--search` is set: `filter_by_search` + print matches; abort if empty.
5. For multi-day ranges: load [Projects Service](projects.md) stats and append a
   project distribution section to the context.
6. `build_context(data)` -> human-readable text (NOTAS, COMANDOS, APLICATIVOS, SITES, REPOS).
7. `call_llm(...)` -> markdown summary (300s timeout).
8. Render: print a project bar chart (if multi-day), then the summary via `termimad` `MadSkin`.
9. Cache: for single-day / specific-date runs, `save_summary` to `summaries/YYYY-MM-DD.md`.
10. Dispatch: if `--send-notion` / `--send-slack`, call [Notion](notion.md) /
    [Slack](slack.md) with title `"{date label} - {machine name}"`.

## Context Format

`build_context` produces sections in this order (each only if non-empty):

```
Período: <dates>

=== NOTAS E EVENTOS ===        # manual tags (hour - label)
=== COMANDOS DO TERMINAL ===   # deduped, secret-scrubbed, max 150
=== APLICATIVOS ABERTOS ===    # top 15 with [Nx] counts
=== SITES VISITADOS ===        # deduped by URL, title-grouped, max 30
=== REPOSITÓRIOS GIT ===       # repo name + commits (date-filtered)
```

## Prompt

`build_prompt` takes the template (custom or `DEFAULT_PROMPT_TEMPLATE`), converts literal
`\n` sequences to real newlines, injects `{lang}` (a per-language instruction string
supporting pt-br, en, es, fr, de, ja, zh, and arbitrary fallback), and injects `{context}`.
If `{context}` is absent from a custom template, data is appended at the end.

The default template asks for five sections: Resumo Geral, Projetos Identificados,
Ferramentas Mais Usadas, Sites e Pesquisas, Sugestões.

## Export

`export_cmd` -> `export_raw` iterates log files and emits rows of `(date, type, content)`
where type is one of `shell`, `app`, `tab`, `git`, `tag`. Output formats:

- **csv** (default): `date,type,content` with CSV-quoted content, to stdout or `-o` file.
- **json**: array of `{"date","type","content"}` objects, pretty-printed.

Noise commands and hostname prefixes are stripped during export. See the [Export Pipeline](../pipeline/export.md).

## Secret Scrubbing

`scrub_secrets(cmd)` redacts values following sensitive flags/keys. It detects:
- `key=value` pairs where the key contains a sensitive term -> `key=[REDACTED]`
- standalone flags (`--password`, `--token`, ...) -> next token becomes `[REDACTED]`

Sensitive terms: `password`, `passwd`, `secret`, `token`, `apikey`, `api_key`, `auth`,
`credential`, `private`, `pass`, `bearer`, `access_key`, `secret_key`, `private_key`.

## Shared Parsing

`read_log_entries(path)` reads a JSONL file and deserializes each non-empty line into a
[Log Entry](../data/log-entry.md) via serde, silently skipping unparseable lines. Used by the
aggregator, exporter, and projects module.

## Related

- Consumes output of [Collector Service](collector.md)
- Calls [LLM Provider APIs](../api/llm-providers.md) via `call_llm`
- Dispatches to [Notion](notion.md) and [Slack](slack.md)
- Loads [Projects Service](projects.md) stats for multi-day summaries
- Caches summaries per [Storage Layout](../infrastructure/storage-layout.md)
- Drives the [Summary Generation Pipeline](../pipeline/summary-generation.md)
