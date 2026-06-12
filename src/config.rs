use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    /// Provider: ollama | openai | anthropic | groq | gemini | openrouter
    #[serde(default = "default_provider")]
    pub provider: String,
    pub ollama_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    pub lang: String,
    #[serde(default)]
    pub notion_token: Option<String>,
    #[serde(default)]
    pub notion_page_id: Option<String>,
    #[serde(default)]
    pub machine_name: Option<String>,
    #[serde(default)]
    pub slack_webhook: Option<String>,
    /// Padrões de privacidade: comandos/URLs/títulos que contêm qualquer um desses
    /// termos (case-insensitive) são removidos dos logs antes de salvar.
    #[serde(default)]
    pub blocked_patterns: Vec<String>,
}

fn default_provider() -> String {
    "ollama".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "llama3.1".into(),
            provider: "ollama".into(),
            ollama_url: "http://localhost:11434".into(),
            api_key: None,
            lang: "pt-br".into(),
            notion_token: None,
            notion_page_id: None,
            machine_name: None,
            slack_webhook: None,
            blocked_patterns: Vec::new(),
        }
    }
}

impl Config {
    /// Carrega config de ~/.config/activity-tracker/config.toml ou cria default
    pub fn load() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Erro lendo {}", path.display()))?;
            let cfg: Config =
                toml::from_str(&content).with_context(|| "Erro parseando config.toml")?;
            Ok(cfg)
        } else {
            let cfg = Config::default();
            cfg.save()?;
            Ok(cfg)
        }
    }

    pub fn get_machine_name(&self) -> String {
        if let Some(name) = &self.machine_name {
            return name.clone();
        }
        std::process::Command::new("hostname")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn display(&self) -> String {
        let api_line = if self.provider == "ollama" {
            format!("  api_url:    {}", self.ollama_url)
        } else {
            format!(
                "  api_key:    {}",
                self.api_key
                    .as_deref()
                    .map(|k| format!(
                        "{}…{}",
                        &k[..k.len().min(6)],
                        &k[k.len().saturating_sub(4)..]
                    ))
                    .unwrap_or_else(|| "não configurado".into())
            )
        };
        let blocked_line = if self.blocked_patterns.is_empty() {
            "  bloqueados: nenhum".to_string()
        } else {
            format!(
                "  bloqueados: {} padrão(s): {}",
                self.blocked_patterns.len(),
                self.blocked_patterns.join(", ")
            )
        };
        format!(
            "Configuração atual ({}):\n\n  provider:   {}\n  modelo:     {}\n{}\n  idioma:     {}\n  máquina:    {}\n  notion:     {}\n  slack:      {}\n{}\n  logs:       {}\n  config:     {}",
            config_path().display(),
            self.provider,
            self.model,
            api_line,
            self.lang,
            self.get_machine_name(),
            if self.notion_token.is_some() && self.notion_page_id.is_some() {
                "configurado"
            } else {
                "não configurado"
            },
            if self.slack_webhook.is_some() {
                "configurado"
            } else {
                "não configurado"
            },
            blocked_line,
            log_dir().display(),
            config_path().display(),
        )
    }
}

pub fn base_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("activity-tracker")
}

pub fn config_path() -> PathBuf {
    base_dir().join("config.toml")
}

pub fn log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("activity-tracker")
        .join("logs")
}

pub fn summary_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("activity-tracker")
        .join("summaries")
}

pub fn summary_path(date: chrono::NaiveDate) -> PathBuf {
    summary_dir().join(format!("{}.md", date.format("%Y-%m-%d")))
}
