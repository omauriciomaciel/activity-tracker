---
okf_version: "0.2"
---

# Activity Tracker - OKF Knowledge Bundle

A self-contained knowledge bundle describing the architecture, services, data models,
processing flows, CLI surface, and infrastructure of the **Activity Tracker** project - a
Rust daemon/CLI that captures system activity (shell history, open windows, browser tabs,
git context) and generates LLM-powered summaries.

## Sections

* [System](system/index.md) - high-level system overview and what the project does
* [Services](services/index.md) - major Rust modules that form the application components
* [Data Sources](data-sources/index.md) - the capture sources feeding the activity logs
* [Data Models](data/index.md) - core data structures and persisted formats
* [Pipelines](pipeline/index.md) - end-to-end processing flows (collection, summary, export, update)
* [API Surface](api/index.md) - CLI commands and external LLM provider APIs consumed
* [Infrastructure](infrastructure/index.md) - installer, CI/CD release pipeline, and storage layout

## Reserved Files

* [Update Log](log.md) - chronological history of this bundle
