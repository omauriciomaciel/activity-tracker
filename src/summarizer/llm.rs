use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// \n aqui são dois chars literais (backslash + n), não newlines reais.
// build_prompt interpreta \n → newline antes de enviar ao LLM.
pub const DEFAULT_PROMPT_TEMPLATE: &str = r"Você é um assistente que analisa dados de atividade de computador e produz um resumo claro.\n{lang}\n\nDados coletados:\n\n{context}\nProduza:\n1. **Resumo Geral**: O que o usuário fez, em 2-3 parágrafos.\n2. **Projetos Identificados**: Projetos ou tarefas em andamento.\n3. **Ferramentas Mais Usadas**: Apps e ferramentas mais utilizados.\n4. **Sites e Pesquisas**: O que foi pesquisado ou lido online.\n5. **Sugestões**: Observações úteis sobre produtividade.\n\nSeja conciso. Não invente informações além dos dados.";

// ─── Structs de API ──────────────────────────────────────────────────────────

// Ollama
#[derive(Serialize)]
struct OllamaReq {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOpts,
}

#[derive(Serialize)]
struct OllamaOpts {
    temperature: f32,
    num_predict: i32,
    seed: i64,
}

#[derive(Deserialize)]
struct OllamaResp {
    response: String,
}

// OpenAI-compatible (OpenAI, Groq, OpenRouter)
#[derive(Serialize)]
struct OpenAiReq {
    model: String,
    messages: Vec<OpenAiMsg>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct OpenAiMsg {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResp {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMsgContent,
}

#[derive(Deserialize)]
struct OpenAiMsgContent {
    content: String,
}

// Anthropic
#[derive(Serialize)]
struct AnthropicReq {
    model: String,
    max_tokens: u32,
    messages: Vec<OpenAiMsg>,
}

#[derive(Deserialize)]
struct AnthropicResp {
    content: Vec<AnthropicBlock>,
}

#[derive(Deserialize)]
struct AnthropicBlock {
    #[serde(rename = "type")]
    kind: String,
    text: String,
}

// Gemini
#[derive(Serialize)]
struct GeminiReq {
    contents: Vec<GeminiContent>,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResp {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResp,
}

#[derive(Deserialize)]
struct GeminiContentResp {
    parts: Vec<GeminiPartResp>,
}

#[derive(Deserialize)]
struct GeminiPartResp {
    text: String,
}

// ─── Dispatcher ──────────────────────────────────────────────────────────────

pub async fn call_llm(
    provider: &str,
    ollama_url: &str,
    api_key: Option<&str>,
    model: &str,
    context: &str,
    lang: &str,
    custom_prompt: Option<&str>,
) -> Result<String> {
    let prompt = build_prompt(context, lang, custom_prompt);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    match provider {
        "ollama" => call_ollama(&client, ollama_url, model, &prompt).await,
        "openai" => {
            call_openai_compat(
                &client,
                "https://api.openai.com",
                api_key,
                model,
                &prompt,
                "openai",
            )
            .await
        }
        "groq" => {
            call_openai_compat(
                &client,
                "https://api.groq.com/openai",
                api_key,
                model,
                &prompt,
                "groq",
            )
            .await
        }
        "openrouter" => {
            call_openai_compat(
                &client,
                "https://openrouter.ai/api",
                api_key,
                model,
                &prompt,
                "openrouter",
            )
            .await
        }
        "anthropic" => call_anthropic(&client, api_key, model, &prompt).await,
        "gemini" => call_gemini(&client, api_key, model, &prompt).await,
        other => anyhow::bail!(
            "Provider '{other}' não suportado. Use: ollama, openai, anthropic, groq, gemini, openrouter"
        ),
    }
}

// ─── Prompt ──────────────────────────────────────────────────────────────────

fn lang_str(lang: &str) -> String {
    match lang {
        "pt-br" => "Responda em português brasileiro.".into(),
        "en" => "Answer in English.".into(),
        "es" => "Responde en español.".into(),
        "fr" => "Répondez en français.".into(),
        "de" => "Antworten Sie auf Deutsch.".into(),
        "ja" => "日本語で回答してください。".into(),
        "zh" => "请用中文回答。".into(),
        other => format!("Respond in {other}."),
    }
}

fn build_prompt(context: &str, lang: &str, custom_prompt: Option<&str>) -> String {
    let template = custom_prompt.unwrap_or(DEFAULT_PROMPT_TEMPLATE);
    let template = template.replace(r"\n", "\n");
    let lang_instruction = lang_str(lang);
    let result = template.replace("{lang}", &lang_instruction);
    if result.contains("{context}") {
        result.replace("{context}", context)
    } else {
        format!("{result}\n\nDados coletados:\n\n{context}")
    }
}

fn validate_url(url: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(url).with_context(|| format!("URL inválida: '{url}'"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        s => anyhow::bail!("URL: esquema '{s}' não permitido (use http ou https)"),
    }
    if let Some(host) = parsed.host_str()
        && matches!(
            host,
            "169.254.169.254" | "metadata.google.internal" | "metadata.google"
        )
    {
        anyhow::bail!("URL: host '{host}' não permitido");
    }
    Ok(())
}

// ─── Providers ───────────────────────────────────────────────────────────────

async fn call_ollama(
    client: &reqwest::Client,
    base_url: &str,
    model: &str,
    prompt: &str,
) -> Result<String> {
    validate_url(base_url)?;
    let req = OllamaReq {
        model: model.to_string(),
        prompt: prompt.to_string(),
        stream: false,
        options: OllamaOpts {
            temperature: 0.0,
            num_predict: 2048,
            seed: 42,
        },
    };

    let resp = client
        .post(format!("{base_url}/api/generate"))
        .json(&req)
        .send()
        .await
        .context("Não foi possível conectar ao Ollama. Está rodando? (ollama serve)")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Ollama erro {status}: {body}");
    }

    let r: OllamaResp = resp
        .json()
        .await
        .context("Erro parseando resposta Ollama")?;
    Ok(r.response)
}

async fn call_openai_compat(
    client: &reqwest::Client,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    prompt: &str,
    provider_name: &str,
) -> Result<String> {
    let key = api_key.ok_or_else(|| anyhow::anyhow!("api_key não configurada para {provider_name}. Execute: activity-tracker config set-api-key <key>"))?;
    let req = OpenAiReq {
        model: model.to_string(),
        messages: vec![OpenAiMsg {
            role: "user".into(),
            content: prompt.to_string(),
        }],
        temperature: 0.0,
        max_tokens: 2048,
    };

    let resp = client
        .post(format!("{base_url}/v1/chat/completions"))
        .bearer_auth(key)
        .json(&req)
        .send()
        .await
        .with_context(|| format!("Erro conectando ao {provider_name}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("{provider_name} erro {status}: {body}");
    }

    let r: OpenAiResp = resp
        .json()
        .await
        .with_context(|| format!("Erro parseando resposta {provider_name}"))?;
    r.choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| anyhow::anyhow!("{provider_name}: resposta vazia"))
}

async fn call_anthropic(
    client: &reqwest::Client,
    api_key: Option<&str>,
    model: &str,
    prompt: &str,
) -> Result<String> {
    let key = api_key.ok_or_else(|| anyhow::anyhow!("api_key não configurada para anthropic. Execute: activity-tracker config set-api-key <key>"))?;
    let req = AnthropicReq {
        model: model.to_string(),
        max_tokens: 2048,
        messages: vec![OpenAiMsg {
            role: "user".into(),
            content: prompt.to_string(),
        }],
    };

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .await
        .context("Erro conectando ao Anthropic")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic erro {status}: {body}");
    }

    let r: AnthropicResp = resp
        .json()
        .await
        .context("Erro parseando resposta Anthropic")?;
    r.content
        .into_iter()
        .find(|b| b.kind == "text")
        .map(|b| b.text)
        .ok_or_else(|| anyhow::anyhow!("Anthropic: resposta vazia"))
}

async fn call_gemini(
    client: &reqwest::Client,
    api_key: Option<&str>,
    model: &str,
    prompt: &str,
) -> Result<String> {
    let key = api_key.ok_or_else(|| anyhow::anyhow!("api_key não configurada para gemini. Execute: activity-tracker config set-api-key <key>"))?;
    let req = GeminiReq {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart {
                text: prompt.to_string(),
            }],
        }],
    };

    let resp = client
        .post(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
        ))
        .json(&req)
        .send()
        .await
        .context("Erro conectando ao Gemini")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Gemini erro {status}: {body}");
    }

    let r: GeminiResp = resp
        .json()
        .await
        .context("Erro parseando resposta Gemini")?;
    r.candidates
        .into_iter()
        .next()
        .and_then(|c| c.content.parts.into_iter().next())
        .map(|p| p.text)
        .ok_or_else(|| anyhow::anyhow!("Gemini: resposta vazia"))
}
