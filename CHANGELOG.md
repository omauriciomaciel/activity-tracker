# CHANGELOG

<!-- version list -->

## v1.7.0 (2026-06-12)

### Features

- **tui**: Adiciona interface de usuário interativa com ratatui — navegar dias, ver atividades brutas e gerar resumo via LLM
  ([`598b530`](https://github.com/omauriciomaciel/activity-tracker/commit/598b530))

### Documentation

- **readme**: Adiciona documentação da TUI interativa com atalhos de teclado e mockup do layout
  ([`3afee88`](https://github.com/omauriciomaciel/activity-tracker/commit/3afee88))

### Chores

- **ci**: Melhora nomenclatura da matriz de build (ubuntu/macos em vez de target triple)
  ([`d0c0dd0`](https://github.com/omauriciomaciel/activity-tracker/commit/d0c0dd0))


## v1.6.0 (2026-06-12)

### Features

- **cli**: Adiciona filtros de busca (`--search`) e exportação de logs em CSV/JSON (`export`)
  ([`0ae233d`](https://github.com/omauriciomaciel/activity-tracker/commit/0ae233d))

- **summarizer**: Adiciona funcionalidade de busca com filtragem por termo nos resultados
  ([`a8d1479`](https://github.com/omauriciomaciel/activity-tracker/commit/a8d1479))

### Chores

- **ci**: Refatora pipeline de build para workflow reutilizável (`workflow_call`)
  ([`14a39a5`](https://github.com/omauriciomaciel/activity-tracker/commit/14a39a5))

- **ci**: Atualiza workflow de release
  ([`1efbb5a`](https://github.com/omauriciomaciel/activity-tracker/commit/1efbb5a))


## v1.5.0 (2026-06-12)

### Chores

- **ci**: Remove build para Windows e atualiza dependências
  ([`bd8c2b1`](https://github.com/omauriciomaciel/activity-tracker/commit/bd8c2b1b11a6d3f7d75ecfa90f2e7eb68c833ae2))

### Documentation

- **readme**: Atualiza instruções de instalação e uso de aliases
  ([`47596a7`](https://github.com/omauriciomaciel/activity-tracker/commit/47596a74a40424ae8dfe6402b3afb3a329581560))

### Features

- **collector**: Captura múltiplos commits por repositório
  ([`4072dae`](https://github.com/omauriciomaciel/activity-tracker/commit/4072dae55a963757e45d239f6c8d3e8176e87791))

- **install**: Adiciona aliases `at` e `ats` para o CLI
  ([`5a2a7b4`](https://github.com/omauriciomaciel/activity-tracker/commit/5a2a7b4de4475889197cd01550c8ca8e96e179b1))


## v1.4.0 (2026-06-12)

### Chores

- **ci**: Automatiza build e release com semantic-release
  ([`8b2bb91`](https://github.com/omauriciomaciel/activity-tracker/commit/8b2bb916a6b3c6af7020e1a1d06d3ef3d25325f2))

### Features

- **summarizer**: Adiciona suporte a múltiplos providers de LLM (OpenAI, Anthropic, Groq, Gemini, OpenRouter)
  ([`6b8e775`](https://github.com/omauriciomaciel/activity-tracker/commit/6b8e775e8a2de15ed8fdfe5c92ed4d8381500066))


## v1.3.0 (2026-06-12)

### Features

- **ci**: Implementa pipeline de build e release automatizado com artefatos por plataforma
  ([`fac3f7e`](https://github.com/omauriciomaciel/activity-tracker/commit/fac3f7e85d1ded83b72931924975ac1a7b03e853))

- **updater**: Implementa sistema de atualização automática via GitHub Releases (download de binário pré-compilado)
  ([`94f533a`](https://github.com/omauriciomaciel/activity-tracker/commit/94f533a6b80ffacb1c70a7fbc42761a4cf9e7b3a))


## v1.2.0 (2026-06-09)

### Features

- **notion**: Implementa integração para exportar resumos como páginas no Notion
  ([`a553bd1`](https://github.com/omauriciomaciel/activity-tracker/commit/a553bd17507e7564fdfb96e9ceb4c2352a39f7ab))


## v1.1.0 (2026-06-09)

### Bug Fixes

- **collector**: Restringe permissões de arquivos sensíveis
  ([`0f9d276`](https://github.com/omauriciomaciel/activity-tracker/commit/0f9d2767676352b298ab0fb90b1ffc2cbe48775d))

- **security**: Reforça proteção de dados e validações de rede
  ([`2bd2304`](https://github.com/omauriciomaciel/activity-tracker/commit/2bd2304eb15d826d41d76ea5684359c7126f6964))

### Code Style

- **core**: Aplica formatação e simplifica sintaxe
  ([`f269517`](https://github.com/omauriciomaciel/activity-tracker/commit/f269517c3d3723e217e3f18a65a498eb536a47f3))

### Features

- **install**: Adiciona suporte a autostart no macOS via LaunchAgent
  ([`89694f6`](https://github.com/omauriciomaciel/activity-tracker/commit/89694f6fc497ee7090ef42e420b491a991ccc96b))


## v1.0.0 (2026-06-08)

### Features

- **core**: Implementa sistema de rastreamento de atividades com Ollama
  ([`29de80f`](https://github.com/omauriciomaciel/activity-tracker/commit/29de80f))

- **daemon**: Implementa gestão de processos e autostart via systemd
  ([`71ebb3f`](https://github.com/omauriciomaciel/activity-tracker/commit/71ebb3f))

- **summary**: Adiciona suporte a resumo de data específica (`--date`)
  ([`bf545f5`](https://github.com/omauriciomaciel/activity-tracker/commit/bf545f5))

- **collector**: Implementa filtragem por data e limpeza de logs
  ([`b66d6d4`](https://github.com/omauriciomaciel/activity-tracker/commit/b66d6d4))

- **updater**: Adiciona comando de atualização automática
  ([`a2e5534`](https://github.com/omauriciomaciel/activity-tracker/commit/a2e5534))

- **summarizer**: Aprimora processamento de logs e sites
  ([`c44487f`](https://github.com/omauriciomaciel/activity-tracker/commit/c44487f))

- **summarizer**: Melhora a formatação visual da saída no terminal
  ([`67a1cc8`](https://github.com/omauriciomaciel/activity-tracker/commit/67a1cc8))

- **infra**: Adiciona script de instalação automatizada (`install.sh`)
  ([`6a9230a`](https://github.com/omauriciomaciel/activity-tracker/commit/6a9230a))

### Bug Fixes

- **collector**: Limpa logs após coleta e valida datas de commit
  ([`9bb6c03`](https://github.com/omauriciomaciel/activity-tracker/commit/9bb6c03))

- **collector**: Permite coleta de histórico sem timestamps
  ([`48ef3ff`](https://github.com/omauriciomaciel/activity-tracker/commit/48ef3ff))

- **core**: Garante consistência na instalação e respostas
  ([`4697555`](https://github.com/omauriciomaciel/activity-tracker/commit/4697555))

### Refactor

- **summarizer**: Otimiza agregação de dados e contexto do LLM
  ([`31b4355`](https://github.com/omauriciomaciel/activity-tracker/commit/31b4355))

### Documentation

- **readme**: Detalha instalação e gestão do daemon
  ([`eb23fa5`](https://github.com/omauriciomaciel/activity-tracker/commit/eb23fa5))

- Adiciona documentação inicial do projeto
  ([`4015dc2`](https://github.com/omauriciomaciel/activity-tracker/commit/4015dc2))

### Chores

- **ci**: Implementa release automatizado com semantic-release
  ([`8d1dea5`](https://github.com/omauriciomaciel/activity-tracker/commit/8d1dea5))

- **deps**: Atualiza edição do Rust para 2024
  ([`b10ea7f`](https://github.com/omauriciomaciel/activity-tracker/commit/b10ea7f))

- **project**: Reestrutura arquivos e atualiza reqwest
  ([`2067021`](https://github.com/omauriciomaciel/activity-tracker/commit/2067021))
