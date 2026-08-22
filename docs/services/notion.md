---
type: Service
title: Notion Integration
description: Creates a Notion sub-page under a configured parent page from a markdown summary, converting headings, lists, code blocks, and inline formatting to Notion rich-text blocks.
tags: [notion, integration, markdown, rich-text, api-client]
status: stable
---

# Notion Integration

**Module**: `src/notion.rs` (~334 lines)

Sends an LLM-generated summary to Notion as a new sub-page under a user-configured parent
page. Converts markdown into Notion block objects with proper rich-text annotations.

## Entry Point

```rust
pub async fn send_page(token: &str, parent_page_id: &str, title: &str, body: &str) -> Result<String>
```

Returns the created page's URL on success.

## API Call

- **Endpoint**: `POST https://api.notion.com/v1/pages`
- **Headers**: `Authorization: Bearer {token}`, `Notion-Version: 2026-03-11`
- **Body**: `{ parent: { page_id }, properties: { title }, children: [blocks...] }`

On non-2xx, bails with `Notion API erro {status}: {body}`.

## Markdown to Notion Blocks

`markdown_to_blocks(text)` is a hand-rolled parser that converts common markdown into Notion
block objects. Supported block types:

| Markdown | Notion block |
|---|---|
| `#` / `##` / `###` headings | `heading_1` / `heading_2` / `heading_3` |
| `**text**` (whole-line bold) | `heading_2` (handles models that omit `##`) |
| `* `, `- `, `•` bullets | `bulleted_list_item` |
| Fenced ```` ``` ```` / `~~~` code | `code` (with language hint or "plain text") |
| Plain paragraphs (joined across blank lines) | `paragraph` |

## Inline Formatting

`parse_inline(text)` produces rich-text spans with `annotations`:

| Markdown | Annotation |
|---|---|
| `` `code` `` | `code: true` |
| `**bold**` | `bold: true` |
| `*italic*` | `italic: true` |

`find_single_asterisk` carefully skips `**` (bold) markers to find true single-asterisk
italic delimiters. `span_chunks` splits content at `MAX_SPAN_CHARS` (1900 chars) because
Notion limits rich-text span content length.

## Configuration

Requires both `notion_token` and `notion_page_id` set in [Config](../data/config.md) via:

```bash
at config set-notion-token secret_xxx
at config set-notion-page <page_id>
```

Triggered by `at summary --send-notion` or the TUI Summary tab `n` key. The page title is
`{date label} - {machine name}` (e.g. `2026-06-12 - MacBook Pro`).

## Related

- Invoked by [Summarizer Service](summarizer.md) `run()` and [TUI Service](tui.md)
- Credentials stored in [Config](../data/config.md)
- Sibling integration: [Slack Integration](slack.md)
