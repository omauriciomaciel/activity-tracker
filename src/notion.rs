use anyhow::{Context, Result};
use serde_json::{Value, json};

const NOTION_VERSION: &str = "2026-03-11";
const MAX_SPAN_CHARS: usize = 1900;

pub async fn send_page(
    token: &str,
    parent_page_id: &str,
    title: &str,
    body: &str,
) -> Result<String> {
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

    let result: Value = resp
        .json()
        .await
        .context("Erro parseando resposta do Notion")?;
    Ok(result["url"].as_str().unwrap_or("").to_string())
}

// \u2500\u2500\u2500 Markdown \u2192 Notion blocks \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

fn markdown_to_blocks(text: &str) -> Vec<Value> {
    let mut blocks: Vec<Value> = Vec::new();
    let mut para_lines: Vec<&str> = Vec::new();
    let mut code_fence: Option<String> = None; // language hint when inside ```
    let mut code_lines: Vec<&str> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Inside a fenced code block — collect until closing ```
        if code_fence.is_some() {
            if trimmed == "```" || trimmed == "~~~" {
                let code_content = code_lines.join("\n");
                let lang = code_fence.take().unwrap_or_default();
                code_lines.clear();
                blocks.push(json!({
                    "object": "block",
                    "type": "code",
                    "code": {
                        "rich_text": [plain_span(&code_content)],
                        "language": if lang.is_empty() { "plain text" } else { &lang }
                    }
                }));
            } else {
                code_lines.push(line);
            }
            continue;
        }

        // Opening fence: ``` or ```lang or ~~~
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush_into(&mut blocks, &mut para_lines);
            let lang = trimmed.trim_start_matches('`').trim_start_matches('~').trim().to_string();
            code_fence = Some(lang);
            continue;
        }

        if trimmed.is_empty() {
            if let Some(b) = flush_paragraph(&para_lines) {
                blocks.push(b);
            }
            para_lines.clear();
            continue;
        }

        // ATX headings: # / ## / ###
        if let Some(inner) = trimmed.strip_prefix("### ") {
            flush_into(&mut blocks, &mut para_lines);
            blocks.push(json!({
                "object": "block",
                "type": "heading_3",
                "heading_3": { "rich_text": parse_inline(inner.trim()) }
            }));
            continue;
        }
        if let Some(inner) = trimmed.strip_prefix("## ") {
            flush_into(&mut blocks, &mut para_lines);
            blocks.push(json!({
                "object": "block",
                "type": "heading_2",
                "heading_2": { "rich_text": parse_inline(inner.trim()) }
            }));
            continue;
        }
        if let Some(inner) = trimmed.strip_prefix("# ") {
            flush_into(&mut blocks, &mut para_lines);
            blocks.push(json!({
                "object": "block",
                "type": "heading_1",
                "heading_1": { "rich_text": parse_inline(inner.trim()) }
            }));
            continue;
        }

        // Setext-style heading: line entirely **text** (Ollama / some models)
        if let Some(inner) = try_bold_heading(trimmed) {
            flush_into(&mut blocks, &mut para_lines);
            blocks.push(json!({
                "object": "block",
                "type": "heading_2",
                "heading_2": { "rich_text": [plain_span(inner)] }
            }));
            continue;
        }

        // Indented sub-bullet (2+ spaces before - or *)
        if let Some(rest) = try_indented_bullet(line) {
            flush_into(&mut blocks, &mut para_lines);
            blocks.push(json!({
                "object": "block",
                "type": "bulleted_list_item",
                "bulleted_list_item": {
                    "rich_text": parse_inline(rest),
                    "color": "default"
                }
            }));
            continue;
        }

        // Bullet item: * text / - text / *   text (Ollama uses "* " and "*   ")
        if let Some(rest) = try_bullet(trimmed) {
            flush_into(&mut blocks, &mut para_lines);
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

fn flush_into(blocks: &mut Vec<Value>, para_lines: &mut Vec<&str>) {
    if let Some(b) = flush_paragraph(para_lines) {
        blocks.push(b);
    }
    para_lines.clear();
}

// Entire line is **text** → treat as heading (some models omit ## prefix)
fn try_bold_heading(line: &str) -> Option<&str> {
    let inner = line.strip_prefix("**")?.strip_suffix("**")?;
    if inner.contains("**") {
        return None;
    }
    Some(inner)
}

fn try_indented_bullet(line: &str) -> Option<&str> {
    // Requires at least 2 leading spaces before - or *
    if line.len() < 3 {
        return None;
    }
    let stripped = line.trim_start();
    let indent = line.len() - stripped.len();
    if indent < 2 {
        return None;
    }
    for prefix in &["- ", "* "] {
        if let Some(rest) = stripped.strip_prefix(prefix) {
            return Some(rest.trim_start());
        }
    }
    None
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
        // Find the earliest marker among ` ** *
        let pos_backtick = remaining.find('`');
        let pos_double = remaining.find("**");
        let pos_single = find_single_asterisk(remaining);

        // Pick the earliest match
        let earliest = [
            pos_backtick.map(|p| (p, 0u8)),
            pos_double.map(|p| (p, 1u8)),
            pos_single.map(|p| (p, 2u8)),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(p, _)| *p);

        match earliest {
            None => {
                spans.extend(span_chunks(remaining, false, false));
                break;
            }
            Some((pos, 0)) => {
                // Inline code: `...`
                if pos > 0 {
                    spans.extend(span_chunks(&remaining[..pos], false, false));
                }
                let after = &remaining[pos + 1..];
                if let Some(close) = after.find('`') {
                    let code = &after[..close];
                    spans.push(json!({
                        "type": "text",
                        "text": { "content": code },
                        "annotations": {
                            "bold": false, "italic": false, "strikethrough": false,
                            "underline": false, "code": true, "color": "default"
                        }
                    }));
                    remaining = &after[close + 1..];
                } else {
                    spans.extend(span_chunks(remaining, false, false));
                    break;
                }
            }
            Some((pos, 1)) => {
                // Bold: **...**
                if pos > 0 {
                    spans.extend(span_chunks(&remaining[..pos], false, false));
                }
                let after = &remaining[pos + 2..];
                if let Some(close) = after.find("**") {
                    spans.extend(span_chunks(&after[..close], true, false));
                    remaining = &after[close + 2..];
                } else {
                    spans.extend(span_chunks(remaining, false, false));
                    break;
                }
            }
            Some((pos, _)) => {
                // Italic: *...*
                if pos > 0 {
                    spans.extend(span_chunks(&remaining[..pos], false, false));
                }
                let after = &remaining[pos + 1..];
                if let Some(close) = find_single_asterisk(after) {
                    spans.extend(span_chunks(&after[..close], false, true));
                    remaining = &after[close + 1..];
                } else {
                    spans.extend(span_chunks(remaining, false, false));
                    break;
                }
            }
        }
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
