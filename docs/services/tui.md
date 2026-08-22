---
type: Service
title: TUI Service
description: Interactive ratatui terminal interface with four tabs (Activities, Summary, Projects, Config), day navigation, async summary generation, and live config editing with i18n.
tags: [tui, ratatui, crossterm, interactive, i18n, config-editing]
status: stable
---

# TUI Service

**Module**: `src/tui/` (mod.rs, render.rs, render_config.rs, config.rs, edit.rs, i18n.rs, locales/*.json - ~2097 lines total)

A full-screen terminal UI built on `ratatui` + `crossterm`, launched via `at tui`. It lets
the user browse days, view raw activity, generate/cache summaries, inspect project time
distribution, and edit all configuration without leaving the terminal.

## Entry Point

```rust
pub struct TuiOptions { model, provider, ollama_url, api_key, lang }
pub async fn run(opts: TuiOptions) -> Result<()>
```

`run` enables raw mode, enters the alternate screen, enables mouse capture, builds an `App`,
runs the event loop, then restores the terminal on exit.

## Sub-modules

| Sub-module | Responsibility |
|---|---|
| `mod.rs` | `App` state, event loop, key dispatch, async summary/send spawning |
| `render.rs` | Top-level layout, header (date + tabs), Activities/Summary/Projects content, footer |
| `render_config.rs` | Config tab rendering (fields, blocked patterns, ignored git paths) |
| `config.rs` | Config field cycling, initial values, applying edits to `Config` |
| `edit.rs` | Multi-line text editing primitives (cursor movement, insert, delete) |
| `i18n.rs` | Locale loader with `t()` / `tf()` translation lookups |
| `locales/*.json` | Translation tables for pt-br, en, es, fr, de, ja, zh |

## Tabs

| Tab | Key | Content |
|---|---|---|
| **Activities** | `1` | Raw shell commands, git commits grouped by repo (grouped/deduped tabs) |
| **Summary** | `2` | Cached or generated LLM summary rendered as markdown (`termimad`); press `n` to send to Notion, `s` to send to Slack |
| **Projects** | `3` | Per-repo time distribution bar chart (commits %, days active); `s`=7d, `m`=30d |
| **Config** | `4` | Editable config fields; cycle provider/lang with arrows, edit text fields with Enter |

## App State

`App` holds: current `date`, loaded `ActivityData`, `SummaryState` (Empty/Loading/Cached/Done/Error),
active tab, scroll position, provider/model/url/api_key/lang overrides, `Config`, config cursor
+ edit mode, project stats + window, and a `SendState` for Notion/Slack dispatch.

## Event Loop

A 100ms poll loop that:

1. Draws the frame via `render::render`.
2. Drains async results from two `mpsc` channels:
   - `rx` - LLM summary results (saved + shown on the Summary tab)
   - `send_rx` - Notion/Slack send results
3. Handles `Event::Key` and `Event::Mouse`.

### Summary generation (async)

Pressing `r` (when activity exists) spawns a `tokio` task calling
`summarizer::call_llm` with the current day's context; the result is sent back over the
channel, cached via `summarizer::save_summary`, and the tab switches to Summary.

### Send to Notion / Slack (async)

On the Summary tab, `n` / `s` spawn tasks calling [Notion Integration](notion.md)
/ [Slack Integration](slack.md) with the cached summary text and a
`"YYYY-MM-DD - {machine}"` title. Missing credentials surface a localized error.

## Config Editing

The Config tab maps rows to fixed indices (`CF_PROVIDER` ... `CF_SLACK`, then blocked
patterns, then ignored git paths). In **browse mode**: `j`/`k` move, arrows cycle
provider/lang, `Enter`/`e` enters **editing mode** for text fields, `d`/`Delete` removes a
block pattern or ignored path, `P` purges ignored git repos from logs, `R` reloads config
from disk. In **editing mode**: full cursor editing (insert, backspace, delete, up/down line
navigation, Home/End, Ctrl-Home/End for buffer start/end). On `Enter` the edit is applied via
`config::cfg_apply` and `Config::save()` is called.

## i18n

`Ui::new(lang)` embeds the matching `locales/*.json` via `include_str!` (compiled in, no
runtime file reads). `t(key)` returns a static string; `tf(key, args)` substitutes
`{placeholder}` tokens. `tab_names()` returns the four localized tab labels. Supported
languages: pt-br (default), en, es, fr, de, ja, zh.

## Mouse Support

`EnableMouseCapture` is on; `render::handle_mouse` translates mouse events to actions
(scrolling, tab/day navigation).

## Related

- Reads activity via [Summarizer Service](summarizer.md) `load_for_date` / `load_summary`
- Generates summaries via `summarizer::call_llm` ([LLM Provider APIs](../api/llm-providers.md))
- Sends to [Notion Integration](notion.md) and [Slack Integration](slack.md)
- Edits [Config](../data/config.md) and purges logs via [Collector Service](collector.md)
- Shows [Projects Service](projects.md) stats
