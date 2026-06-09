use anyhow::{Context, Result};
use serde_json::{json, Value};

const NOTION_VERSION: &str = "2026-03-11";
const MAX_SPAN_CHARS: usize = 1900;

pub async fn send_page(token: &str, parent_page_id: &str, title: &str, body: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let payload = json!({
        "parent": { "page_id": parent_page_id },
        "properties": {
            "title": {
                "title": [{ "type": "text", "text": { "content": title } }]
            }
        },
        "children": markdown_to_blocks(body),
    });

    let resp = client
        .post("https://api.notion.com/v1/pages")
        .header("Authorization", format!("Bearer {token}"))
        .header("Notion-Version", NOTION_VERSION)
        .json(&payload)
        .send()
        .await
        .context("Erro conectando ao Notion")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Notion API erro {status}: {body}");
    }

    let result: Value = resp.json().await.context("Erro parseando resposta do Notion")?;
    Ok(result["url"].as_str().unwrap_or("").to_string())
}

// \u2500\u2500\u2500 Markdown \u2192 Notion blocks \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

fn markdown_to_blocks(text: &str) -> Vec<Value> {
    let mut blocks: Vec<Value> = Vec::new();
    let mut para_lines: Vec<&str> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if let Some(b) = flush_paragraph(&para_lines) {
                blocks.push(b);
            }
            para_lines.clear();
            continue;
        }

        // Heading: line is entirely **text** (Ollama uses this for section titles)
        if let Some(inner) = try_heading(trimmed) {
            if let Some(b) = flush_paragraph(&para_lines) {
                blocks.push(b);
            }
            para_lines.clear();
            blocks.push(json!({
                "object": "block",
                "type": "heading_2",
                "heading_2": { "rich_text": [plain_span(inner)] }
            }));
            continue;
        }

        // Bullet item: * text / - text / *   text (Ollama uses "* " and "*   ")
        if let Some(rest) = try_bullet(trimmed) {
            if let Some(b) = flush_paragraph(&para_lines) {
                blocks.push(b);
            }
            para_lines.clear();
            blocks.push(json!({
                "object": "block",
                "type": "bulleted_list_item",
                "bulleted_list_item": { "rich_text": parse_inline(rest) }
            }));
            continue;
        }

        para_lines.push(trimmed);
    }

    if let Some(b) = flush_paragraph(&para_lines) {
        blocks.push(b);
    }

    blocks
}

fn try_heading(line: &str) -> Option<&str> {
    let inner = line.strip_prefix("**")?.strip_suffix("**")?;
    if inner.contains("**") {
        return None;
    }
    Some(inner)
}

fn try_bullet(line: &str) -> Option<&str> {
    for prefix in &["*   ", "*  ", "* ", "- ", "•  ", "•"] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(rest.trim_start());
        }
    }
    None
}

fn flush_paragraph(lines: &[&str]) -> Option<Value> {
    if lines.is_empty() {
        return None;
    }
    let content = lines.join(" ");
    let rt = parse_inline(content.trim());
    if rt.is_empty() {
        return None;
    }
    Some(json!({
        "object": "block",
        "type": "paragraph",
        "paragraph": { "rich_text": rt }
    }))
}

// \u2500\u2500\u2500 Inline markdown \u2192 rich_text spans \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

fn parse_inline(text: &str) -> Vec<Value> {
    let mut spans: Vec<Value> = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if let Some(open) = remaining.find("**") {
            let before = &remaining[..open];
            if !before.is_empty() {
                spans.extend(span_chunks(before, false, false));
            }
            let after_open = &remaining[open + 2..];
            if let Some(close) = after_open.find("**") {
                spans.extend(span_chunks(&after_open[..close], true, false));
                remaining = &after_open[close + 2..];
                continue;
            }
            spans.extend(span_chunks(remaining, false, false));
            break;
        }

        if let Some(open) = find_single_asterisk(remaining) {
            let before = &remaining[..open];
            if !before.is_empty() {
                spans.extend(span_chunks(before, false, false));
            }
            let after_open = &remaining[open + 1..];
            if let Some(close) = find_single_asterisk(after_open) {
                spans.extend(span_chunks(&after_open[..close], false, true));
                remaining = &after_open[close + 1..];
                continue;
            }
            spans.extend(span_chunks(remaining, false, false));
            break;
        }

        spans.extend(span_chunks(remaining, false, false));
        break;
    }

    spans
}

fn find_single_asterisk(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'*' {
            let prev = i > 0 && bytes[i - 1] == b'*';
            let next = i + 1 < bytes.len() && bytes[i + 1] == b'*';
            if !prev && !next {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn span_chunks(text: &str, bold: bool, italic: bool) -> Vec<Value> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return vec![];
    }
    chars
        .chunks(MAX_SPAN_CHARS)
        .map(|chunk| {
            let content: String = chunk.iter().collect();
            json!({
                "type": "text",
                "text": { "content": content },
                "annotations": {
                    "bold": bold,
                    "italic": italic,
                    "strikethrough": false,
                    "underline": false,
                    "code": false,
                    "color": "default"
                }
            })
        })
        .collect()
}

fn plain_span(text: &str) -> Value {
    json!({
        "type": "text",
        "text": { "content": text },
        "annotations": {
            "bold": false, "italic": false, "strikethrough": false,
            "underline": false, "code": false, "color": "default"
        }
    })
}
