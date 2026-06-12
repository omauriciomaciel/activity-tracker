use anyhow::Result;
use chrono::{Local, NaiveDate};
use serde::Serialize;
use std::collections::HashSet;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command;

// ─── Tipos ──────────────────────────────────────

#[derive(Serialize)]
#[serde(tag = "type")]
enum Entry {
    #[serde(rename = "shell")]
    Shell { ts: String, commands: Vec<String> },

    #[serde(rename = "apps")]
    Apps { ts: String, windows: Vec<String> },

    #[serde(rename = "chrome_tabs")]
    ChromeTabs { ts: String, tabs: Vec<TabInfo> },

    #[serde(rename = "context")]
    Context { ts: String, data: ContextData },

    #[serde(rename = "tag")]
    Tag { ts: String, label: String },
}

#[derive(Serialize)]
pub struct TabInfo {
    pub title: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visited_at: Option<String>,
}

#[derive(Serialize)]
pub struct ContextData {
    pub git_repos: Vec<GitRepoInfo>,
}

#[derive(Serialize)]
pub struct GitRepoInfo {
    pub repo: String,
    pub commits: Vec<String>,
}

// ─── Ponto de entrada ───────────────────────────

/// Executa todas as coletas e salva no arquivo JSONL do dia.
/// Retorna o número de entradas salvas.
pub fn collect_all(log_dir: &Path, blocked: &[String]) -> Result<usize> {
    std::fs::create_dir_all(log_dir)?;

    let date = Local::now().format("%Y-%m-%d").to_string();
    let ts = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let log_file = log_dir.join(format!("{date}.jsonl"));

    let mut entries: Vec<Entry> = Vec::new();

    // Shell history
    match capture_shell_history(log_dir, &ts, blocked) {
        Ok(e) => entries.push(e),
        Err(err) => eprintln!("Aviso:shell history: {err}"),
    }

    // Apps/janelas abertas
    match capture_open_windows(&ts, blocked) {
        Ok(e) => entries.push(e),
        Err(err) => eprintln!("Aviso:open windows: {err}"),
    }

    // Chrome tabs
    match capture_chrome_tabs(&ts, blocked) {
        Ok(e) => entries.push(e),
        Err(err) => eprintln!("Aviso:chrome tabs: {err}"),
    }

    // Git context
    match capture_git_context(&ts) {
        Ok(e) => entries.push(e),
        Err(err) => eprintln!("Aviso:git context: {err}"),
    }

    // Salvar
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&log_file, std::fs::Permissions::from_mode(0o600));
    }

    let count = entries.len();
    for entry in &entries {
        use std::io::Write;
        let json = serde_json::to_string(entry)?;
        writeln!(file, "{json}")?;
    }

    clean_all_logs(log_dir)?;

    Ok(count)
}

// ─── Privacidade ────────────────────────────────

fn is_blocked(text: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let lower = text.to_lowercase();
    patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
}

// ─── 1. Shell History ───────────────────────────

fn capture_shell_history(log_dir: &Path, ts: &str, blocked: &[String]) -> Result<Entry> {
    let home = home_dir();
    let today = Local::now().date_naive();
    let trivial: HashSet<&str> = [
        "ls", "cd", "clear", "pwd", "exit", "history", "ll", "la", "l",
    ]
    .into_iter()
    .collect();
    let mut commands: Vec<String> = Vec::new();

    // Bash — usa timestamps do HISTTIMEFORMAT para filtrar por data
    let bash_hist = home.join(".bash_history");
    if bash_hist.exists() {
        let marker = log_dir.join(".last_bash_pos");
        let raw = read_incremental(&bash_hist, &marker, 400)?;
        for cmd in filter_by_date(raw, today) {
            let trimmed = cmd.trim();
            let first = trimmed.split_whitespace().next().unwrap_or("");
            if !first.is_empty() && !trivial.contains(first) && !is_blocked(trimmed, blocked) {
                commands.push(trimmed.to_string());
            }
        }
    }

    // Zsh — formato `: timestamp:0;comando`
    let zsh_hist = home.join(".zsh_history");
    if zsh_hist.exists() {
        let marker = log_dir.join(".last_zsh_pos");
        let raw = read_incremental(&zsh_hist, &marker, 100)?;
        for line in raw {
            // ": 1780520426:0;git status"
            let (ts_secs, cmd) = if let Some(rest) = line.strip_prefix(": ") {
                let mut parts = rest.splitn(2, ';');
                let meta = parts.next().unwrap_or("");
                let cmd = parts.next().unwrap_or("").to_string();
                let secs = meta.split(':').next().unwrap_or("").trim().to_string();
                (secs, cmd)
            } else {
                (
                    String::new(),
                    line.split_once(';')
                        .map(|x| x.1)
                        .unwrap_or(&line)
                        .to_string(),
                )
            };

            if cmd.is_empty() {
                continue;
            }

            if !ts_secs.is_empty()
                && let Ok(secs) = ts_secs.parse::<i64>()
            {
                let cmd_date = chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.date_naive());
                if cmd_date.map(|d| d != today).unwrap_or(false) {
                    continue;
                }
            }

            let first = cmd.split_whitespace().next().unwrap_or("");
            if !first.is_empty() && !trivial.contains(first) && !is_blocked(cmd.trim(), blocked) {
                commands.push(cmd.trim().to_string());
            }
        }
    }

    // Fish — linhas `- cmd: ...`
    let fish_hist = home.join(".local/share/fish/fish_history");
    if fish_hist.exists() {
        let marker = log_dir.join(".last_fish_pos");
        let raw = read_incremental(&fish_hist, &marker, 200)?;
        for line in raw {
            if let Some(cmd) = line.strip_prefix("- cmd: ") {
                let first = cmd.split_whitespace().next().unwrap_or("");
                if !first.is_empty() && !trivial.contains(first) && !is_blocked(cmd.trim(), blocked)
                {
                    commands.push(cmd.trim().to_string());
                }
            }
        }
    }

    Ok(Entry::Shell {
        ts: ts.to_string(),
        commands,
    })
}

/// Itera sobre linhas de bash history com HISTTIMEFORMAT, retornando apenas
/// os comandos cujo timestamp corresponde à `date` alvo.
/// Se não houver nenhum marcador de timestamp no lote, retorna todos os comandos
/// (o read_incremental já garante que são linhas novas desde a última coleta).
fn filter_by_date(raw: Vec<String>, date: NaiveDate) -> Vec<String> {
    let has_timestamps = raw.iter().any(|line| {
        let inner = line.trim().strip_prefix('#').unwrap_or(line.trim());
        !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit())
    });

    if !has_timestamps {
        return raw
            .into_iter()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
    }

    let mut result = Vec::new();
    let mut current_date: Option<NaiveDate> = None;

    for line in raw {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let inner = trimmed.strip_prefix('#').unwrap_or(trimmed);
        if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(secs) = inner.parse::<i64>() {
                current_date = chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.date_naive());
            }
            continue;
        }

        if current_date.map(|d| d == date).unwrap_or(false) {
            result.push(trimmed.to_string());
        }
    }
    result
}

/// Lê novas linhas de um arquivo desde a última posição salva no marker.
fn read_incremental(file: &Path, marker: &Path, max: usize) -> Result<Vec<String>> {
    let meta = std::fs::metadata(file)?;
    let current_size = meta.len();

    let last_pos: u64 = if marker.exists() {
        std::fs::read_to_string(marker)?.trim().parse().unwrap_or(0)
    } else {
        // Primeira vez: pegar só as últimas linhas
        current_size.saturating_sub(4096)
    };

    let mut lines = Vec::new();

    if current_size > last_pos {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = std::fs::File::open(file)?;
        f.seek(SeekFrom::Start(last_pos))?;
        let mut buf = String::new();
        f.take(current_size - last_pos)
            .read_to_string(&mut buf)
            .ok();
        lines = buf
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(max)
            .map(|s| s.to_string())
            .collect();
    }

    // Atualizar marker
    std::fs::write(marker, current_size.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(marker, std::fs::Permissions::from_mode(0o600));
    }
    Ok(lines)
}

// ─── 2. Janelas abertas ─────────────────────────

fn capture_open_windows(ts: &str, blocked: &[String]) -> Result<Entry> {
    let mut windows: Vec<String> = Vec::new();

    // Tentar wmctrl (X11)
    if let Ok(out) = Command::new("wmctrl").args(["-l"]).output()
        && out.status.success()
    {
        for line in out.stdout.lines().map_while(Result::ok) {
            // Formato: 0x... desktop_num host título_da_janela
            let parts: Vec<&str> = line.splitn(4, char::is_whitespace).collect();
            if parts.len() >= 4 {
                let raw_title = parts[3..].join(" ");
                // Remover prefixo de hostname que wmctrl inclui
                let hostname = parts[2]; // 3ª coluna é o hostname
                let title = raw_title
                    .trim()
                    .strip_prefix(hostname)
                    .unwrap_or(raw_title.trim())
                    .trim()
                    .to_string();
                if !title.is_empty() && !is_blocked(&title, blocked) {
                    windows.push(title);
                }
            }
        }
    }

    // Fallback: xdotool
    if windows.is_empty()
        && let Ok(out) = Command::new("xdotool")
            .args(["search", "--onlyvisible", "--name", ""])
            .output()
        && out.status.success()
    {
        for line in out.stdout.lines().map_while(Result::ok) {
            if let Ok(wid) = line.trim().parse::<u64>()
                && let Ok(name_out) = Command::new("xdotool")
                    .args(["getwindowname", &wid.to_string()])
                    .output()
            {
                let name = String::from_utf8_lossy(&name_out.stdout).trim().to_string();
                if !name.is_empty() && !is_blocked(&name, blocked) {
                    windows.push(name);
                }
            }
        }
        windows.truncate(30);
    }

    // Fallback macOS
    if windows.is_empty() && cfg!(target_os = "macos")
        && let Ok(out) = Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events" to get name of every application process whose visible is true"#,
            ])
            .output()
            && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for app in text.split(", ") {
            let name = app.trim().to_string();
            if !name.is_empty() && !is_blocked(&name, blocked) {
                windows.push(name);
            }
        }
    }

    // Último recurso: /proc no Linux
    if windows.is_empty()
        && cfg!(target_os = "linux")
        && let Ok(out) = Command::new("ps").args(["axo", "comm"]).output()
    {
        let gui_hints: HashSet<&str> = [
            "code",
            "firefox",
            "chrome",
            "chromium",
            "brave",
            "slack",
            "discord",
            "spotify",
            "telegram",
            "nautilus",
            "thunar",
            "alacritty",
            "kitty",
            "wezterm",
            "gnome-terminal",
            "konsole",
            "tilix",
            "obs",
            "gimp",
            "inkscape",
            "blender",
            "libreoffice",
            "thunderbird",
            "signal",
            "vlc",
            "mpv",
        ]
        .into_iter()
        .collect();

        let mut seen = HashSet::new();
        for line in out.stdout.lines().map_while(Result::ok) {
            let name = line.trim().to_string();
            if gui_hints.contains(name.as_str())
                && seen.insert(name.clone())
                && !is_blocked(&name, blocked)
            {
                windows.push(name);
            }
        }
    }

    Ok(Entry::Apps {
        ts: ts.to_string(),
        windows,
    })
}

// ─── 3. Chrome Tabs ─────────────────────────────

fn capture_chrome_tabs(ts: &str, blocked: &[String]) -> Result<Entry> {
    let mut tabs: Vec<TabInfo> = Vec::new();

    // Método 1: Chrome DevTools Protocol
    if let Ok(devtools_tabs) = fetch_devtools_tabs() {
        tabs = devtools_tabs;
    }

    // Método 2: SQLite history DB
    if tabs.is_empty() {
        tabs = read_chrome_history_db()?;
    }

    tabs.retain(|t| !is_blocked(&t.url, blocked) && !is_blocked(&t.title, blocked));

    Ok(Entry::ChromeTabs {
        ts: ts.to_string(),
        tabs,
    })
}

fn fetch_devtools_tabs() -> Result<Vec<TabInfo>> {
    // Blocking HTTP pois collector roda em thread sync
    let body = reqwest::blocking::get("http://localhost:9222/json/list")?.text()?;

    let entries: Vec<serde_json::Value> = serde_json::from_str(&body)?;
    let tabs = entries
        .iter()
        .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("page"))
        .map(|e| TabInfo {
            title: e["title"].as_str().unwrap_or("").to_string(),
            url: e["url"].as_str().unwrap_or("").to_string(),
            visited_at: None,
        })
        .collect();

    Ok(tabs)
}

fn read_chrome_history_db() -> Result<Vec<TabInfo>> {
    let home = home_dir();
    let candidates = [
        home.join(".config/google-chrome/Default/History"),
        home.join(".config/chromium/Default/History"),
        home.join(".config/BraveSoftware/Brave-Browser/Default/History"),
        // macOS
        home.join("Library/Application Support/Google/Chrome/Default/History"),
        home.join("Library/Application Support/Chromium/Default/History"),
        home.join("Library/Application Support/BraveSoftware/Brave-Browser/Default/History"),
    ];

    for db_path in &candidates {
        if !db_path.exists() {
            continue;
        }

        // Chrome trava o DB — copiar para dir privado do usuário (evita TOCTOU em /tmp)
        let private_dir = dirs::data_local_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("activity-tracker");
        let _ = std::fs::create_dir_all(&private_dir);
        let tmp = private_dir.join(format!("chrome_tmp_{}.db", std::process::id()));
        // H4 — limpar temp mesmo se copy falhar
        if std::fs::copy(db_path, &tmp).is_err() {
            let _ = std::fs::remove_file(&tmp);
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }

        let result = (|| -> Result<Vec<TabInfo>> {
            let conn = rusqlite::Connection::open_with_flags(
                &tmp,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;

            // Últimas 2h. Chrome: microseg desde 1601-01-01
            let mut stmt = conn.prepare(
                "SELECT title, url,
                        datetime(last_visit_time/1000000 - 11644473600, 'unixepoch', 'localtime')
                 FROM urls
                 WHERE last_visit_time > (
                     (strftime('%s','now') + 11644473600) * 1000000 - 7200000000
                 )
                 ORDER BY last_visit_time DESC
                 LIMIT 40",
            )?;

            let tabs = stmt
                .query_map([], |row| {
                    Ok(TabInfo {
                        title: row.get(0)?,
                        url: row.get(1)?,
                        visited_at: row.get(2).ok(),
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(tabs)
        })();

        let _ = std::fs::remove_file(&tmp);

        if let Ok(tabs) = result {
            return Ok(tabs);
        }
    }

    Ok(Vec::new())
}

// ─── 4. Git context ─────────────────────────────

fn capture_git_context(ts: &str) -> Result<Entry> {
    let home = home_dir();
    let today = Local::now().date_naive();
    let mut repos: Vec<GitRepoInfo> = Vec::new();

    // find -maxdepth 4 -name .git
    if let Ok(out) = Command::new("find")
        .args([
            "-P", // never follow symlinks
            home.to_str().unwrap_or("."),
            "-maxdepth",
            "4",
            "-name",
            ".git",
            "-type",
            "d",
        ])
        .output()
    {
        for line in out.stdout.lines().map_while(Result::ok) {
            let git_dir = line.trim();
            let repo_dir = git_dir.strip_suffix("/.git").unwrap_or(git_dir);

            // Reject paths outside $HOME (symlink escape guard)
            if !std::path::Path::new(repo_dir).starts_with(&home) {
                continue;
            }

            let since = today.format("%Y-%m-%d").to_string();
            if let Ok(log_out) = Command::new("git")
                .args([
                    "-C",
                    repo_dir,
                    "log",
                    &format!("--since={} 00:00:00", since),
                    "--format=%ai %s",
                ])
                .output()
            {
                let commits: Vec<String> = String::from_utf8_lossy(&log_out.stdout)
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
                if !commits.is_empty() {
                    repos.push(GitRepoInfo {
                        repo: repo_dir.to_string(),
                        commits,
                    });
                }
            }

            if repos.len() >= 15 {
                break;
            }
        }
    }

    Ok(Entry::Context {
        ts: ts.to_string(),
        data: ContextData { git_repos: repos },
    })
}

// ─── Limpeza de logs existentes ─────────────────

/// Reprocessa todos os arquivos JSONL no diretório, removendo entradas fora da data do arquivo.
/// Retorna o número de entradas removidas no total.
pub fn clean_all_logs(log_dir: &Path) -> Result<usize> {
    let mut total_removed = 0;

    let entries = match std::fs::read_dir(log_dir) {
        Ok(e) => e,
        Err(_) => return Ok(0),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let date = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        if let Some(date) = date {
            total_removed += clean_log_file(&path, date)?;
        }
    }

    Ok(total_removed)
}

fn clean_log_file(path: &Path, date: NaiveDate) -> Result<usize> {
    let content = std::fs::read_to_string(path)?;
    let mut out_lines: Vec<String> = Vec::new();
    let mut removed = 0;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut val: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                out_lines.push(line.to_string());
                continue;
            }
        };

        match val.get("type").and_then(|t| t.as_str()) {
            Some("shell") => {
                if let Some(cmds) = val["commands"].as_array().cloned() {
                    // Reutiliza filter_by_date: converte Value → String, filtra, converte de volta
                    let raw: Vec<String> = cmds
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    let before = raw.len();
                    let kept = filter_by_date(raw, date);
                    removed += before.saturating_sub(kept.len());

                    if kept.is_empty() {
                        removed += 1;
                        continue;
                    }
                    val["commands"] = serde_json::json!(kept);
                }
            }
            Some("context") => {
                if let Some(repos) = val["data"]["git_repos"].as_array().cloned() {
                    let kept: Vec<serde_json::Value> = repos
                        .into_iter()
                        .filter(|r| {
                            // Keep repos that have at least one commit on the target date
                            r["commits"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter().any(|c| {
                                        c.as_str()
                                            .and_then(|s| s.get(..10))
                                            .and_then(|s| {
                                                NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
                                            })
                                            .map(|d| d == date)
                                            .unwrap_or(false)
                                    })
                                })
                                .unwrap_or(false)
                        })
                        .collect();

                    let removed_here = val["data"]["git_repos"]
                        .as_array()
                        .map(|a| a.len())
                        .unwrap_or(0)
                        .saturating_sub(kept.len());
                    removed += removed_here;

                    if kept.is_empty() {
                        removed += 1;
                        continue;
                    }
                    val["data"]["git_repos"] = serde_json::json!(kept);
                }
            }
            _ => {}
        }

        out_lines.push(serde_json::to_string(&val)?);
    }

    let new_content = out_lines.join("\n") + if out_lines.is_empty() { "" } else { "\n" };
    std::fs::write(path, new_content)?;
    Ok(removed)
}

// ─── Tags manuais ────────────────────────────────

/// Escreve uma entrada de tag manual no JSONL do dia (ou de uma data específica).
pub fn write_tag(log_dir: &Path, label: &str, date_opt: Option<NaiveDate>) -> Result<()> {
    std::fs::create_dir_all(log_dir)?;

    let (date, ts) = if let Some(d) = date_opt {
        let ts = format!("{}T00:00:00", d.format("%Y-%m-%d"));
        (d.format("%Y-%m-%d").to_string(), ts)
    } else {
        let now = Local::now();
        (
            now.format("%Y-%m-%d").to_string(),
            now.format("%Y-%m-%dT%H:%M:%S").to_string(),
        )
    };

    let log_file = log_dir.join(format!("{date}.jsonl"));

    let entry = Entry::Tag {
        ts,
        label: label.trim().to_string(),
    };

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&log_file, std::fs::Permissions::from_mode(0o600));
    }

    use std::io::Write;
    let json = serde_json::to_string(&entry)?;
    writeln!(file, "{json}")?;

    Ok(())
}

pub fn delete_tag(log_dir: &Path, label: &str, date_opt: Option<NaiveDate>) -> Result<()> {
    let date = date_opt.unwrap_or_else(|| Local::now().date_naive());
    let path = log_dir.join(format!("{}.jsonl", date.format("%Y-%m-%d")));
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&path)?;
    let target = label.trim().to_lowercase();
    let kept: Vec<&str> = content
        .lines()
        .filter(|line| {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if v["type"] == "tag"
                    && v["label"].as_str().map(|s| s.to_lowercase()) == Some(target.clone())
                {
                    return false;
                }
            }
            true
        })
        .collect();
    let new_content = if kept.is_empty() {
        String::new()
    } else {
        format!("{}\n", kept.join("\n"))
    };
    std::fs::write(&path, new_content)?;
    Ok(())
}

// ─── Helpers ────────────────────────────────────

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}
