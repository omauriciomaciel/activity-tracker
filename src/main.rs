mod collector;
mod config;
mod daemon;
mod summarizer;
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
            days,
            date,
            model,
            ollama_url,
            lang,
            verbose,
        } => {
            let model = model.unwrap_or_else(|| cfg.model.clone());
            let url = ollama_url.unwrap_or_else(|| cfg.ollama_url.clone());
            let lang = lang.unwrap_or_else(|| cfg.lang.clone());

            summarizer::run(days, date.as_deref(), &model, &url, &lang, verbose).await?;
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
            ConfigAction::Show => {
                println!("{}", cfg.display());
            }
        },
    }

    Ok(())
}
