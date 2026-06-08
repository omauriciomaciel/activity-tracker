use anyhow::Result;
use chrono::Local;
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
    pub last_commit: String,
}

// ─── Ponto de entrada ───────────────────────────

/// Executa todas as coletas e salva no arquivo JSONL do dia.
/// Retorna o número de entradas salvas.
pub fn collect_all(log_dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(log_dir)?;

    let date = Local::now().format("%Y-%m-%d").to_string();
    let ts = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let log_file = log_dir.join(format!("{date}.jsonl"));

    let mut entries: Vec<Entry> = Vec::new();

    // Shell history
    match capture_shell_history(log_dir, &ts) {
        Ok(e) => entries.push(e),
        Err(err) => eprintln!("Aviso:shell history: {err}"),
    }

    // Apps/janelas abertas
    match capture_open_windows(&ts) {
        Ok(e) => entries.push(e),
        Err(err) => eprintln!("Aviso:open windows: {err}"),
    }

    // Chrome tabs
    match capture_chrome_tabs(&ts) {
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

    let count = entries.len();
    for entry in &entries {
        use std::io::Write;
        let json = serde_json::to_string(entry)?;
        writeln!(file, "{json}")?;
    }

    Ok(count)
}

// ─── 1. Shell History ───────────────────────────

fn capture_shell_history(log_dir: &Path, ts: &str) -> Result<Entry> {
    let home = home_dir();
    let mut commands: Vec<String> = Vec::new();

    // Bash — lê incrementalmente via marker
    let bash_hist = home.join(".bash_history");
    if bash_hist.exists() {
        let marker = log_dir.join(".last_bash_pos");
        let new_cmds = read_incremental(&bash_hist, &marker, 100)?;
        commands.extend(new_cmds);
    }

    // Zsh — formato `: timestamp:0;comando`
    let zsh_hist = home.join(".zsh_history");
    if zsh_hist.exists() {
        let marker = log_dir.join(".last_zsh_pos");
        let raw = read_incremental(&zsh_hist, &marker, 100)?;
        for line in raw {
            let cmd = line
                .splitn(2, ';')
                .nth(1)
                .unwrap_or(&line)
                .to_string();
            if !cmd.is_empty() {
                commands.push(cmd);
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
                commands.push(cmd.to_string());
            }
        }
    }

    // Filtrar timestamps do bash HISTTIMEFORMAT (#1780520382) e comandos triviais
    let trivial: HashSet<&str> =
        ["ls", "cd", "clear", "pwd", "exit", "history", "ll", "la", "l"]
            .into_iter()
            .collect();

    commands.retain(|c| {
        let trimmed = c.trim();
        // Linha de timestamp do HISTTIMEFORMAT: "#" seguido só de dígitos
        if trimmed.starts_with('#') && trimmed[1..].chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
        // Só dígitos soltos (artefato de parse)
        if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
        let first = trimmed.split_whitespace().next().unwrap_or("");
        !first.is_empty() && !trivial.contains(first)
    });

    Ok(Entry::Shell {
        ts: ts.to_string(),
        commands,
    })
}

/// Lê novas linhas de um arquivo desde a última posição salva no marker.
fn read_incremental(file: &Path, marker: &Path, max: usize) -> Result<Vec<String>> {
    let meta = std::fs::metadata(file)?;
    let current_size = meta.len();

    let last_pos: u64 = if marker.exists() {
        std::fs::read_to_string(marker)?
            .trim()
            .parse()
            .unwrap_or(0)
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
        f.take(current_size - last_pos).read_to_string(&mut buf).ok();
        lines = buf
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(max)
            .map(|s| s.to_string())
            .collect();
    }

    // Atualizar marker
    std::fs::write(marker, current_size.to_string())?;
    Ok(lines)
}

// ─── 2. Janelas abertas ─────────────────────────

fn capture_open_windows(ts: &str) -> Result<Entry> {
    let mut windows: Vec<String> = Vec::new();

    // Tentar wmctrl (X11)
    if let Ok(out) = Command::new("wmctrl").args(["-l"]).output() {
        if out.status.success() {
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
                    if !title.is_empty() {
                        windows.push(title);
                    }
                }
            }
        }
    }

    // Fallback: xdotool
    if windows.is_empty() {
        if let Ok(out) = Command::new("xdotool")
            .args(["search", "--onlyvisible", "--name", ""])
            .output()
        {
            if out.status.success() {
                for line in out.stdout.lines().map_while(Result::ok) {
                    if let Ok(wid) = line.trim().parse::<u64>() {
                        if let Ok(name_out) = Command::new("xdotool")
                            .args(["getwindowname", &wid.to_string()])
                            .output()
                        {
                            let name = String::from_utf8_lossy(&name_out.stdout)
                                .trim()
                                .to_string();
                            if !name.is_empty() {
                                windows.push(name);
                            }
                        }
                    }
                }
                windows.truncate(30);
            }
        }
    }

    // Fallback macOS
    if windows.is_empty() && cfg!(target_os = "macos") {
        if let Ok(out) = Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events" to get name of every application process whose visible is true"#,
            ])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for app in text.split(", ") {
                    let name = app.trim().to_string();
                    if !name.is_empty() {
                        windows.push(name);
                    }
                }
            }
        }
    }

    // Último recurso: /proc no Linux
    if windows.is_empty() && cfg!(target_os = "linux") {
        if let Ok(out) = Command::new("ps")
            .args(["axo", "comm"])
            .output()
        {
            let gui_hints: HashSet<&str> = [
                "code", "firefox", "chrome", "chromium", "brave", "slack",
                "discord", "spotify", "telegram", "nautilus", "thunar",
                "alacritty", "kitty", "wezterm", "gnome-terminal",
                "konsole", "tilix", "obs", "gimp", "inkscape", "blender",
                "libreoffice", "thunderbird", "signal", "vlc", "mpv",
            ]
            .into_iter()
            .collect();

            let mut seen = HashSet::new();
            for line in out.stdout.lines().map_while(Result::ok) {
                let name = line.trim().to_string();
                if gui_hints.contains(name.as_str()) && seen.insert(name.clone()) {
                    windows.push(name);
                }
            }
        }
    }

    Ok(Entry::Apps {
        ts: ts.to_string(),
        windows,
    })
}

// ─── 3. Chrome Tabs ─────────────────────────────

fn capture_chrome_tabs(ts: &str) -> Result<Entry> {
    let mut tabs: Vec<TabInfo> = Vec::new();

    // Método 1: Chrome DevTools Protocol
    if let Ok(devtools_tabs) = fetch_devtools_tabs() {
        tabs = devtools_tabs;
    }

    // Método 2: SQLite history DB
    if tabs.is_empty() {
        tabs = read_chrome_history_db()?;
    }

    Ok(Entry::ChromeTabs {
        ts: ts.to_string(),
        tabs,
    })
}

fn fetch_devtools_tabs() -> Result<Vec<TabInfo>> {
    // Blocking HTTP pois collector roda em thread sync
    let body = reqwest::blocking::get("http://localhost:9222/json/list")?
        .text()?;

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

        // Chrome trava o DB — copiar para tmp
        let tmp = std::env::temp_dir().join(format!(
            "activity_tracker_chrome_{}.db",
            std::process::id()
        ));
        if std::fs::copy(db_path, &tmp).is_err() {
            continue;
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
    let mut repos: Vec<GitRepoInfo> = Vec::new();

    // find -maxdepth 4 -name .git
    if let Ok(out) = Command::new("find")
        .args([
            home.to_str().unwrap_or("."),
            "-maxdepth", "4",
            "-name", ".git",
            "-type", "d",
        ])
        .output()
    {
        for line in out.stdout.lines().map_while(Result::ok) {
            let git_dir = line.trim();
            let repo_dir = git_dir.strip_suffix("/.git").unwrap_or(git_dir);

            if let Ok(log_out) = Command::new("git")
                .args(["-C", repo_dir, "log", "-1", "--format=%ai %s"])
                .output()
            {
                let msg = String::from_utf8_lossy(&log_out.stdout).trim().to_string();
                if !msg.is_empty() {
                    repos.push(GitRepoInfo {
                        repo: repo_dir.to_string(),
                        last_commit: msg,
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

// ─── Helpers ────────────────────────────────────

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}
