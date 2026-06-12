# CHANGELOG

<!-- version list -->

## v1.10.0 (2026-06-12)

### Chores

- **ci**: atualiza python-semantic-release e refatora changelog
  ([`170b7f1`](https://github.com/omauriciomaciel/activity-tracker/commit/170b7f1f1dd6eafc7600e683aae97957b93d54b2))

- **install**: torna o script de instalação POSIX compatível
  ([`e12aa61`](https://github.com/omauriciomaciel/activity-tracker/commit/e12aa61a04b2b2b9cd9b27fca766157995d8f426))

### Documentation

- **readme**: documenta nova aba de configurações na TUI
  ([`c39e66f`](https://github.com/omauriciomaciel/activity-tracker/commit/c39e66f46bc45e19904dd1362d12f3a853a8b30d))

### Features

- **notion**: melhora conversão de markdown para blocos
  ([`f2263c9`](https://github.com/omauriciomaciel/activity-tracker/commit/f2263c9f9b722a50cf21c658f42a5fa5457cb7f1))

- **tui**: adiciona renderização básica de markdown
  ([`c4a90a1`](https://github.com/omauriciomaciel/activity-tracker/commit/c4a90a1b150742eb25e21a5d7f9660481acbea9c))

- **tui**: agrupa abas duplicadas na renderização de atividades
  ([`093466d`](https://github.com/omauriciomaciel/activity-tracker/commit/093466d8dcbf2b60af4ef55602d7ca1c0c0085f0))

- **collector**: adiciona funcionalidade de remover tags
  ([`a1cc9c0`](https://github.com/omauriciomaciel/activity-tracker/commit/a1cc9c044fa8159f9444b50b55b6e171d8a279c1))

- **tui**: implementa suporte a interação via mouse
  ([`7dfd836`](https://github.com/omauriciomaciel/activity-tracker/commit/7dfd8360ccf7b96d4e84d3883f96921c555d8c14))

- **config**: adiciona suporte a prompts customizados para o LLM
  ([`17c2cdc`](https://github.com/omauriciomaciel/activity-tracker/commit/17c2cdcb243a7426be77c25b4c7f2309e4aac783))

- **core**: adiciona funcionalidade de anotações manuais (tags)
  ([`b84fad3`](https://github.com/omauriciomaciel/activity-tracker/commit/b84fad314d3fbebdebdcfeda1c87eb604fa82cbd))

### Refactoring

- **summarizer, tui**: reestrutura módulos de resumo e interface
  ([`1c36687`](https://github.com/omauriciomaciel/activity-tracker/commit/1c36687fe587f36f2893f1d7d2f974fdd5cdc34f))


## v1.9.0 (2026-06-12)

### Chores

- **ci**: atualiza versão e ajusta release automation
  ([`93dc4d4`](https://github.com/omauriciomaciel/activity-tracker/commit/93dc4d43af18046bdf202feb383bafd1508d5c3b))

### Features

- **tui**: implementa edição interativa de configurações
  ([`303e953`](https://github.com/omauriciomaciel/activity-tracker/commit/303e9539f66f34cc3c48fa528be9f1239b4cf361))

- **tui**: adiciona aba de visualização de configurações
  ([`35c7939`](https://github.com/omauriciomaciel/activity-tracker/commit/35c79390fc460c134ed9d850e056b9035ae7210c))

- **collector**: implementa filtragem de padrões de privacidade
  ([`7734195`](https://github.com/omauriciomaciel/activity-tracker/commit/7734195f8721acda67efc6550aff6d8409ec789b))

- **summarizer**: implementa persistência de resumos em disco
  ([`d82cd5d`](https://github.com/omauriciomaciel/activity-tracker/commit/d82cd5d866d032fbfb8191e0cfa5ce304be8c117))

- **slack**: adiciona integração para envio de resumos via webhook
  ([`318f101`](https://github.com/omauriciomaciel/activity-tracker/commit/318f1014ed0cef526d4fd91254c92b197fa65266))


## v1.8.0 (2026-06-12)

### Documentation

- **changelog**: atualiza histórico de versões e detalhes
  ([`08e4515`](https://github.com/omauriciomaciel/activity-tracker/commit/08e4515143624965390c96b1f51a16839eb75093))

### Features

- **install**: implementa instalação via binários pré-compilados
  ([`e0d9813`](https://github.com/omauriciomaciel/activity-tracker/commit/e0d98131a9b6540941d87a51b3d9ea5c58078300))

- **projects**: adiciona análise e visualização de estatísticas de projetos
  ([`9f64e24`](https://github.com/omauriciomaciel/activity-tracker/commit/9f64e244f253b8f8fe155c3b07da4a9b09945c49))


## v1.6.0 (2026-06-12)

### Chores

- **ci**: melhora nomenclatura e configuração do build
  ([`d0c0dd0`](https://github.com/omauriciomaciel/activity-tracker/commit/d0c0dd0026b6dda0c98132c88a88ad25e95d3e03))

### Documentation

- **readme**: adiciona documentação da TUI interativa
  ([`3afee88`](https://github.com/omauriciomaciel/activity-tracker/commit/3afee88eec0b7904485509f0b721fb8cc5f224ff))

### Features

- **tui**: adiciona interface de usuário interativa
  ([`598b530`](https://github.com/omauriciomaciel/activity-tracker/commit/598b5302e40c7c9e9520b62f362690178eb37662))


## v1.5.0 (2026-06-12)

### Chores

- **ci**: refatora pipeline de build para workflow reutilizável
  ([`14a39a5`](https://github.com/omauriciomaciel/activity-tracker/commit/14a39a52cb39832742d5783971ab138e1682c98b))

- **ci**: atualiza workflow de release
  ([`1efbb5a`](https://github.com/omauriciomaciel/activity-tracker/commit/1efbb5a15eb7a1c3ba415ba47724d454f058441a))

- **ci**: remove build para Windows e atualiza dependências
  ([`bd8c2b1`](https://github.com/omauriciomaciel/activity-tracker/commit/bd8c2b1b11a6d3f7d75ecfa90f2e7eb68c833ae2))

### Documentation

- **readme**: atualiza instruções de instalação e uso de aliases
  ([`47596a7`](https://github.com/omauriciomaciel/activity-tracker/commit/47596a74a40424ae8dfe6402b3afb3a329581560))

### Features

- **cli**: adiciona filtros de busca e exportação de logs
  ([`0ae233d`](https://github.com/omauriciomaciel/activity-tracker/commit/0ae233d0104952fac4dad84b19fb671bdec8f012))

- **summarizer**: adiciona funcionalidade de busca nos resultados
  ([`a8d1479`](https://github.com/omauriciomaciel/activity-tracker/commit/a8d1479b0a226dbba47d0ccf8059020e9edf43c4))

- **install**: adiciona aliases 'at' e 'ats' para o CLI
  ([`5a2a7b4`](https://github.com/omauriciomaciel/activity-tracker/commit/5a2a7b4de4475889197cd01550c8ca8e96e179b1))

- **collector**: captura múltiplos commits por repositório
  ([`4072dae`](https://github.com/omauriciomaciel/activity-tracker/commit/4072dae55a963757e45d239f6c8d3e8176e87791))


## v1.4.0 (2026-06-12)

### Chores

- **ci**: automatiza build e release com semantic-release
  ([`8b2bb91`](https://github.com/omauriciomaciel/activity-tracker/commit/8b2bb916a6b3c6af7020e1a1d06d3ef3d25325f2))

### Features

- **summarizer**: adiciona suporte a múltiplos providers de LLM
  ([`6b8e775`](https://github.com/omauriciomaciel/activity-tracker/commit/6b8e775e8a2de15ed8fdfe5c92ed4d8381500066))


## v1.3.0 (2026-06-12)

### Features

- **updater**: implementa sistema de atualização automática via GitHub
  ([`94f533a`](https://github.com/omauriciomaciel/activity-tracker/commit/94f533a6b80ffacb1c70a7fbc42761a4cf9e7b3a))

- **ci**: implementa pipeline de build e release automatizado
  ([`fac3f7e`](https://github.com/omauriciomaciel/activity-tracker/commit/fac3f7e85d1ded83b72931924975ac1a7b03e853))


## v1.2.0 (2026-06-09)

### Features

- **notion**: implementa integração para exportar resumos
  ([`a553bd1`](https://github.com/omauriciomaciel/activity-tracker/commit/a553bd17507e7564fdfb96e9ceb4c2352a39f7ab))


## v1.1.0 (2026-06-09)

### Bug Fixes

- **security**: reforça proteção de dados e validações de rede
  ([`2bd2304`](https://github.com/omauriciomaciel/activity-tracker/commit/2bd2304eb15d826d41d76ea5684359c7126f6964))

- **collector**: restringe permissões de arquivos sensíveis
  ([`0f9d276`](https://github.com/omauriciomaciel/activity-tracker/commit/0f9d2767676352b298ab0fb90b1ffc2cbe48775d))

### Code Style

- **core**: aplica formatação e simplifica sintaxe
  ([`f269517`](https://github.com/omauriciomaciel/activity-tracker/commit/f269517c3d3723e217e3f18a65a498eb536a47f3))

### Features

- **install**: adiciona suporte a autostart no macOS
  ([`89694f6`](https://github.com/omauriciomaciel/activity-tracker/commit/89694f6fc497ee7090ef42e420b491a991ccc96b))


## v1.0.0 (2026-06-08)

### Bug Fixes

- **collector**: limpa logs após coleta e valida datas de commit
  ([`9bb6c03`](https://github.com/omauriciomaciel/activity-tracker/commit/9bb6c03036dcdcbd99104c78116a37931ddca015))

- **collector**: permite coleta de histórico sem timestamps
  ([`48ef3ff`](https://github.com/omauriciomaciel/activity-tracker/commit/48ef3ff85d6dbc51a91a1d3b49acd8619d424744))

- **core**: garante consistência na instalação e respostas
  ([`4697555`](https://github.com/omauriciomaciel/activity-tracker/commit/4697555738c1efa8a1d9e9b323ebf857d035bde9))

### Chores

- **ci**: implementa release automatizado com semantic-release
  ([`8d1dea5`](https://github.com/omauriciomaciel/activity-tracker/commit/8d1dea546d029662acac8ba11150b0ebad632a1f))

- **project**: reestrutura arquivos e atualiza reqwest
  ([`2067021`](https://github.com/omauriciomaciel/activity-tracker/commit/2067021d8a22b488b8e39346888dad8242c139f3))

- **deps**: atualiza edição do Rust para 2024
  ([`b10ea7f`](https://github.com/omauriciomaciel/activity-tracker/commit/b10ea7f0825486b76ecbfd521a887754d2c536fa))

- **git**: ignora diretório .serena
  ([`a9c3933`](https://github.com/omauriciomaciel/activity-tracker/commit/a9c3933e7af4911d0c8f384ca805c173e2b5cfc4))

### Code Style

- **cli**: remove emojis e simplifica mensagens de log
  ([`92162eb`](https://github.com/omauriciomaciel/activity-tracker/commit/92162eb0f565d98295503f91c4e041e3cfe8e6a1))

### Documentation

- **readme**: detalha instalação e gestão do daemon
  ([`eb23fa5`](https://github.com/omauriciomaciel/activity-tracker/commit/eb23fa5bfaea24b70c5745b748eaac6780ead5d6))

- adiciona documentação inicial do projeto
  ([`4015dc2`](https://github.com/omauriciomaciel/activity-tracker/commit/4015dc286620a0595c91227715ae3be6880db9ab))

### Features

- **summarizer**: aprimora processamento de logs e sites
  ([`c44487f`](https://github.com/omauriciomaciel/activity-tracker/commit/c44487fc7562cda0138f1aca4788b0708dd14aa9))

- **summarizer**: melhora a formatação visual da saída no terminal
  ([`67a1cc8`](https://github.com/omauriciomaciel/activity-tracker/commit/67a1cc884a641ba0efb7033702c3de4b0dc7ae9a))

- **updater**: adiciona comando de atualização automática
  ([`a2e5534`](https://github.com/omauriciomaciel/activity-tracker/commit/a2e5534e66e19158d0da126cf4749503d2ce5d08))

- **collector**: implementa filtragem por data e limpeza de logs
  ([`b66d6d4`](https://github.com/omauriciomaciel/activity-tracker/commit/b66d6d4b92e451e5c45a60595359559c49bdec27))

- **daemon**: implementa gestão de processos e autostart via systemd
  ([`71ebb3f`](https://github.com/omauriciomaciel/activity-tracker/commit/71ebb3f74f1a44fae1a246fd88f4c477138b0a92))

- **summary**: adiciona suporte a resumo de data específica
  ([`bf545f5`](https://github.com/omauriciomaciel/activity-tracker/commit/bf545f5662817a571e11d392237d7c1f616fae4c))

- **infra**: adiciona script de instalação automatizada
  ([`6a9230a`](https://github.com/omauriciomaciel/activity-tracker/commit/6a9230a776b5b42f7b3155fa767978a936ea66e9))

- **core**: implementa sistema de rastreamento de atividades com Ollama
  ([`29de80f`](https://github.com/omauriciomaciel/activity-tracker/commit/29de80f187d5f6d1efca4514832c07ca2dcd82bd))

### Refactoring

- **summarizer**: otimiza agregação de dados e contexto do LLM
  ([`31b4355`](https://github.com/omauriciomaciel/activity-tracker/commit/31b4355027530a8e223e7090637715d05c6b14e1))
