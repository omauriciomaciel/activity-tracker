use crate::config::Config;
use super::{
    CF_ADD_BLOCK, CF_MACHINE, CF_MODEL, CF_NOTION_PAGE, CF_NOTION_TOKEN, CF_PROMPT,
    CF_SLACK, CF_URL_OR_KEY,
};
use crate::summarizer;

pub(super) fn cfg_cycle(current: &mut String, options: &[&str], dir: i32) {
    let pos = options
        .iter()
        .position(|&o| o == current.as_str())
        .unwrap_or(0);
    let n = options.len();
    let new_pos = ((pos as i32 + dir).rem_euclid(n as i32)) as usize;
    *current = options[new_pos].to_string();
}

pub(super) fn cfg_initial_value(cfg: &Config, cursor: usize) -> Option<String> {
    match cursor {
        CF_MODEL => Some(cfg.model.clone()),
        CF_URL_OR_KEY => Some(if cfg.provider == "ollama" {
            cfg.ollama_url.clone()
        } else {
            cfg.api_key.clone().unwrap_or_default()
        }),
        CF_PROMPT => Some(
            cfg.custom_prompt
                .clone()
                .unwrap_or_else(|| summarizer::DEFAULT_PROMPT_TEMPLATE.to_string()),
        ),
        CF_MACHINE => Some(cfg.machine_name.clone().unwrap_or_default()),
        CF_NOTION_TOKEN => Some(cfg.notion_token.clone().unwrap_or_default()),
        CF_NOTION_PAGE => Some(cfg.notion_page_id.clone().unwrap_or_default()),
        CF_SLACK => Some(cfg.slack_webhook.clone().unwrap_or_default()),
        CF_ADD_BLOCK => Some(String::new()),
        c if c > CF_ADD_BLOCK => cfg.blocked_patterns.get(c - CF_ADD_BLOCK - 1).cloned(),
        _ => None,
    }
}

pub(super) fn cfg_apply(cfg: &mut Config, cursor: usize, value: String) {
    let v = value.trim().to_string();
    match cursor {
        CF_MODEL => cfg.model = v,
        CF_URL_OR_KEY => {
            if cfg.provider == "ollama" {
                cfg.ollama_url = v;
            } else {
                cfg.api_key = if v.is_empty() { None } else { Some(v) };
            }
        }
        CF_PROMPT => cfg.custom_prompt = if v.is_empty() { None } else { Some(v) },
        CF_MACHINE => cfg.machine_name = if v.is_empty() { None } else { Some(v) },
        CF_NOTION_TOKEN => cfg.notion_token = if v.is_empty() { None } else { Some(v) },
        CF_NOTION_PAGE => cfg.notion_page_id = if v.is_empty() { None } else { Some(v) },
        CF_SLACK => cfg.slack_webhook = if v.is_empty() { None } else { Some(v) },
        CF_ADD_BLOCK => {
            if !v.is_empty() {
                cfg.blocked_patterns.push(v);
            }
        }
        c if c > CF_ADD_BLOCK => {
            let idx = c - CF_ADD_BLOCK - 1;
            if idx < cfg.blocked_patterns.len() {
                if v.is_empty() {
                    cfg.blocked_patterns.remove(idx);
                } else {
                    cfg.blocked_patterns[idx] = v;
                }
            }
        }
        _ => {}
    }
}
