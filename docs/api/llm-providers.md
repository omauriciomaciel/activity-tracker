---
type: API
title: LLM Provider APIs
description: The six external LLM HTTP APIs consumed by the summarizer (Ollama, OpenAI-compatible, Anthropic, Gemini), including endpoints, request/response shapes, and the URL-validation guard.
tags: [llm, api, ollama, openai, anthropic, gemini, groq, openrouter, external]
status: stable
---

# LLM Provider APIs

**Implemented in**: `src/summarizer/llm.rs`
**Dispatcher**: `call_llm(provider, ollama_url, api_key, model, context, lang, custom_prompt)`

Activity Tracker supports six LLM providers. All requests use a single `reqwest::Client` with
a 300-second timeout. Each provider has its own request/response structs (serde).

## Provider Matrix

| Provider | Base URL | Auth | Endpoint |
|---|---|---|---|
| `ollama` (default) | `{ollama_url}` (default `http://localhost:11434`) | none | `POST /api/generate` |
| `openai` | `https://api.openai.com` | `Bearer {key}` | `POST /v1/chat/completions` |
| `groq` | `https://api.groq.com/openai` | `Bearer {key}` | `POST /v1/chat/completions` |
| `openrouter` | `https://openrouter.ai/api` | `Bearer {key}` | `POST /v1/chat/completions` |
| `anthropic` | `https://api.anthropic.com` | `x-api-key: {key}` + `anthropic-version: 2023-06-01` | `POST /v1/messages` |
| `gemini` | `https://generativelanguage.googleapis.com` | `?key={key}` query param | `POST /v1beta/models/{model}:generateContent` |

`groq` and `openrouter` reuse the OpenAI-compatible request/response shape via
`call_openai_compat` with a `provider_name` label for error messages.

## Request Shapes

### Ollama

```json
{ "model": "<model>", "prompt": "<prompt>", "stream": false,
  "options": { "temperature": 0.0, "num_predict": 2048, "seed": 42 } }
```

Response: `{ "response": "<text>" }`. Uses temperature 0 and a fixed seed (42) for
deterministic output.

### OpenAI-compatible (OpenAI, Groq, OpenRouter)

```json
{ "model": "<model>",
  "messages": [ { "role": "user", "content": "<prompt>" } ],
  "temperature": 0.0, "max_tokens": 2048 }
```

Response: `{ "choices": [ { "message": { "content": "<text>" } } ] }`. Takes the first choice.

### Anthropic

```json
{ "model": "<model>", "max_tokens": 2048,
  "messages": [ { "role": "user", "content": "<prompt>" } ] }
```

Headers: `x-api-key: {key}`, `anthropic-version: 2023-06-01`.
Response: `{ "content": [ { "type": "text", "text": "<text>" } ] }`. Takes the first `text` block.

### Gemini

```json
{ "contents": [ { "parts": [ { "text": "<prompt>" } ] } ] }
```

URL: `.../v1beta/models/{model}:generateContent?key={key}`.
Response: `{ "candidates": [ { "content": { "parts": [ { "text": "<text>" } ] } } ] }`.
Takes the first candidate's first part.

## URL Validation (SSRF guard)

`validate_url(url)` runs before any Ollama call:

- Rejects non-`http`/`https` schemes.
- Blocks metadata-service hosts: `169.254.169.254`, `metadata.google.internal`,
  `metadata.google`.

This prevents a malicious `ollama_url` config from reaching cloud metadata endpoints.

## Prompt Construction

`build_prompt(context, lang, custom_prompt)`:

1. Template = `custom_prompt` or `DEFAULT_PROMPT_TEMPLATE`.
2. Replace literal `\n` (backslash-n) with real newlines.
3. Inject `{lang}` -> per-language instruction (pt-br, en, es, fr, de, ja, zh, or
   `"Respond in {lang}."` fallback).
4. Inject `{context}` -> the activity context. If `{context}` is absent, append the data.

The default template requests five sections (Resumo Geral, Projetos Identificados,
Ferramentas Mais Usadas, Sites e Pesquisas, Sugestões) and instructs the model not to
invent information beyond the data.

## Error Handling

Every provider bails with `"{provider} erro {status}: {body}"` on non-2xx, and contextual
errors for connection/parse failures. Missing API keys produce a setup hint pointing to
`activity-tracker config set-api-key <key>`.

## Configuration

Provider, model, URL, and key come from [Config](../data/config.md) and can be overridden
per-invocation via CLI flags (see [CLI API](cli.md) `summary`).

## Related

- Called by [Summarizer Service](../services/summarizer.md) and [TUI Service](../services/tui.md)
- Driven by [Summary Generation Pipeline](../pipeline/summary-generation.md)
- Selection stored in [Config](../data/config.md)
