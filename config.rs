use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub ollama_url: String,
    pub lang: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "llama3.1".into(),
            ollama_url: "http://localhost:11434".into(),
            lang: "pt-br".into(),
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
            let cfg: Config = toml::from_str(&content)
                .with_context(|| "Erro parseando config.toml")?;
            Ok(cfg)
        } else {
            let cfg = Config::default();
            cfg.save()?;
            Ok(cfg)
        }
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
        format!(
            "📋 Configuração atual ({}):\n\n  modelo:     {}\n  ollama_url: {}\n  idioma:     {}\n  logs:       {}\n  config:     {}",
            config_path().display(),
            self.model,
            self.ollama_url,
            self.lang,
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
