use anyhow::{Context, Result};
use colored::Colorize;
use serde::Deserialize;
use std::path::PathBuf;
use termimad::MadSkin;

mod aggregate;
mod export;
mod llm;

pub use aggregate::{ActivityData, build_context, load_for_date};
pub use export::export_cmd;
pub use llm::{call_llm, DEFAULT_PROMPT_TEMPLATE};

use crate::config;

// ─── Tipos de parsing (shared por aggregate e export) ───────────────────────

#[derive(Deserialize)]
#[serde(tag = "type")]
pub(super) enum LogEntry {
    #[serde(rename = "shell")]
    Shell { commands: Vec<String> },
    #[serde(rename = "apps")]
    Apps { windows: Vec<String> },
    #[serde(rename = "chrome_tabs")]
    ChromeTabs { tabs: Vec<TabEntry> },
    #[serde(rename = "context")]
    Context { data: CtxData },
    #[serde(rename = "tag")]
    Tag { ts: String, label: String },
}

#[derive(Deserialize)]
pub(super) struct TabEntry {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize)]
pub(super) struct CtxData {
    #[serde(default)]
    pub git_repos: Vec<GitEntry>,
}

#[derive(Deserialize)]
pub(super) struct GitEntry {
    pub repo: String,
    #[serde(default)]
    pub commits: Vec<String>,
    #[serde(default)]
    pub last_commit: String,
}

pub(super) fn is_noise_command(cmd: &str) -> bool {
    let t = cmd.trim();
    let inner = t.strip_prefix('#').unwrap_or(t);
    if inner.chars().all(|c| c.is_ascii_digit()) && !inner.is_empty() {
        return true;
    }
    let first = t.split_whitespace().next().unwrap_or("");
    matches!(
        first,
        "ls" | "cd" | "clear" | "pwd" | "exit" | "history" | "ll" | "la" | "l"
    )
}

pub(super) fn strip_hostname_prefix(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
        i += 1;
    }
    if i > 5 && i < s.len() && bytes[i] == b' ' {
        s[i + 1..].trim()
    } else {
        s
    }
}

// ─── RunOptions e orquestração principal ─────────────────────────────────────

pub struct RunOptions<'a> {
    pub days: u32,
    pub date: Option<&'a str>,
    pub model: &'a str,
    pub provider: &'a str,
    pub ollama_url: &'a str,
    pub api_key: Option<&'a str>,
    pub lang: &'a str,
    pub machine_name: &'a str,
    pub notion: Option<(&'a str, &'a str)>,
    pub slack: Option<&'a str>,
    pub search: Option<&'a str>,
    pub custom_prompt: Option<&'a str>,
}

pub async fn run(opts: RunOptions<'_>) -> Result<()> {
    let RunOptions {
        days,
        date,
        model,
        provider,
        ollama_url,
        api_key,
        lang,
        machine_name,
        notion,
        slack,
        search,
        custom_prompt,
    } = opts;
    let log_dir = config::log_dir();

    let (files, label) = if let Some(raw) = date {
        let parsed = parse_date(raw)?;
        let path = log_dir.join(format!("{}.jsonl", parsed.format("%Y-%m-%d")));
        if !path.exists() {
            println!("Aviso: Nenhum log encontrado para {raw}.");
            println!("   Verifique o formato: YYYY-DD-MM  (ex: 2026-08-06)");
            return Ok(());
        }
        (vec![path], raw.to_string())
    } else {
        let files = find_log_files(&log_dir, days);
        let label = if days == 1 {
            "hoje".to_string()
        } else {
            format!("últimos {days} dia(s)")
        };
        (files, label)
    };

    if files.is_empty() {
        println!(
            "{} Nenhum log nos últimos {days} dias.",
            "Aviso:".yellow().bold()
        );
        println!("   Rode primeiro: {}", "activity-tracker start".cyan());
        println!("   Ou coleta manual: {}", "activity-tracker collect".cyan());
        return Ok(());
    }
    println!(
        "{} arquivo(s) de log encontrados",
        files.len().to_string().cyan()
    );

    let data = aggregate::aggregate(&files)?;
    let data = aggregate::condense_for_period(data, days);
    let data = if let Some(query) = search {
        let filtered = aggregate::filter_by_search(data, query);
        aggregate::print_search_results(&filtered, query);
        let empty = filtered.commands.is_empty()
            && filtered.top_apps.is_empty()
            && filtered.tabs.is_empty()
            && filtered.repos.is_empty()
            && filtered.tags.is_empty();
        if empty {
            println!("{}", "Nenhum resultado encontrado.".yellow());
            return Ok(());
        }
        filtered
    } else {
        data
    };

    let project_stats = if date.is_none() && days > 1 {
        crate::projects::load_stats(days)
            .ok()
            .filter(|s| !s.is_empty())
    } else {
        None
    };

    let context = if let Some(ref stats) = project_stats {
        format!(
            "{}{}",
            aggregate::build_context(&data),
            crate::projects::format_context(stats, days)
        )
    } else {
        aggregate::build_context(&data)
    };

    println!(
        "{}",
        format!("Enviando para {provider} (modelo: {model})...").dimmed()
    );
    let summary =
        llm::call_llm(provider, ollama_url, api_key, model, &context, lang, custom_prompt).await?;

    let border = "━".repeat(47);

    if let Some(ref stats) = project_stats {
        println!("\n{}", border.cyan());
        println!(
            "  {}  {}",
            "PROJETOS".bold().white(),
            format!("— últimos {days} dias").cyan()
        );
        println!("{}", border.cyan());
        let max_name = stats
            .iter()
            .map(|s| s.name.len())
            .max()
            .unwrap_or(10)
            .min(20);
        for s in stats {
            let bar_len = ((s.pct / 100.0) * 20.0).round() as usize;
            let bar = format!("{:<20}", "█".repeat(bar_len));
            println!(
                "  {:<width$}  {}  {:>5.1}%  ({}c, {}d)",
                s.name,
                bar,
                s.pct,
                s.commits,
                s.days_active,
                width = max_name,
            );
        }
    }

    println!("\n{}", border.cyan());
    println!(
        "  {}  {}",
        "RESUMO DE ATIVIDADES".bold().white(),
        format!("— {label}").cyan()
    );
    println!("  {}", format!("Modelo: {provider}/{model}").dimmed());
    println!("{}\n", border.cyan());
    let skin = MadSkin::default();
    skin.print_text(&summary);
    println!("\n{}", border.cyan());

    if days == 1 || date.is_some() {
        let save_date = if let Some(raw) = date {
            parse_date(raw).ok()
        } else {
            Some(chrono::Local::now().date_naive())
        };
        if let Some(d) = save_date {
            save_summary(d, &summary);
        }
    }

    let date_label = match data.dates.as_slice() {
        [] => chrono::Local::now().format("%Y-%m-%d").to_string(),
        [single] => single.clone(),
        dates => format!("{} a {}", dates.last().unwrap(), dates.first().unwrap()),
    };
    let title = format!("{date_label} — {machine_name}");

    if let Some((token, page_id)) = notion {
        print!("{}", "Enviando ao Notion...".dimmed());
        match crate::notion::send_page(token, page_id, &title, &summary).await {
            Ok(url) => println!(" {}", url.cyan()),
            Err(e) => eprintln!(" erro: {e}"),
        }
    }

    if let Some(webhook_url) = slack {
        print!("{}", "Enviando ao Slack...".dimmed());
        match crate::slack::send_message(webhook_url, &title, &summary).await {
            Ok(()) => println!(" {}", "ok".green()),
            Err(e) => eprintln!(" erro: {e}"),
        }
    }

    Ok(())
}

fn parse_date(s: &str) -> Result<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%d-%m")
        .with_context(|| format!("Data inválida: '{s}'. Use o formato YYYY-DD-MM (ex: 2026-08-06)"))
}

fn find_log_files(log_dir: &std::path::Path, days: u32) -> Vec<PathBuf> {
    let today = chrono::Local::now().date_naive();
    (0..days)
        .filter_map(|d| {
            let date = today - chrono::Duration::days(d as i64);
            let path = log_dir.join(format!("{}.jsonl", date.format("%Y-%m-%d")));
            path.exists().then_some(path)
        })
        .collect()
}

pub fn save_summary(date: chrono::NaiveDate, text: &str) {
    let dir = crate::config::summary_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = crate::config::summary_path(date);
    let _ = std::fs::write(path, text);
}

pub fn load_summary(date: chrono::NaiveDate) -> Option<String> {
    let path = crate::config::summary_path(date);
    std::fs::read_to_string(path).ok()
}
