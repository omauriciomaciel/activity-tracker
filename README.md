# Activity Tracker

Daemon em Rust que captura atividades do sistema (terminal, janelas abertas, abas do Chrome/Brave, repositórios Git) e gera resumos inteligentes via [Ollama](https://ollama.com).

## Instalação

```bash
# Instala o binário em ~/.local/bin
./install.sh
```

Ou compile manualmente:

```bash
cargo build --release
./target/release/activity-tracker --version
```

**Pré-requisitos:**
- **Rust/Cargo** — [rustup.rs](https://rustup.rs)
- **Ollama** — `ollama serve` + `ollama pull llama3.1`
- Opcional: `wmctrl` para captura de janelas X11 (`sudo apt install wmctrl`)

## Uso

```bash
# Iniciar daemon (coleta automática — padrão: a cada 5 min)
activity-tracker start
activity-tracker start --interval 10   # intervalo customizado em minutos

# Coleta manual única
activity-tracker collect

# Resumo dos últimos N dias
activity-tracker summary
activity-tracker summary --days 7
activity-tracker summary --model mistral
activity-tracker summary --lang en
activity-tracker summary --verbose      # exibe dados brutos antes do resumo

# Resumo de uma data específica (formato YYYY-DD-MM)
activity-tracker summary --date 2026-06-06

# Configuração persistente
activity-tracker config set-model llama3.1
activity-tracker config set-url http://localhost:11434
activity-tracker config set-lang pt-br   # pt-br | en | es
activity-tracker config show
```

## O que é capturado

| Fonte | Detalhes |
|---|---|
| **Shell history** | Bash, Zsh e Fish — leitura incremental (só novos comandos desde a última coleta) |
| **Janelas abertas** | `wmctrl` (X11) → `xdotool` → `osascript` (macOS) → `ps` (fallback Linux) |
| **Chrome / Brave** | DevTools Protocol (porta 9222) ou SQLite history DB (últimas 2 horas) |
| **Git** | Último commit de cada repo encontrado até 4 níveis abaixo do home (máx. 15 repos) |

Comandos triviais são filtrados automaticamente (`ls`, `cd`, `clear`, `pwd`, etc.).

## Armazenamento

| Arquivo | Caminho |
|---|---|
| Logs diários (JSONL) | `~/.local/share/activity-tracker/logs/YYYY-MM-DD.jsonl` |
| Configuração | `~/.config/activity-tracker/config.toml` |

Cada linha do `.jsonl` é uma entrada tipada (`shell`, `apps`, `chrome_tabs` ou `context`).

## Configuração padrão

```toml
model      = "llama3.1"
ollama_url = "http://localhost:11434"
lang       = "pt-br"
```

## Resumo gerado

O resumo inclui:

1. **Resumo Geral** — o que foi feito, em 2-3 parágrafos
2. **Projetos Identificados** — tarefas em andamento detectadas
3. **Ferramentas Mais Usadas** — apps e CLIs mais frequentes
4. **Sites e Pesquisas** — o que foi lido ou pesquisado online
5. **Sugestões** — observações de produtividade

## Chrome com DevTools Protocol

Para capturar abas em tempo real (sem depender do histórico SQLite), inicie o Chrome com:

```bash
google-chrome --remote-debugging-port=9222
```
