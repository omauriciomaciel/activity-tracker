---
type: Service
title: Slack Integration
description: Sends a markdown summary to a Slack channel via an incoming webhook, formatted as Block Kit with a header and mrkdwn sections.
tags: [slack, integration, webhook, block-kit, mrkdwn]
status: stable
---

# Slack Integration

**Module**: `src/slack.rs` (~58 lines)

Posts an LLM-generated summary to a Slack channel using a configured incoming webhook,
formatted with Block Kit.

## Entry Point

```rust
pub async fn send_message(webhook_url: &str, title: &str, body: &str) -> Result<()>
```

## API Call

- **Endpoint**: the user's incoming webhook URL (`https://hooks.slack.com/services/...`)
- **Method**: `POST`
- **Body**: `{ "blocks": [ header, section, section, ... ] }`

The first block is a `header` (plain_text, emoji disabled) with the title. The body is then
split into `section` blocks of `mrkdwn` text, each capped at `BLOCK_CHAR_LIMIT` (2900 chars)
with `char_boundary_at` ensuring cuts land on UTF-8 char boundaries.

## Markdown Conversion

`md_to_mrkdwn` converts Slack's format: `**bold**` -> `*bold*` (Slack mrkdwn uses single
asterisks for bold). Other markdown is passed through.

## Configuration

Requires `slack_webhook` set in [Config](../data/config.md):

```bash
at config set-slack-webhook https://hooks.slack.com/services/T.../B.../...
```

Triggered by `at summary --send-slack` or the TUI Summary tab `s` key. Can be combined with
`--send-notion` to publish to both. The message title is `{date label} - {machine name}`.

## Related

- Invoked by [Summarizer Service](summarizer.md) `run()` and [TUI Service](tui.md)
- Webhook URL stored in [Config](../data/config.md)
- Sibling integration: [Notion Integration](notion.md)
