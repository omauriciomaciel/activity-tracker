use anyhow::{Context, Result};
use serde_json::json;

const BLOCK_CHAR_LIMIT: usize = 2900;

pub async fn send_message(webhook_url: &str, title: &str, body: &str) -> Result<()> {
    let client = reqwest::Client::new();

    let text = md_to_mrkdwn(body);
    let mut blocks = vec![json!({
        "type": "header",
        "text": { "type": "plain_text", "text": title, "emoji": false }
    })];

    // Split body into ≤2900-char section blocks
    let mut remaining = text.as_str();
    while !remaining.is_empty() {
        let cut = char_boundary_at(remaining, BLOCK_CHAR_LIMIT);
        blocks.push(json!({
            "type": "section",
            "text": { "type": "mrkdwn", "text": &remaining[..cut] }
        }));
        remaining = &remaining[cut..];
    }

    let payload = json!({ "blocks": blocks });

    let resp = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .await
        .context("Erro conectando ao Slack")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Slack webhook erro {status}: {body}");
    }

    Ok(())
}

// Convert **bold** → *bold*  (Slack mrkdwn uses single asterisks)
fn md_to_mrkdwn(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            chars.next(); // consume second *
            out.push('*');
        } else {
            out.push(c);
        }
    }
    out
}

fn char_boundary_at(s: &str, max_bytes: usize) -> usize {
    if s.len() <= max_bytes {
        return s.len();
    }
    let mut idx = max_bytes;
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}
