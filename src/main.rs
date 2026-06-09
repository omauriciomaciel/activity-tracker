mod collector;
mod config;
mod daemon;
mod summarizer;
mod notion;
mod updater;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "activity-tracker")]
#[command(about = "Captura atividades e resume com Ollama — tudo em Rust")]
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

    /// Gera um resumo das atividades recentes via Ollama
    Summary {
        /// Quantos dias para trás resumir (ignorado se --date for usado)
        #[arg(short, long, default_value = "3")]
        days: u32,

        /// Data específica no formato YYYY-DD-MM (ex: 2026-08-06)
        #[arg(long)]
        date: Option<String>,

        /// Modelo do Ollama (sobrescreve o padrão salvo)
        #[arg(short, long)]
        model: Option<String>,

        /// URL do Ollama
        #[arg(long)]
        ollama_url: Option<String>,

        /// Idioma do resumo
        #[arg(long)]
        lang: Option<String>,

        /// Resumir apenas o dia de hoje (atalho para --days 1)
        #[arg(long)]
        today: bool,

        /// Enviar resumo ao Notion após gerar
        #[arg(long)]
        send_notion: bool,

        /// Mostrar dados brutos
        #[arg(long)]
        verbose: bool,
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
    /// Define o modelo padrão do Ollama
    SetModel {
        /// Nome do modelo (ex: llama3.1, mistral, gemma2)
        name: String,
    },
    /// Define a URL padrão do Ollama
    SetUrl {
        /// URL (ex: http://localhost:11434)
        url: String,
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
            let entry_count = collector::collect_all(&log_dir)?;
            println!("Coleta concluída — {entry_count} entradas salvas");
        }

        Commands::CleanLogs => {
            let log_dir = config::log_dir();
            let removed = collector::clean_all_logs(&log_dir)?;
            println!("Logs limpos — {removed} entradas removidas");
        }

        Commands::Update => {
            updater::run()?;
        }

        Commands::Summary {
            mut days,
            mut date,
            model,
            ollama_url,
            lang,
            today,
            send_notion,
            verbose,
        } => {
            if today {
                days = 1;
                date = None;
            }
            let model = model.unwrap_or_else(|| cfg.model.clone());
            let url = ollama_url.unwrap_or_else(|| cfg.ollama_url.clone());
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

let machine = cfg.get_machine_name();
            summarizer::run(summarizer::RunOptions {
                days,
                date: date.as_deref(),
                model: &model,
                ollama_url: &url,
                lang: &lang,
                machine_name: &machine,
                notion: notion.as_ref().map(|(t, p)| (t.as_str(), p.as_str())),
                verbose,
            }).await?;
        }

        Commands::Config { action } => match action {
            ConfigAction::SetModel { name } => {
                cfg.model = name.clone();
                cfg.save()?;
                println!("Modelo padrão: {name}");
            }
            ConfigAction::SetUrl { url } => {
                cfg.ollama_url = url.clone();
                cfg.save()?;
                println!("URL do Ollama: {url}");
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
            ConfigAction::Show => {
                println!("{}", cfg.display());
            }
        },
    }

    Ok(())
}
