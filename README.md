# Activity Tracker

Daemon em Rust que captura atividades do sistema (terminal, janelas abertas, abas do Chrome/Brave, repositórios Git) e gera resumos inteligentes via [Ollama](https://ollama.com).

## Instalação

```bash
./install.sh
```

O script compila o binário, instala em `~/.local/bin` e configura um **serviço systemd de usuário** que inicia automaticamente no login.

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
# Daemon em background
activity-tracker start                     # inicia em background
activity-tracker start --interval 10       # intervalo customizado em minutos
activity-tracker stop                      # para o daemon
activity-tracker status                    # verifica se está rodando

# Coleta manual única
activity-tracker collect

# Resumo dos últimos N dias
activity-tracker summary
activity-tracker summary --days 7
activity-tracker summary --model mistral
activity-tracker summary --lang en
activity-tracker summary --verbose         # exibe dados brutos antes do resumo

# Resumo de uma data específica (formato YYYY-DD-MM)
activity-tracker summary --date 2026-06-06

# Configuração persistente
activity-tracker config set-model llama3.1
activity-tracker config set-url http://localhost:11434
activity-tracker config set-lang pt-br     # pt-br | en | es
activity-tracker config show
```

## Autostart no login

O `install.sh` cria e habilita um serviço systemd de usuário automaticamente. Para gerenciá-lo:

```bash
systemctl --user status activity-tracker   # status do serviço
systemctl --user stop activity-tracker     # parar
systemctl --user start activity-tracker    # iniciar
systemctl --user disable activity-tracker  # remover do autostart

journalctl --user -u activity-tracker -f   # logs em tempo real
```

> Para o autostart funcionar após reinicialização sem sessão gráfica ativa, habilite o linger:
> ```bash
> loginctl enable-linger $USER
> ```

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
| PID do daemon | `~/.local/share/activity-tracker/daemon.pid` |
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
