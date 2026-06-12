use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::io::BufRead;
use std::path::PathBuf;

use crate::config;
use super::{LogEntry, is_noise_command, strip_hostname_prefix};

pub fn export_cmd(days: u32, date: Option<&str>, format: &str, output: Option<&str>) -> Result<()> {
    let log_dir = config::log_dir();
    let files = if let Some(raw) = date {
        let parsed = super::parse_date(raw)?;
        let path = log_dir.join(format!("{}.jsonl", parsed.format("%Y-%m-%d")));
        if path.exists() {
            vec![path]
        } else {
            eprintln!("Aviso: nenhum log para a data {raw}.");
            return Ok(());
        }
    } else {
        super::find_log_files(&log_dir, days)
    };
    if files.is_empty() {
        eprintln!("Nenhum log encontrado.");
        return Ok(());
    }
    export_raw(&files, format, output)
}

fn export_raw(files: &[PathBuf], format: &str, output: Option<&str>) -> Result<()> {
    let mut rows: Vec<(String, &'static str, String)> = Vec::new();

    for file in files {
        let date_str = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let f = std::fs::File::open(file)?;
        let mut seen: HashSet<String> = HashSet::new();
        for line in std::io::BufReader::new(f)
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().is_empty())
        {
            match serde_json::from_str::<LogEntry>(&line) {
                Ok(LogEntry::Shell { commands }) => {
                    for cmd in commands {
                        let cmd = cmd.trim().to_string();
                        if cmd.is_empty() || is_noise_command(&cmd) {
                            continue;
                        }
                        if seen.insert(format!("s:{cmd}")) {
                            rows.push((date_str.clone(), "shell", cmd));
                        }
                    }
                }
                Ok(LogEntry::Apps { windows }) => {
                    for w in windows {
                        let w = strip_hostname_prefix(w.trim()).to_string();
                        if w.is_empty() {
                            continue;
                        }
                        if seen.insert(format!("a:{w}")) {
                            rows.push((date_str.clone(), "app", w));
                        }
                    }
                }
                Ok(LogEntry::ChromeTabs { tabs }) => {
                    for t in tabs {
                        let url = t.url.trim().to_string();
                        if url.is_empty() {
                            continue;
                        }
                        if seen.insert(format!("t:{url}")) {
                            let content = if t.title.trim().is_empty() || t.title.trim() == url {
                                url
                            } else {
                                format!("{} | {}", t.title.trim(), t.url.trim())
                            };
                            rows.push((date_str.clone(), "tab", content));
                        }
                    }
                }
                Ok(LogEntry::Context { data }) => {
                    for r in data.git_repos {
                        let repo = r.repo.clone();
                        let commits: Vec<String> = if !r.commits.is_empty() {
                            r.commits
                        } else if !r.last_commit.is_empty() {
                            vec![r.last_commit]
                        } else {
                            continue;
                        };
                        for c in commits {
                            let content = format!("{repo} | {c}");
                            if seen.insert(format!("g:{content}")) {
                                rows.push((date_str.clone(), "git", content));
                            }
                        }
                    }
                }
                Ok(LogEntry::Tag { label, .. }) => {
                    if seen.insert(format!("n:{label}")) {
                        rows.push((date_str.clone(), "tag", label));
                    }
                }
                Err(_) => {}
            }
        }
    }

    let out = match format {
        "json" => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|(d, t, c)| serde_json::json!({"date": d, "type": t, "content": c}))
                .collect();
            serde_json::to_string_pretty(&arr)?
        }
        _ => {
            let mut s = String::from("date,type,content\n");
            for (date, typ, content) in &rows {
                let escaped = content.replace('"', "\"\"");
                s.push_str(&format!("{date},{typ},\"{escaped}\"\n"));
            }
            s
        }
    };

    match output {
        Some(path) => {
            std::fs::write(path, &out)?;
            eprintln!("{} {}", "Exportado:".green(), path);
        }
        None => print!("{out}"),
    }
    Ok(())
}
