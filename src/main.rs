mod collector;
mod config;
mod daemon;
mod notion;
mod projects;
mod slack;
mod summarizer;
mod tui;
mod updater;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "activity-tracker")]
#[command(about = "Captura atividades e resume com LLM — tudo em Rust")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inicia o daemon em background (ou em primeiro plano com --foreground)
    Start {
        /// Intervalo de coleta em minutos
        #[arg(short, long, default_value = "5")]
        interval: u64,

        /// Mantém processo em primeiro plano (usado pelo systemd)
        #[arg(long)]
        foreground: bool,
    },

    /// Para o daemon em background
    Stop,

    /// Mostra status do daemon
    Status,

    /// Executa uma única coleta manual
    Collect,

    /// Gera um resumo das atividades recentes via LLM
    Summary {
        /// Quantos dias para trás resumir (ignorado se --date for usado)
        #[arg(short, long, default_value = "3")]
        days: u32,

        /// Data específica no formato YYYY-DD-MM (ex: 2026-08-06)
        #[arg(long)]
        date: Option<String>,

        /// Modelo a usar (sobrescreve o padrão salvo)
        #[arg(short, long)]
        model: Option<String>,

        /// Provider de LLM: ollama, openai, anthropic, groq, gemini, openrouter
        #[arg(long)]
        provider: Option<String>,

        /// URL do Ollama (apenas para provider=ollama)
        #[arg(long)]
        ollama_url: Option<String>,

        /// API key do provider (sobrescreve o padrão salvo)
        #[arg(long)]
        api_key: Option<String>,

        /// Idioma do resumo
        #[arg(long)]
        lang: Option<String>,

        /// Resumir apenas o dia de hoje (atalho para --days 1)
        #[arg(long)]
        today: bool,

        /// Resumir os últimos 7 dias (atalho para --days 7)
        #[arg(long)]
        week: bool,

        /// Resumir os últimos 30 dias (atalho para --days 30)
        #[arg(long)]
        month: bool,

        /// Filtrar logs pelo termo antes de resumir (ex: --search docker)
        #[arg(long)]
        search: Option<String>,

        /// Enviar resumo ao Notion após gerar
        #[arg(long)]
        send_notion: bool,

        /// Enviar resumo ao Slack via webhook configurado
        #[arg(long)]
        send_slack: bool,
    },

    /// Exporta logs de atividade em CSV ou JSON
    Export {
        /// Número de dias para exportar
        #[arg(short, long, default_value = "1")]
        days: u32,

        /// Data específica no formato YYYY-DD-MM (ex: 2026-08-06)
        #[arg(long)]
        date: Option<String>,

        /// Formato de saída: csv, json
        #[arg(long, default_value = "csv")]
        format: String,

        /// Arquivo de saída (padrão: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// TUI interativa: navegar dias, ver atividades brutas, gerar resumo
    Tui {
        /// Modelo a usar (sobrescreve o padrão salvo)
        #[arg(short, long)]
        model: Option<String>,

        /// Provider de LLM: ollama, openai, anthropic, groq, gemini, openrouter
        #[arg(long)]
        provider: Option<String>,

        /// URL do Ollama (apenas para provider=ollama)
        #[arg(long)]
        ollama_url: Option<String>,

        /// API key do provider (sobrescreve o padrão salvo)
        #[arg(long)]
        api_key: Option<String>,

        /// Idioma do resumo
        #[arg(long)]
        lang: Option<String>,
    },

    /// Adiciona uma anotação manual ao log do dia (ex: "reunião de planning")
    Tag {
        /// Texto da anotação (se omitido, lista as tags do dia)
        label: Option<String>,

        /// Lista as tags em vez de adicionar
        #[arg(short, long)]
        list: bool,

        /// Remove a tag com o texto fornecido
        #[arg(short, long)]
        delete: bool,

        /// Data no formato YYYY-DD-MM (padrão: hoje)
        #[arg(long)]
        date: Option<String>,
    },

    /// Remove entradas fora da data de cada arquivo de log
    CleanLogs,

    /// Atualiza o binário via git pull + cargo build
    Update,

    /// Gerencia configurações persistentes
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Define o modelo padrão
    SetModel {
        /// Nome do modelo (ex: llama3.1, gpt-4o-mini, claude-haiku-4-5-20251001)
        name: String,
    },
    /// Define o provider de LLM padrão
    SetProvider {
        /// Provider: ollama, openai, anthropic, groq, gemini, openrouter
        name: String,
    },
    /// Define a URL padrão do Ollama
    SetUrl {
        /// URL (ex: http://localhost:11434)
        url: String,
    },
    /// Define a API key para providers cloud (OpenAI, Anthropic, Groq, Gemini, OpenRouter)
    SetApiKey {
        /// API key do provider
        key: String,
    },
    /// Define o idioma padrão do resumo
    SetLang {
        /// Código (ex: pt-br, en, es)
        lang: String,
    },
    /// Define o nome desta máquina (aparece no título das notas do Notion)
    SetMachineName {
        /// Nome da máquina (ex: "MacBook Pro", "servidor-casa")
        name: String,
    },
    /// Define o token da integração do Notion
    SetNotionToken {
        /// Token no formato secret_xxx (em notion.com/my-integrations)
        token: String,
    },
    /// Define o ID da página pai no Notion
    SetNotionPage {
        /// ID da página (URL da página → copiar ID após o último /)
        page_id: String,
    },
    /// Define o webhook URL do Slack para envio de resumos
    SetSlackWebhook {
        /// Webhook URL (em api.slack.com/apps → Incoming Webhooks)
        url: String,
    },
    /// Adiciona um padrão de privacidade (comandos/URLs/títulos que o contenham serão bloqueados)
    AddBlock {
        /// Termo a bloquear (ex: "senha", "postgres://", "meusite.com")
        pattern: String,
    },
    /// Remove um padrão de privacidade
    RemoveBlock {
        /// Termo a remover da lista de bloqueados
        pattern: String,
    },
    /// Lista os padrões de privacidade ativos
    ListBlocks,
    /// Define um prompt customizado para o LLM. Use {context} para os dados e {lang} para o idioma.
    SetPrompt {
        /// Template do prompt. Ex: "Analise as atividades. {lang}\n\n{context}\n\nFoque em produtividade."
        template: String,
    },
    /// Remove o prompt customizado (volta ao padrão)
    ClearPrompt,
    /// Mostra a configuração atual
    Show,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut cfg = config::Config::load()?;

    match cli.command {
        Commands::Start {
            interval,
            foreground,
        } => {
            daemon::run(interval, foreground).await?;
        }

        Commands::Stop => {
            daemon::stop()?;
        }

        Commands::Status => {
            daemon::status()?;
        }

        Commands::Collect => {
            let log_dir = config::log_dir();
            let entry_count = collector::collect_all(&log_dir, &cfg.blocked_patterns, &cfg.ignored_git_paths)?;
            println!("Coleta concluída — {entry_count} entradas salvas");
        }

        Commands::Tag { label, list, delete, date } => {
            let log_dir = config::log_dir();

            let target_date = if let Some(ref raw) = date {
                Some(
                    chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                        .with_context(|| format!("Data inválida: '{raw}'. Use YYYY-MM-DD (ex: 2026-08-06)"))?,
                )
            } else {
                None
            };

            if delete {
                if let Some(lbl) = label {
                    collector::delete_tag(&log_dir, &lbl, target_date)?;
                    let when = target_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
                    println!("Tag removida em {when}: {lbl}");
                } else {
                    eprintln!("Erro: forneça o texto da tag para deletar. Ex: at tag --delete \"reunião\"");
                    std::process::exit(1);
                }
            } else if list || label.is_none() {
                // Listar tags do dia
                let d = target_date.unwrap_or_else(|| chrono::Local::now().date_naive());
                let data = summarizer::load_for_date(d)?;
                if data.tags.is_empty() {
                    println!("Nenhuma tag para {}.", d.format("%Y-%m-%d"));
                } else {
                    println!("Tags para {}:", d.format("%Y-%m-%d"));
                    for (hour, lbl) in &data.tags {
                        println!("  {hour}  {lbl}");
                    }
                }
            } else if let Some(lbl) = label {
                collector::write_tag(&log_dir, &lbl, target_date)?;
                let when = target_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
                println!("Tag adicionada em {when}: {lbl}");
            }
        }

        Commands::CleanLogs => {
            let log_dir = config::log_dir();
            let removed = collector::clean_all_logs(&log_dir)?;
            let purged = collector::purge_ignored_git_repos(&log_dir, &cfg.ignored_git_paths)?;
            println!("Logs limpos — {removed} entradas de data removidas, {purged} repos git ignorados removidos");
        }

        Commands::Update => {
            updater::run()?;
        }

        Commands::Export {
            days,
            date,
            format,
            output,
        } => {
            summarizer::export_cmd(days, date.as_deref(), &format, output.as_deref())?;
        }

        Commands::Tui {
            model,
            provider,
            ollama_url,
            api_key,
            lang,
        } => {
            let model = model.unwrap_or_else(|| cfg.model.clone());
            let provider = provider.unwrap_or_else(|| cfg.provider.clone());
            let url = ollama_url.unwrap_or_else(|| cfg.ollama_url.clone());
            let api_key = api_key.or_else(|| cfg.api_key.clone());
            let lang = lang.unwrap_or_else(|| cfg.lang.clone());
            tui::run(tui::TuiOptions {
                model,
                provider,
                ollama_url: url,
                api_key,
                lang,
            })
            .await?;
        }

        Commands::Summary {
            mut days,
            mut date,
            model,
            provider,
            ollama_url,
            api_key,
            lang,
            today,
            week,
            month,
            search,
            send_notion,
            send_slack,
        } => {
            if today {
                days = 1;
                date = None;
            } else if week {
                days = 7;
                date = None;
            } else if month {
                days = 30;
                date = None;
            }
            let model = model.unwrap_or_else(|| cfg.model.clone());
            let provider = provider.unwrap_or_else(|| cfg.provider.clone());
            let url = ollama_url.unwrap_or_else(|| cfg.ollama_url.clone());
            let api_key = api_key.or_else(|| cfg.api_key.clone());
            let lang = lang.unwrap_or_else(|| cfg.lang.clone());
            let notion = if send_notion {
                match (&cfg.notion_token, &cfg.notion_page_id) {
                    (Some(t), Some(p)) => Some((t.clone(), p.clone())),
                    _ => {
                        eprintln!("Erro: Notion não configurado. Execute:");
                        eprintln!("  activity-tracker config set-notion-token secret_xxx");
                        eprintln!("  activity-tracker config set-notion-page <page_id>");
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            let slack = if send_slack {
                match &cfg.slack_webhook {
                    Some(url) => Some(url.clone()),
                    None => {
                        eprintln!("Erro: Slack não configurado. Execute:");
                        eprintln!(
                            "  activity-tracker config set-slack-webhook https://hooks.slack.com/..."
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            let machine = cfg.get_machine_name();
            summarizer::run(summarizer::RunOptions {
                days,
                date: date.as_deref(),
                model: &model,
                provider: &provider,
                ollama_url: &url,
                api_key: api_key.as_deref(),
                lang: &lang,
                machine_name: &machine,
                notion: notion.as_ref().map(|(t, p)| (t.as_str(), p.as_str())),
                slack: slack.as_deref(),
                search: search.as_deref(),
                custom_prompt: cfg.custom_prompt.as_deref(),
            })
            .await?;
        }

        Commands::Config { action } => match action {
            ConfigAction::SetModel { name } => {
                cfg.model = name.clone();
                cfg.save()?;
                println!("Modelo padrão: {name}");
            }
            ConfigAction::SetProvider { name } => {
                let valid = [
                    "ollama",
                    "openai",
                    "anthropic",
                    "groq",
                    "gemini",
                    "openrouter",
                ];
                if !valid.contains(&name.as_str()) {
                    eprintln!("Provider inválido: {name}");
                    eprintln!("Providers suportados: {}", valid.join(", "));
                    std::process::exit(1);
                }
                cfg.provider = name.clone();
                cfg.save()?;
                println!("Provider: {name}");
            }
            ConfigAction::SetUrl { url } => {
                cfg.ollama_url = url.clone();
                cfg.save()?;
                println!("URL do Ollama: {url}");
            }
            ConfigAction::SetApiKey { key } => {
                cfg.api_key = Some(key);
                cfg.save()?;
                println!("API key salva");
            }
            ConfigAction::SetLang { lang } => {
                cfg.lang = lang.clone();
                cfg.save()?;
                println!("Idioma: {lang}");
            }
            ConfigAction::SetMachineName { name } => {
                cfg.machine_name = Some(name.clone());
                cfg.save()?;
                println!("Nome da máquina: {name}");
            }
            ConfigAction::SetNotionToken { token } => {
                cfg.notion_token = Some(token.clone());
                cfg.save()?;
                println!("Notion token salvo");
            }
            ConfigAction::SetNotionPage { page_id } => {
                cfg.notion_page_id = Some(page_id.clone());
                cfg.save()?;
                println!("Notion page ID salvo: {page_id}");
            }
            ConfigAction::SetSlackWebhook { url } => {
                cfg.slack_webhook = Some(url.clone());
                cfg.save()?;
                println!("Slack webhook salvo");
            }
            ConfigAction::AddBlock { pattern } => {
                let lower = pattern.to_lowercase();
                if cfg
                    .blocked_patterns
                    .iter()
                    .any(|p| p.to_lowercase() == lower)
                {
                    println!("Padrão já existe: {pattern}");
                } else {
                    cfg.blocked_patterns.push(pattern.clone());
                    cfg.save()?;
                    println!("Padrão adicionado: {pattern}");
                    println!(
                        "Total: {} padrão(s) bloqueado(s)",
                        cfg.blocked_patterns.len()
                    );
                }
            }
            ConfigAction::RemoveBlock { pattern } => {
                let lower = pattern.to_lowercase();
                let before = cfg.blocked_patterns.len();
                cfg.blocked_patterns.retain(|p| p.to_lowercase() != lower);
                if cfg.blocked_patterns.len() < before {
                    cfg.save()?;
                    println!("Padrão removido: {pattern}");
                } else {
                    println!("Padrão não encontrado: {pattern}");
                }
            }
            ConfigAction::ListBlocks => {
                if cfg.blocked_patterns.is_empty() {
                    println!("Nenhum padrão de privacidade configurado.");
                } else {
                    println!("{} padrão(s) bloqueado(s):", cfg.blocked_patterns.len());
                    for p in &cfg.blocked_patterns {
                        println!("  - {p}");
                    }
                }
            }
            ConfigAction::SetPrompt { template } => {
                cfg.custom_prompt = Some(template.clone());
                cfg.save()?;
                println!("Prompt customizado salvo.");
                println!("  Placeholders disponíveis: {{context}}, {{lang}}");
                if !template.contains("{context}") {
                    println!("  Aviso: {{context}} não encontrado — dados serão anexados ao final.");
                }
            }
            ConfigAction::ClearPrompt => {
                cfg.custom_prompt = None;
                cfg.save()?;
                println!("Prompt customizado removido. Usando o prompt padrão.");
            }
            ConfigAction::Show => {
                println!("{}", cfg.display());
            }
        },
    }

    Ok(())
}
