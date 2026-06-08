use crate::config;
use anyhow::{Context, Result};
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

    // 1. Encontrar arquivos
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
        println!("Aviso: Nenhum log nos últimos {days} dias.");
        println!("   Rode primeiro: activity-tracker start");
        println!("   Ou coleta manual: activity-tracker collect");
        return Ok(());
    }
    println!("{} arquivo(s) de log encontrados", files.len());

    // 2. Agregar
    let data = aggregate(&files, verbose)?;
    if verbose {
        println!("\nDados agregados:");
        println!("{}", serde_json::to_string_pretty(&data)?);
    }

    // 3. Chamar Ollama
    println!("Enviando para Ollama (modelo: {model})...\n");
    let summary = call_ollama(ollama_url, model, &data, lang).await?;

    println!("-----------------------------------------------");
    println!("  RESUMO DE ATIVIDADES — {label}");
    println!("  Modelo: {model}");
    println!("-----------------------------------------------\n");
    println!("{summary}");
    println!("\n-----------------------------------------------");

    Ok(())
}

/// Parseia entrada no formato YYYY-DD-MM para NaiveDate.
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

fn aggregate(files: &[PathBuf], verbose: bool) -> Result<serde_json::Value> {
    let mut all_commands: Vec<String> = Vec::new();
    let mut app_counts: HashMap<String, u32> = HashMap::new();
    let mut tabs: Vec<serde_json::Value> = Vec::new();
    let mut repos: Vec<serde_json::Value> = Vec::new();
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
                Ok(LogEntry::Shell { commands }) => {
                    all_commands.extend(commands);
                }
                Ok(LogEntry::Apps { windows }) => {
                    for w in windows {
                        let w = w.trim().to_string();
                        if !w.is_empty() {
                            *app_counts.entry(w).or_default() += 1;
                        }
                    }
                }
                Ok(LogEntry::ChromeTabs { tabs: t }) => {
                    for tab in t {
                        tabs.push(serde_json::json!({
                            "title": tab.title,
                            "url": tab.url
                        }));
                    }
                }
                Ok(LogEntry::Context { data }) => {
                    for r in data.git_repos {
                        repos.push(serde_json::json!({
                            "repo": r.repo,
                            "last_commit": r.last_commit
                        }));
                    }
                }
                Err(e) => {
                    if verbose {
                        eprintln!("Aviso: ignorando linha: {e}");
                    }
                }
            }
        }
    }

    // Dedup commands
    all_commands.dedup();
    let commands: Vec<&String> = all_commands.iter().take(200).collect();

    // Top apps
    let mut top_apps: Vec<_> = app_counts.into_iter().collect();
    top_apps.sort_by(|a, b| b.1.cmp(&a.1));
    let top_apps: Vec<serde_json::Value> = top_apps
        .into_iter()
        .take(20)
        .map(|(name, count)| serde_json::json!({"app": name, "count": count}))
        .collect();

    // Dedup tabs by URL
    let mut seen = HashSet::new();
    let unique_tabs: Vec<&serde_json::Value> = tabs
        .iter()
        .filter(|t| {
            let url = t["url"].as_str().unwrap_or("");
            seen.insert(url.to_string())
        })
        .take(50)
        .collect();

    Ok(serde_json::json!({
        "periodo": dates,
        "comandos_terminal": commands,
        "aplicativos_mais_usados": top_apps,
        "sites_visitados": unique_tabs,
        "repositorios_git": repos,
    }))
}

async fn call_ollama(
    base_url: &str,
    model: &str,
    data: &serde_json::Value,
    lang: &str,
) -> Result<String> {
    let lang_instruction = match lang {
        "pt-br" => "Responda em português brasileiro.",
        "en" => "Answer in English.",
        "es" => "Responde en español.",
        other => &format!("Respond in {other}."),
    };

    let prompt = format!(
r#"Você é um assistente que analisa dados de atividade de computador e produz um resumo claro.

{lang_instruction}

Dados coletados:

{data}

Produza:

1. **Resumo Geral**: O que o usuário fez, em 2-3 parágrafos.
2. **Projetos Identificados**: Projetos ou tarefas em andamento.
3. **Ferramentas Mais Usadas**: Apps e ferramentas mais utilizados.
4. **Sites e Pesquisas**: O que foi pesquisado ou lido online.
5. **Sugestões**: Observações úteis sobre produtividade.

Seja conciso. Não invente informações além dos dados."#,
        data = serde_json::to_string_pretty(data)?
    );

    let req = OllamaReq {
        model: model.to_string(),
        prompt,
        stream: false,
        options: OllamaOpts {
            temperature: 0.3,
            num_predict: 2048,
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
