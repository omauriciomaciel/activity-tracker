use anyhow::Result;
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::PathBuf;

use crate::config;
use super::{LogEntry, is_noise_command, strip_hostname_prefix};

// ─── Dados agregados ─────────────────────────────────────────────────────────

pub struct ActivityData {
    pub dates: Vec<String>,
    pub commands: Vec<String>,
    pub top_apps: Vec<(String, u32)>,
    pub tabs: Vec<(String, String)>,       // (title, url)
    pub repos: Vec<(String, Vec<String>)>, // (path, commits)
    pub tags: Vec<(String, String)>,       // (ts_hora, label)
}

pub fn load_for_date(date: chrono::NaiveDate) -> Result<ActivityData> {
    let log_dir = config::log_dir();
    let path = log_dir.join(format!("{}.jsonl", date.format("%Y-%m-%d")));
    if !path.exists() {
        return Ok(ActivityData {
            dates: vec![],
            commands: vec![],
            top_apps: vec![],
            tabs: vec![],
            repos: vec![],
            tags: vec![],
        });
    }
    aggregate(&[path])
}

pub fn build_context(data: &ActivityData) -> String {
    let mut out = String::new();

    out.push_str(&format!("Período: {}\n\n", data.dates.join(", ")));

    if !data.tags.is_empty() {
        out.push_str("=== NOTAS E EVENTOS ===\n");
        for (hour, label) in &data.tags {
            out.push_str(&format!("  {hour} - {label}\n"));
        }
        out.push('\n');
    }

    if !data.commands.is_empty() {
        out.push_str("=== COMANDOS DO TERMINAL ===\n");
        for cmd in &data.commands {
            let cmd = scrub_secrets(cmd);
            out.push_str(&format!("  {cmd}\n"));
        }
        out.push('\n');
    }

    if !data.top_apps.is_empty() {
        out.push_str("=== APLICATIVOS ABERTOS ===\n");
        for (app, count) in &data.top_apps {
            if *count > 1 {
                out.push_str(&format!("  {app} [{count}x]\n"));
            } else {
                out.push_str(&format!("  {app}\n"));
            }
        }
        out.push('\n');
    }

    if !data.tabs.is_empty() {
        out.push_str("=== SITES VISITADOS ===\n");
        let mut title_counts: Vec<(String, usize)> = Vec::new();
        let mut seen_titles: HashMap<String, usize> = HashMap::new();
        for (title, url) in &data.tabs {
            let label = if title.is_empty() || title == url {
                url.clone()
            } else {
                title.clone()
            };
            if let Some(idx) = seen_titles.get(&label) {
                title_counts[*idx].1 += 1;
            } else {
                seen_titles.insert(label.clone(), title_counts.len());
                title_counts.push((label, 1));
            }
        }
        for (label, count) in &title_counts {
            if *count > 1 {
                out.push_str(&format!("  {label} ({count}x)\n"));
            } else {
                out.push_str(&format!("  {label}\n"));
            }
        }
        out.push('\n');
    }

    if !data.repos.is_empty() {
        out.push_str("=== REPOSITÓRIOS GIT ===\n");
        for (repo, commits) in &data.repos {
            let name = std::path::Path::new(repo)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(repo);
            out.push_str(&format!("  {name}:\n"));
            for commit in commits {
                out.push_str(&format!("    - {commit}\n"));
            }
        }
        out.push('\n');
    }

    out
}

pub(super) fn aggregate(files: &[PathBuf]) -> Result<ActivityData> {
    let mut seen_commands: HashSet<String> = HashSet::new();
    let mut commands: Vec<String> = Vec::new();
    let mut app_counts: HashMap<String, u32> = HashMap::new();
    let mut seen_urls: HashSet<String> = HashSet::new();
    let mut tabs: Vec<(String, String)> = Vec::new();
    let mut repo_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut dates: Vec<String> = Vec::new();
    let mut tags: Vec<(String, String)> = Vec::new();

    for file in files {
        let date_str = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let file_date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok();
        dates.push(date_str);

        let f = std::fs::File::open(file)?;
        let lines: Vec<String> = std::io::BufReader::new(f)
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().is_empty())
            .collect();
        for line in lines.iter().rev() {
            let line = line.as_str();

            match serde_json::from_str::<LogEntry>(line) {
                Ok(LogEntry::Shell { commands: cmds }) => {
                    for cmd in cmds {
                        let cmd = cmd.trim().to_string();
                        if cmd.is_empty() || is_noise_command(&cmd) {
                            continue;
                        }
                        if seen_commands.insert(cmd.clone()) {
                            commands.push(cmd);
                        }
                    }
                }
                Ok(LogEntry::Apps { windows }) => {
                    for w in windows {
                        let w = strip_hostname_prefix(w.trim()).to_string();
                        if !w.is_empty() {
                            *app_counts.entry(w).or_default() += 1;
                        }
                    }
                }
                Ok(LogEntry::ChromeTabs { tabs: t }) => {
                    for tab in t {
                        let url = tab.url.trim().to_string();
                        if !url.is_empty() && seen_urls.insert(url.clone()) {
                            tabs.push((tab.title.trim().to_string(), url));
                        }
                    }
                }
                Ok(LogEntry::Tag { ts, label }) => {
                    let hour = ts.get(11..16).unwrap_or(&ts).to_string();
                    tags.push((hour, label));
                }
                Ok(LogEntry::Context { data }) => {
                    for r in data.git_repos {
                        let incoming: Vec<String> = if !r.commits.is_empty() {
                            r.commits
                        } else if !r.last_commit.is_empty() {
                            vec![r.last_commit]
                        } else {
                            continue;
                        };

                        let entry = repo_map.entry(r.repo).or_default();
                        for c in incoming {
                            if let Some(file_date) = file_date {
                                let commit_date = c.get(..10).and_then(|s| {
                                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
                                });
                                if commit_date.map(|d| d != file_date).unwrap_or(false) {
                                    continue;
                                }
                            }
                            if !entry.contains(&c) {
                                entry.push(c);
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        }
    }

    let mut top_apps: Vec<(String, u32)> = app_counts.into_iter().collect();
    top_apps.sort_by_key(|b| std::cmp::Reverse(b.1));
    top_apps.truncate(15);

    tabs.truncate(30);

    let mut repos: Vec<(String, Vec<String>)> = repo_map
        .into_iter()
        .filter(|(_, commits)| !commits.is_empty())
        .collect();
    repos.sort_by(|a, b| {
        let latest_a = a.1.iter().max().map(|s| s.as_str()).unwrap_or("");
        let latest_b = b.1.iter().max().map(|s| s.as_str()).unwrap_or("");
        latest_b.cmp(latest_a)
    });

    tags.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(ActivityData {
        dates,
        commands: commands.into_iter().take(150).collect(),
        top_apps,
        tabs,
        repos,
        tags,
    })
}

pub(super) fn condense_for_period(mut data: ActivityData, days: u32) -> ActivityData {
    if days > 6 {
        let cmd_limit = if days > 20 { 60 } else { 100 };
        data.commands.truncate(cmd_limit);
        data.top_apps.truncate(10);
        data.tabs.truncate(20);
    }
    data
}

pub(super) fn filter_by_search(data: ActivityData, query: &str) -> ActivityData {
    let q = query.to_lowercase();
    let commands = data
        .commands
        .into_iter()
        .filter(|c| c.to_lowercase().contains(&q))
        .collect();
    let top_apps = data
        .top_apps
        .into_iter()
        .filter(|(a, _)| a.to_lowercase().contains(&q))
        .collect();
    let tabs = data
        .tabs
        .into_iter()
        .filter(|(t, u)| t.to_lowercase().contains(&q) || u.to_lowercase().contains(&q))
        .collect();
    let repos = data
        .repos
        .into_iter()
        .filter_map(|(r, commits)| {
            let repo_matches = r.to_lowercase().contains(&q);
            let commits: Vec<String> = commits
                .into_iter()
                .filter(|c| c.to_lowercase().contains(&q) || repo_matches)
                .collect();
            if repo_matches || !commits.is_empty() {
                Some((r, commits))
            } else {
                None
            }
        })
        .collect();
    let tags = data
        .tags
        .into_iter()
        .filter(|(_, label)| label.to_lowercase().contains(&q))
        .collect();
    ActivityData {
        dates: data.dates,
        commands,
        top_apps,
        tabs,
        repos,
        tags,
    }
}

pub(super) fn print_search_results(data: &ActivityData, query: &str) {
    let total = data.commands.len()
        + data.top_apps.len()
        + data.tabs.len()
        + data.repos.iter().map(|(_, c)| c.len()).sum::<usize>();
    let border = "━".repeat(47);
    println!("\n{}", border.cyan());
    println!(
        "  {}  {}",
        "RESULTADOS".bold().white(),
        format!("— \"{}\"  ({} match(es))", query, total).cyan()
    );
    println!("{}\n", border.cyan());
    if !data.commands.is_empty() {
        println!("{}", "[Comandos]".bold());
        for cmd in &data.commands {
            println!("  {cmd}");
        }
        println!();
    }
    if !data.top_apps.is_empty() {
        println!("{}", "[Aplicativos]".bold());
        for (app, _) in &data.top_apps {
            println!("  {app}");
        }
        println!();
    }
    if !data.tabs.is_empty() {
        println!("{}", "[Sites]".bold());
        for (title, url) in &data.tabs {
            if title.is_empty() || title == url {
                println!("  {url}");
            } else {
                println!("  {title}");
            }
        }
        println!();
    }
    if !data.repos.is_empty() {
        println!("{}", "[Git]".bold());
        for (repo, commits) in &data.repos {
            let name = std::path::Path::new(repo)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(repo);
            for commit in commits {
                println!("  {name}: {commit}");
            }
        }
        println!();
    }
}

fn scrub_secrets(cmd: &str) -> String {
    const SENSITIVE: &[&str] = &[
        "password",
        "passwd",
        "secret",
        "token",
        "apikey",
        "api_key",
        "auth",
        "credential",
        "private",
        "pass",
        "bearer",
        "access_key",
        "secret_key",
        "private_key",
    ];

    let tokens: Vec<&str> = cmd.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(tokens.len());
    let mut redact_next = false;

    for token in &tokens {
        if redact_next {
            out.push("[REDACTED]".to_string());
            redact_next = false;
            continue;
        }
        if let Some(eq_pos) = token.find('=') {
            let key = token[..eq_pos].trim_start_matches('-').to_lowercase();
            if SENSITIVE.iter().any(|s| key.contains(s)) && eq_pos + 1 < token.len() {
                out.push(format!("{}=[REDACTED]", &token[..eq_pos]));
                continue;
            }
        }
        let flag_name = token
            .trim_start_matches('-')
            .split('=')
            .next()
            .unwrap_or("")
            .to_lowercase();
        if token.starts_with('-')
            && !token.contains('=')
            && SENSITIVE.iter().any(|s| flag_name.contains(s))
        {
            out.push(token.to_string());
            redact_next = true;
            continue;
        }
        out.push(token.to_string());
    }

    out.join(" ")
}
