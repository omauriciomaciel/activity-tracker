use crate::config;
use anyhow::{Context, Result};
use colored::Colorize;
use termimad::MadSkin;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::PathBuf;

// ─── Tipos internos para parsing ────────────────

#[derive(Deserialize)]
#[serde(tag = "type")]
enum LogEntry {
    #[serde(rename = "shell")]
    Shell { commands: Vec<String> },
    #[serde(rename = "apps")]
    Apps { windows: Vec<String> },
    #[serde(rename = "chrome_tabs")]
    ChromeTabs { tabs: Vec<TabEntry> },
    #[serde(rename = "context")]
    Context { data: CtxData },
}

#[derive(Deserialize)]
struct TabEntry {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
}

#[derive(Deserialize)]
struct CtxData {
    #[serde(default)]
    git_repos: Vec<GitEntry>,
}

#[derive(Deserialize)]
struct GitEntry {
    repo: String,
    last_commit: String,
}

// ─── Dados agregados ─────────────────────────────

struct ActivityData {
    dates: Vec<String>,
    commands: Vec<String>,
    top_apps: Vec<(String, u32)>,
    tabs: Vec<(String, String)>, // (title, url)
    repos: Vec<(String, String)>, // (path, last_commit)
}

// ─── Ollama ─────────────────────────────────────

#[derive(Serialize)]
struct OllamaReq {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOpts,
}

#[derive(Serialize)]
struct OllamaOpts {
    temperature: f32,
    num_predict: i32,
    seed: i64,
}

#[derive(Deserialize)]
struct OllamaResp {
    response: String,
}

// ─── Lógica principal ───────────────────────────

pub async fn run(
    days: u32,
    date: Option<&str>,
    model: &str,
    ollama_url: &str,
    lang: &str,
    verbose: bool,
) -> Result<()> {
    let log_dir = config::log_dir();

    let (files, label) = if let Some(raw) = date {
        let parsed = parse_date(raw)?;
        let path = log_dir.join(format!("{}.jsonl", parsed.format("%Y-%m-%d")));
        if !path.exists() {
            println!("Aviso: Nenhum log encontrado para {raw}.");
            println!("   Verifique o formato: YYYY-DD-MM  (ex: 2026-08-06)");
            return Ok(());
        }
        (vec![path], format!("{raw}"))
    } else {
        let files = find_log_files(&log_dir, days);
        (files, format!("últimos {days} dia(s)"))
    };

    if files.is_empty() {
        println!("{} Nenhum log nos últimos {days} dias.", "Aviso:".yellow().bold());
        println!("   Rode primeiro: {}", "activity-tracker start".cyan());
        println!("   Ou coleta manual: {}", "activity-tracker collect".cyan());
        return Ok(());
    }
    println!("{} arquivo(s) de log encontrados", files.len().to_string().cyan());

    let data = aggregate(&files)?;
    let context = build_context(&data);

    if verbose {
        println!("\n{}\n{context}\n{}\n", "--- Contexto enviado ao Ollama ---".dimmed(), "---".dimmed());
    }

    println!("{}", format!("Enviando para Ollama (modelo: {model})...").dimmed());
    let summary = call_ollama(ollama_url, model, &context, lang).await?;

    let border = "━".repeat(47);
    println!("\n{}", border.cyan());
    println!("  {}  {}", "RESUMO DE ATIVIDADES".bold().white(), format!("— {label}").cyan());
    println!("  {}", format!("Modelo: {model}").dimmed());
    println!("{}\n", border.cyan());
    let skin = MadSkin::default();
    skin.print_text(&summary);
    println!("\n{}", border.cyan());

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

fn is_noise_command(cmd: &str) -> bool {
    // Timestamps do HISTTIMEFORMAT: "#digits" ou só dígitos
    let t = cmd.trim();
    let inner = t.strip_prefix('#').unwrap_or(t);
    if inner.chars().all(|c| c.is_ascii_digit()) && !inner.is_empty() {
        return true;
    }
    // Comandos triviais
    let first = t.split_whitespace().next().unwrap_or("");
    matches!(first, "ls" | "cd" | "clear" | "pwd" | "exit" | "history" | "ll" | "la" | "l")
}

fn strip_hostname_prefix(s: &str) -> &str {
    // wmctrl prefixa títulos com o hostname: "hostname título da janela"
    // Detecta se começa com palavra(s) com hífen (padrão de hostname Linux) seguida de espaço
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
        i += 1;
    }
    // Só descarta o prefixo se for longo o suficiente para ser um hostname (>5 chars) e seguido de espaço
    if i > 5 && i < s.len() && bytes[i] == b' ' {
        s[i + 1..].trim()
    } else {
        s
    }
}

fn aggregate(files: &[PathBuf]) -> Result<ActivityData> {
    let mut seen_commands: HashSet<String> = HashSet::new();
    let mut commands: Vec<String> = Vec::new();
    let mut app_counts: HashMap<String, u32> = HashMap::new();
    let mut seen_urls: HashSet<String> = HashSet::new();
    let mut tabs: Vec<(String, String)> = Vec::new();
    // Dedup repos por path, mantendo o commit mais recente (primeiro encontrado = mais recente no JSONL)
    let mut repo_map: HashMap<String, String> = HashMap::new();
    let mut dates: Vec<String> = Vec::new();

    for file in files {
        let date = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        dates.push(date);

        let f = std::fs::File::open(file)?;
        for line in std::io::BufReader::new(f).lines() {
            let line = match line {
                Ok(l) if !l.trim().is_empty() => l,
                _ => continue,
            };

            match serde_json::from_str::<LogEntry>(&line) {
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
                Ok(LogEntry::Context { data }) => {
                    for r in data.git_repos {
                        // Mantém apenas a primeira entrada por repo (mais recente no JSONL)
                        repo_map.entry(r.repo).or_insert(r.last_commit);
                    }
                }
                Err(_) => {}
            }
        }
    }

    let mut top_apps: Vec<(String, u32)> = app_counts.into_iter().collect();
    top_apps.sort_by(|a, b| b.1.cmp(&a.1));
    top_apps.truncate(15);

    tabs.truncate(30);

    let mut repos: Vec<(String, String)> = repo_map.into_iter().collect();
    // Ordenar por data do commit (string ISO, ordenação lexicográfica funciona)
    repos.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(ActivityData {
        dates,
        commands: commands.into_iter().take(150).collect(),
        top_apps,
        tabs,
        repos,
    })
}

/// Constrói o contexto como texto plano — muito mais compacto que JSON pretty-printed.
fn build_context(data: &ActivityData) -> String {
    let mut out = String::new();

    out.push_str(&format!("Período: {}\n\n", data.dates.join(", ")));

    if !data.commands.is_empty() {
        out.push_str("=== COMANDOS DO TERMINAL ===\n");
        for cmd in &data.commands {
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
        for (title, url) in &data.tabs {
            if title.is_empty() || title == url {
                out.push_str(&format!("  {url}\n"));
            } else {
                out.push_str(&format!("  {title}\n"));
            }
        }
        out.push('\n');
    }

    if !data.repos.is_empty() {
        out.push_str("=== REPOSITÓRIOS GIT ===\n");
        for (repo, commit) in &data.repos {
            // Mostrar só o nome do repo, não o path completo
            let name = std::path::Path::new(repo)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(repo);
            out.push_str(&format!("  {name}: {commit}\n"));
        }
        out.push('\n');
    }

    out
}

async fn call_ollama(
    base_url: &str,
    model: &str,
    context: &str,
    lang: &str,
) -> Result<String> {
    let lang_instruction = match lang {
        "pt-br" => "Responda em português brasileiro.",
        "en" => "Answer in English.",
        "es" => "Responde en español.",
        other => &format!("Respond in {other}."),
    };

    let prompt = format!(
        "Você é um assistente que analisa dados de atividade de computador e produz um resumo claro.\n\
         {lang_instruction}\n\n\
         Dados coletados:\n\n\
         {context}\n\
         Produza:\n\
         1. **Resumo Geral**: O que o usuário fez, em 2-3 parágrafos.\n\
         2. **Projetos Identificados**: Projetos ou tarefas em andamento.\n\
         3. **Ferramentas Mais Usadas**: Apps e ferramentas mais utilizados.\n\
         4. **Sites e Pesquisas**: O que foi pesquisado ou lido online.\n\
         5. **Sugestões**: Observações úteis sobre produtividade.\n\n\
         Seja conciso. Não invente informações além dos dados."
    );

    let req = OllamaReq {
        model: model.to_string(),
        prompt,
        stream: false,
        options: OllamaOpts {
            temperature: 0.0,
            num_predict: 2048,
            seed: 42,
        },
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let resp = client
        .post(format!("{base_url}/api/generate"))
        .json(&req)
        .send()
        .await
        .context("Não foi possível conectar ao Ollama. Está rodando? (ollama serve)")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Ollama erro {status}: {body}");
    }

    let ollama: OllamaResp = resp.json().await.context("Erro parseando resposta")?;
    Ok(ollama.response)
}
