# Activity Tracker

Daemon em Rust que captura atividades do sistema (terminal, janelas abertas, abas do Chrome/Brave, repositórios Git) e gera resumos inteligentes via LLM. Suporta Ollama (local), OpenAI, Anthropic, Groq, Gemini e OpenRouter. Opcionalmente envia o resumo como nota para o [Notion](https://notion.so).

## Instalação

```bash
curl -fsSL https://raw.githubusercontent.com/omauriciomaciel/activity-tracker/main/install.sh | sh
```

O script detecta o OS/arquitetura, baixa o binário pré-compilado da última release do GitHub, instala em `~/.local/bin`, configura o autostart e adiciona aliases ao shell:

- **macOS** — cria um LaunchAgent em `~/Library/LaunchAgents/` e abre as telas de permissão necessárias
- **Linux (systemd)** — cria e ativa um serviço systemd de usuário

Após a instalação, dois aliases ficam disponíveis:

```
at   →  activity-tracker
ats  →  activity-tracker summary
```

Instalar em diretório customizado:

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/omauriciomaciel/activity-tracker/main/install.sh | sh
```

Ou compilar a partir do fonte:

```bash
cargo build --release
./target/release/activity-tracker --version
```

**Pré-requisitos:**
- **LLM** — Ollama local (`ollama serve`) ou API key de um provider cloud
- Opcional: `wmctrl` para captura de janelas X11 (`sudo apt install wmctrl`)

## Providers de LLM

| Provider | Modelos exemplo | Requer |
|---|---|---|
| `ollama` | llama3.1, mistral, gemma2 | Ollama rodando localmente |
| `openai` | gpt-4o, gpt-4o-mini | API key OpenAI |
| `anthropic` | claude-sonnet-4-6, claude-haiku-4-5-20251001 | API key Anthropic |
| `groq` | llama-3.1-8b-instant, mixtral-8x7b | API key Groq |
| `gemini` | gemini-2.0-flash, gemini-1.5-pro | API key Google |
| `openrouter` | qualquer modelo disponível | API key OpenRouter |

O padrão é `ollama`. Para usar outro provider:

```bash
at config set-provider openai
at config set-api-key sk-...
at config set-model gpt-4o-mini
```

## Uso

```bash
# Daemon em background
at start                     # inicia em background
at start --interval 10       # intervalo customizado em minutos
at stop                      # para o daemon
at status                    # verifica se está rodando

# Coleta manual única
at collect

# Resumo dos últimos N dias
ats
ats --days 7
ats --today                  # atalho para o dia de hoje
ats --week                   # atalho para --days 7
ats --month                  # atalho para --days 30
ats --model gpt-4o-mini
ats --lang en
ats --verbose                # exibe dados brutos antes do resumo

# Busca por termo nos logs antes de resumir
ats --search docker           # filtra tudo que contém "docker" e resume
ats --search "cargo build"

# Resumo de uma data específica (formato YYYY-DD-MM)
ats --date 2026-06-06

# Override de provider por sessão (sem alterar config)
ats --provider anthropic --api-key sk-ant-... --model claude-haiku-4-5-20251001
ats --provider groq --model llama-3.1-8b-instant

# Enviar resumo ao Notion
ats --today --send-notion

# TUI interativa
at tui                       # abre a interface, hoje como data inicial
at tui --provider openai --model gpt-4o-mini

# Exportar logs em CSV ou JSON
at export                    # hoje em CSV (stdout)
at export --days 7           # últimos 7 dias
at export --format json      # formato JSON
at export --days 7 -o semana.csv  # salvar em arquivo
at export --date 2026-06-10  # data específica

# Configuração persistente
at config set-provider ollama       # ollama | openai | anthropic | groq | gemini | openrouter
at config set-model llama3.1
at config set-api-key <key>         # API key para providers cloud
at config set-url http://localhost:11434  # URL do Ollama (provider=ollama)
at config set-lang pt-br            # pt-br | en | es
at config set-notion-token secret_xxx
at config set-notion-page <page_id>
at config set-machine-name "MacBook Pro"  # opcional, usa hostname se omitido
at config show
```

## TUI interativa

`at tui` abre uma interface de terminal completa:

```
┌──────────────────── ◄ 2026-06-12 (hoje) ► ────────────────────┐
│     Atividades     │      Resumo      │      Projetos         │
├───────────────────────────────────────────────────────────────┤
│ ■ SHELL  (42 comandos)                                        │
│ ─────────────────────────────────────────────────────────     │
│   $ cargo build --release                                     │
│   $ git commit -m "feat: tui interativa"                      │
│   $ vim src/tui.rs                                            │
│   ...                                                         │
│                                                               │
│ ■ GIT  (2 repos, 5 commits)                                   │
│ ─────────────────────────────────────────────────────────     │
│   activity-tracker                                            │
│     ↳ 2026-06-12 feat: tui interativa                        │
├───────────────────────────────────────────────────────────────┤
│ ←/→ dia  Tab aba  ↑↓ scroll  r resumo  q sair                │
└───────────────────────────────────────────────────────────────┘
```

A aba **Projetos** exibe distribuição de tempo por repositório:

```
  activity-tracker  ████████████████░░░░░░░░   67.3%  (12c, 5d)
  meu-projeto       ████████░░░░░░░░░░░░░░░░   32.7%  ( 6c, 3d)
```

| Tecla | Ação |
|---|---|
| `←` / `→` | Navegar entre dias |
| `Tab` / `1` / `2` / `3` | Alternar entre abas Atividades, Resumo e Projetos |
| `↑` / `↓` ou `j` / `k` | Rolar conteúdo |
| `PgUp` / `PgDn` | Rolar rápido |
| `r` | Gerar resumo via LLM (usa config salva) |
| `s` | Na aba Projetos: janela de 7 dias |
| `m` | Na aba Projetos: janela de 30 dias |
| `q` / `Esc` | Sair |

Aceita os mesmos flags de provider que o `summary` (`--provider`, `--model`, `--api-key`, `--lang`).

## Autostart no login

### macOS

O `install.sh` cria e carrega um LaunchAgent automaticamente:

```bash
launchctl list | grep activity-tracker     # status
launchctl unload ~/Library/LaunchAgents/com.activity-tracker.plist  # parar
launchctl load -w ~/Library/LaunchAgents/com.activity-tracker.plist # iniciar

tail -f ~/.local/share/activity-tracker/daemon.log  # logs
```

### Linux (systemd)

```bash
systemctl --user status activity-tracker   # status
systemctl --user stop activity-tracker     # parar
systemctl --user start activity-tracker    # iniciar
systemctl --user disable activity-tracker  # remover do autostart

journalctl --user -u activity-tracker -f   # logs em tempo real
```

> Para o autostart funcionar após reinicialização sem sessão gráfica ativa (Linux):
> ```bash
> loginctl enable-linger $USER
> ```

## Permissões no macOS

O `install.sh` abre as telas automaticamente. Caso precise configurar manualmente, adicione o binário (`~/.local/bin/activity-tracker`) em:

- **System Settings → Privacy & Security → Full Disk Access** — para ler o histórico do Chrome/Brave
- **System Settings → Privacy & Security → Accessibility** — para capturar títulos de janelas via `osascript`

## Integração com Notion

Crie uma [Internal Integration](https://www.notion.so/my-integrations) no Notion, compartilhe uma página com ela e configure:

```bash
activity-tracker config set-notion-token secret_xxx
activity-tracker config set-notion-page <page_id>   # ID após o último / na URL da página
```

A partir daí, qualquer resumo pode ser enviado com `--send-notion`:

```bash
ats --today --send-notion
```

O resumo é criado como subpágina da página configurada. O título da nota usa a data real e o nome da máquina, ex: `2026-06-09 — MacBook Pro`. Se `machine_name` não estiver configurado, usa o hostname do sistema.

## O que é capturado

| Fonte | Detalhes |
|---|---|
| **Shell history** | Bash, Zsh e Fish — leitura incremental (só novos comandos desde a última coleta) |
| **Janelas abertas** | `wmctrl` (X11) → `xdotool` → `osascript` (macOS) → `ps` (fallback Linux) |
| **Chrome / Brave** | DevTools Protocol (porta 9222) ou SQLite history DB (últimas 2 horas) |
| **Git** | Todos os commits do dia por repo, encontrados até 4 níveis abaixo do home (máx. 15 repos) |

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
provider   = "ollama"
model      = "llama3.1"
ollama_url = "http://localhost:11434"
lang       = "pt-br"
# api_key         = "sk-..."         # opcional, para providers cloud
# notion_token    = "secret_xxx"     # opcional
# notion_page_id  = "abc123..."      # opcional
# machine_name    = "MacBook Pro"    # opcional, padrão: hostname do sistema
```

## Resumo gerado

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
