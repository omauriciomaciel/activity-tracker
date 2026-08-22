# Pipelines

End-to-end processing flows.

* [Collection Pipeline](collection.md) - the daemon's periodic capture-and-persist loop
* [Summary Generation Pipeline](summary-generation.md) - aggregate logs, call LLM, render, cache, and dispatch
* [Export Pipeline](export.md) - export raw log rows to CSV or JSON
* [Self-Update Pipeline](self-update.md) - check GitHub, download, extract, atomic-replace
