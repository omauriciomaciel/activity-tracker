use crate::config;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::PathBuf;
use termimad::MadSkin;

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
    #[serde(default)]
    commits: Vec<String>,
    // backward compat: old logs have last_commit string
    #[serde(default)]
    last_commit: String,
}

// ─── Dados agregados ─────────────────────────────

pub struct ActivityData {
    pub dates: Vec<String>,
    pub commands: Vec<String>,
    pub top_apps: Vec<(String, u32)>,
    pub tabs: Vec<(String, String)>,       // (title, url)
    pub repos: Vec<(String, Vec<String>)>, // (path, commits)
}

// ─── Structs de API ──────────────────────────────

// Ollama
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

// OpenAI-compatible (OpenAI, Groq, OpenRouter)
#[derive(Serialize)]
struct OpenAiReq {
    model: String,
    messages: Vec<OpenAiMsg>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct OpenAiMsg {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResp {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMsgContent,
}

#[derive(Deserialize)]
struct OpenAiMsgContent {
    content: String,
}

// Anthropic
#[derive(Serialize)]
struct AnthropicReq {
    model: String,
    max_tokens: u32,
    messages: Vec<OpenAiMsg>,
}

#[derive(Deserialize)]
struct AnthropicResp {
    content: Vec<AnthropicBlock>,
}

#[derive(Deserialize)]
struct AnthropicBlock {
    #[serde(rename = "type")]
    kind: String,
    text: String,
}

// Gemini
#[derive(Serialize)]
struct GeminiReq {
    contents: Vec<GeminiContent>,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResp {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResp,
}

#[derive(Deserialize)]
struct GeminiContentResp {
    parts: Vec<GeminiPartResp>,
}

#[derive(Deserialize)]
struct GeminiPartResp {
    text: String,
}

// ─── Lógica principal ───────────────────────────

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
    pub search: Option<&'a str>,
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
        search,
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

    let data = aggregate(&files)?;
    let data = condense_for_period(data, days);
    let data = if let Some(query) = search {
        let filtered = filter_by_search(data, query);
        print_search_results(&filtered, query);
        let empty = filtered.commands.is_empty()
            && filtered.top_apps.is_empty()
            && filtered.tabs.is_empty()
            && filtered.repos.is_empty();
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
            build_context(&data),
            crate::projects::format_context(stats, days)
        )
    } else {
        build_context(&data)
    };

    println!(
        "{}",
        format!("Enviando para {provider} (modelo: {model})...").dimmed()
    );
    let summary = call_llm(provider, ollama_url, api_key, model, &context, lang).await?;

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

    if let Some((token, page_id)) = notion {
        let date_label = match data.dates.as_slice() {
            [] => chrono::Local::now().format("%Y-%m-%d").to_string(),
            [single] => single.clone(),
            dates => format!("{} a {}", dates.last().unwrap(), dates.first().unwrap()),
        };
        let title = format!("{date_label} — {machine_name}");
        print!("{}", "Enviando ao Notion...".dimmed());
        match crate::notion::send_page(token, page_id, &title, &summary).await {
            Ok(url) => println!(" {}", url.cyan()),
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
        });
    }
    aggregate(&[path])
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
    matches!(
        first,
        "ls" | "cd" | "clear" | "pwd" | "exit" | "history" | "ll" | "la" | "l"
    )
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
    let mut repo_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut dates: Vec<String> = Vec::new();

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
                Ok(LogEntry::Context { data }) => {
                    for r in data.git_repos {
                        // Merge commits list (new format) or fall back to last_commit (old format)
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
    // Sort by most recent commit across all commits for the repo
    repos.sort_by(|a, b| {
        let latest_a = a.1.iter().max().map(|s| s.as_str()).unwrap_or("");
        let latest_b = b.1.iter().max().map(|s| s.as_str()).unwrap_or("");
        latest_b.cmp(latest_a)
    });

    Ok(ActivityData {
        dates,
        commands: commands.into_iter().take(150).collect(),
        top_apps,
        tabs,
        repos,
    })
}

fn condense_for_period(mut data: ActivityData, days: u32) -> ActivityData {
    if days > 6 {
        let cmd_limit = if days > 20 { 60 } else { 100 };
        data.commands.truncate(cmd_limit);
        data.top_apps.truncate(10);
        data.tabs.truncate(20);
    }
    data
}

fn filter_by_search(data: ActivityData, query: &str) -> ActivityData {
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
    ActivityData {
        dates: data.dates,
        commands,
        top_apps,
        tabs,
        repos,
    }
}

fn print_search_results(data: &ActivityData, query: &str) {
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
        // KEY=VALUE
        if let Some(eq_pos) = token.find('=') {
            let key = token[..eq_pos].trim_start_matches('-').to_lowercase();
            if SENSITIVE.iter().any(|s| key.contains(s)) && eq_pos + 1 < token.len() {
                out.push(format!("{}=[REDACTED]", &token[..eq_pos]));
                continue;
            }
        }
        // --flag <value> (flag name contains sensitive keyword, value is next token)
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

/// Constrói o contexto como texto plano — muito mais compacto que JSON pretty-printed.
pub(crate) fn build_context(data: &ActivityData) -> String {
    let mut out = String::new();

    out.push_str(&format!("Período: {}\n\n", data.dates.join(", ")));

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

fn validate_url(url: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(url).with_context(|| format!("URL inválida: '{url}'"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        s => anyhow::bail!("URL: esquema '{s}' não permitido (use http ou https)"),
    }
    if let Some(host) = parsed.host_str()
        && matches!(
            host,
            "169.254.169.254" | "metadata.google.internal" | "metadata.google"
        )
    {
        anyhow::bail!("URL: host '{host}' não permitido");
    }
    Ok(())
}

fn build_prompt(context: &str, lang: &str) -> String {
    let lang_instruction = match lang {
        "pt-br" => "Responda em português brasileiro.",
        "en" => "Answer in English.",
        "es" => "Responde en español.",
        other => {
            return format!(
                "Respond in {other}.\n\nDados coletados:\n\n{context}\nProduza:\n1. **Resumo Geral**: O que o usuário fez, em 2-3 parágrafos.\n2. **Projetos Identificados**: Projetos ou tarefas em andamento.\n3. **Ferramentas Mais Usadas**: Apps e ferramentas mais utilizados.\n4. **Sites e Pesquisas**: O que foi pesquisado ou lido online.\n5. **Sugestões**: Observações úteis sobre produtividade.\n\nSeja conciso. Não invente informações além dos dados."
            );
        }
    };
    format!(
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
    )
}

pub(crate) async fn call_llm(
    provider: &str,
    ollama_url: &str,
    api_key: Option<&str>,
    model: &str,
    context: &str,
    lang: &str,
) -> Result<String> {
    let prompt = build_prompt(context, lang);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    match provider {
        "ollama" => call_ollama(&client, ollama_url, model, &prompt).await,
        "openai" => {
            call_openai_compat(
                &client,
                "https://api.openai.com",
                api_key,
                model,
                &prompt,
                "openai",
            )
            .await
        }
        "groq" => {
            call_openai_compat(
                &client,
                "https://api.groq.com/openai",
                api_key,
                model,
                &prompt,
                "groq",
            )
            .await
        }
        "openrouter" => {
            call_openai_compat(
                &client,
                "https://openrouter.ai/api",
                api_key,
                model,
                &prompt,
                "openrouter",
            )
            .await
        }
        "anthropic" => call_anthropic(&client, api_key, model, &prompt).await,
        "gemini" => call_gemini(&client, api_key, model, &prompt).await,
        other => anyhow::bail!(
            "Provider '{other}' não suportado. Use: ollama, openai, anthropic, groq, gemini, openrouter"
        ),
    }
}

async fn call_ollama(
    client: &reqwest::Client,
    base_url: &str,
    model: &str,
    prompt: &str,
) -> Result<String> {
    validate_url(base_url)?;
    let req = OllamaReq {
        model: model.to_string(),
        prompt: prompt.to_string(),
        stream: false,
        options: OllamaOpts {
            temperature: 0.0,
            num_predict: 2048,
            seed: 42,
        },
    };

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

    let r: OllamaResp = resp
        .json()
        .await
        .context("Erro parseando resposta Ollama")?;
    Ok(r.response)
}

async fn call_openai_compat(
    client: &reqwest::Client,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    prompt: &str,
    provider_name: &str,
) -> Result<String> {
    let key = api_key.ok_or_else(|| anyhow::anyhow!("api_key não configurada para {provider_name}. Execute: activity-tracker config set-api-key <key>"))?;
    let req = OpenAiReq {
        model: model.to_string(),
        messages: vec![OpenAiMsg {
            role: "user".into(),
            content: prompt.to_string(),
        }],
        temperature: 0.0,
        max_tokens: 2048,
    };

    let resp = client
        .post(format!("{base_url}/v1/chat/completions"))
        .bearer_auth(key)
        .json(&req)
        .send()
        .await
        .with_context(|| format!("Erro conectando ao {provider_name}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("{provider_name} erro {status}: {body}");
    }

    let r: OpenAiResp = resp
        .json()
        .await
        .with_context(|| format!("Erro parseando resposta {provider_name}"))?;
    r.choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| anyhow::anyhow!("{provider_name}: resposta vazia"))
}

async fn call_anthropic(
    client: &reqwest::Client,
    api_key: Option<&str>,
    model: &str,
    prompt: &str,
) -> Result<String> {
    let key = api_key.ok_or_else(|| anyhow::anyhow!("api_key não configurada para anthropic. Execute: activity-tracker config set-api-key <key>"))?;
    let req = AnthropicReq {
        model: model.to_string(),
        max_tokens: 2048,
        messages: vec![OpenAiMsg {
            role: "user".into(),
            content: prompt.to_string(),
        }],
    };

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .await
        .context("Erro conectando ao Anthropic")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic erro {status}: {body}");
    }

    let r: AnthropicResp = resp
        .json()
        .await
        .context("Erro parseando resposta Anthropic")?;
    r.content
        .into_iter()
        .find(|b| b.kind == "text")
        .map(|b| b.text)
        .ok_or_else(|| anyhow::anyhow!("Anthropic: resposta vazia"))
}

async fn call_gemini(
    client: &reqwest::Client,
    api_key: Option<&str>,
    model: &str,
    prompt: &str,
) -> Result<String> {
    let key = api_key.ok_or_else(|| anyhow::anyhow!("api_key não configurada para gemini. Execute: activity-tracker config set-api-key <key>"))?;
    let req = GeminiReq {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart {
                text: prompt.to_string(),
            }],
        }],
    };

    let resp = client
        .post(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
        ))
        .json(&req)
        .send()
        .await
        .context("Erro conectando ao Gemini")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Gemini erro {status}: {body}");
    }

    let r: GeminiResp = resp
        .json()
        .await
        .context("Erro parseando resposta Gemini")?;
    r.candidates
        .into_iter()
        .next()
        .and_then(|c| c.content.parts.into_iter().next())
        .map(|p| p.text)
        .ok_or_else(|| anyhow::anyhow!("Gemini: resposta vazia"))
}

// ─── Export ─────────────────────────────────────────────────────────────────

pub fn export_cmd(days: u32, date: Option<&str>, format: &str, output: Option<&str>) -> Result<()> {
    let log_dir = config::log_dir();
    let files = if let Some(raw) = date {
        let parsed = parse_date(raw)?;
        let path = log_dir.join(format!("{}.jsonl", parsed.format("%Y-%m-%d")));
        if path.exists() {
            vec![path]
        } else {
            eprintln!("Aviso: nenhum log para a data {raw}.");
            return Ok(());
        }
    } else {
        find_log_files(&log_dir, days)
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
