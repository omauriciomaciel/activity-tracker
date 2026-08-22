# Services

Major Rust modules that implement the application's components.

* [Collector Service](collector.md) - captures shell/windows/browser/git activity and writes daily JSONL logs
* [Daemon Service](daemon.md) - background process lifecycle and interval-based collection loop
* [Summarizer Service](summarizer.md) - aggregates logs, builds LLM context, calls providers, renders and persists summaries
* [TUI Service](tui.md) - interactive ratatui terminal interface with Activities, Summary, Projects, and Config tabs
* [Projects Service](projects.md) - computes per-repository commit distribution and time stats
* [Updater Service](updater.md) - self-update binary from GitHub Releases via atomic rename
* [Notion Integration](notion.md) - creates Notion sub-pages from markdown summaries
* [Slack Integration](slack.md) - sends summaries to Slack channels via incoming webhook
