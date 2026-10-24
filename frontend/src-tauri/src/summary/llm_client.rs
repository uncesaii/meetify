use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

const REQUEST_TIMEOUT_DURATION: Duration = Duration::from_secs(300);

async fn await_or_cancel<T>(
    operation: impl Future<Output = T>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<T, String> {
    let Some(token) = cancellation_token else {
        return Ok(operation.await);
    };

    tokio::select! {
        biased;
        _ = token.cancelled() => Err("Summary generation was cancelled".to_string()),
        result = operation => Ok(result),
    }
}

// Generic structure for OpenAI-compatible API chat messages
#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// Generic structure for OpenAI-compatible API chat requests
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

/// Build OpenAI-compat JSON body.
pub fn build_openai_compat_chat_body(
    provider: &LLMProvider,
    model_name: &str,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
) -> serde_json::Value {
    let (max_tokens_val, temperature_val, top_p_val) = if *provider == LLMProvider::CustomOpenAI {
        (max_tokens, temperature, top_p)
    } else {
        (None, None, None)
    };

    serde_json::json!(ChatRequest {
        model: model_name.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            }
        ],
        max_tokens: max_tokens_val,
        temperature: temperature_val,
        top_p: top_p_val,
    })
}

// Generic structure for OpenAI-compatible API chat responses
#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
pub struct Choice {
    pub message: MessageContent,
}

#[derive(Deserialize, Debug)]
pub struct MessageContent {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
}

// Gemini-specific request/response structures
#[derive(Debug, Serialize)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct GeminiResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize, Debug)]
pub struct GeminiCandidate {
    pub content: GeminiContent,
}

impl GeminiResponse {
    fn completion(&self) -> Option<LlmCompletion> {
        let content = self
            .candidates
            .as_ref()?
            .first()?
            .content
            .parts
            .first()?
            .text
            .trim()
            .to_string();
        Some(LlmCompletion {
            content,
            reasoning_stripped: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LlmCompletion {
    pub content: String,
    pub reasoning_stripped: bool,
}

impl ChatResponse {
    fn completion(&self) -> Result<LlmCompletion, String> {
        let message = &self
            .choices
            .first()
            .ok_or("No content in LLM response")?
            .message;
        Ok(LlmCompletion {
            content: message
                .content
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string(),
            reasoning_stripped: [message.reasoning.as_deref(), message.reasoning_content.as_deref()]
                .into_iter()
                .flatten()
                .any(|reasoning| !reasoning.trim().is_empty()),
        })
    }
}

/// LLM Provider enumeration for multi-provider support
#[derive(Debug, Clone, PartialEq)]
pub enum LLMProvider {
    Gemini,
    Groq,
    OpenRouter,
    CustomOpenAI,
}

impl LLMProvider {
    /// Parse provider from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "gemini" | "google" => Ok(Self::Gemini),
            "groq" => Ok(Self::Groq),
            "openrouter" => Ok(Self::OpenRouter),
            "custom-openai" => Ok(Self::CustomOpenAI),
            _ => Err(format!("Unsupported LLM provider: {}", s)),
        }
    }
}

/// Generates a summary using the specified LLM provider
pub(crate) async fn generate_summary(
    client: &Client,
    provider: &LLMProvider,
    model_name: &str,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    _ollama_endpoint: Option<&str>,
    custom_openai_endpoint: Option<&str>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    _app_data_dir: Option<&PathBuf>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<LlmCompletion, String> {
    // Check if cancelled before starting
    if let Some(token) = cancellation_token {
        if token.is_cancelled() {
            return Err("Summary generation was cancelled".to_string());
        }
    }

    // Handle Gemini separately (different API format)
    if provider == &LLMProvider::Gemini {
        return generate_gemini_summary(
            client,
            model_name,
            api_key,
            system_prompt,
            user_prompt,
            cancellation_token,
        )
        .await;
    }

    let (api_url, mut headers) = match provider {
        LLMProvider::Groq => (
            "https://api.groq.com/openai/v1/chat/completions".to_string(),
            header::HeaderMap::new(),
        ),
        LLMProvider::OpenRouter => (
            "https://openrouter.ai/api/v1/chat/completions".to_string(),
            header::HeaderMap::new(),
        ),
        LLMProvider::CustomOpenAI => {
            let endpoint = custom_openai_endpoint
                .ok_or_else(|| "Custom OpenAI endpoint not configured".to_string())?;
            (
                format!("{}/chat/completions", endpoint.trim_end_matches('/')),
                header::HeaderMap::new(),
            )
        }
        LLMProvider::Gemini => {
            unreachable!("Gemini is handled before this match statement")
        }
    };

    // Add authorization header
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {}", api_key)
            .parse()
            .map_err(|_| "Invalid authorization header".to_string())?,
    );
    headers.insert(
        header::CONTENT_TYPE,
        "application/json"
            .parse()
            .map_err(|_| "Invalid content type".to_string())?,
    );

    // Build request body
    let request_body = build_openai_compat_chat_body(
        provider,
        model_name,
        system_prompt,
        user_prompt,
        max_tokens,
        temperature,
        top_p,
    );

    info!("🤖 LLM Request to {}: model={}", provider_name(provider), model_name);

    // Send request with timeout and cancellation support
    let request_future = client
        .post(api_url.clone())
        .headers(headers.clone())
        .json(&request_body)
        .timeout(REQUEST_TIMEOUT_DURATION)
        .send();

    let response = if let Some(token) = cancellation_token {
        tokio::select! {
            result = request_future => {
                result.map_err(|e| {
                    if e.is_timeout() {
                        format!(
                            "LLM request timed out after {} seconds",
                            REQUEST_TIMEOUT_DURATION.as_secs()
                        )
                    } else {
                        format!("Failed to send request to LLM: {}", e)
                    }
                })?
            }
            _ = token.cancelled() => {
                return Err("Summary generation was cancelled".to_string());
            }
        }
    } else {
        request_future.await.map_err(|e| {
            if e.is_timeout() {
                format!(
                    "LLM request timed out after {} seconds",
                    REQUEST_TIMEOUT_DURATION.as_secs()
                )
            } else {
                format!("Failed to send request to LLM: {}", e)
            }
        })?
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = await_or_cancel(response.text(), cancellation_token)
            .await?
            .unwrap_or_else(|error| format!("Failed to read LLM error response body: {error}"));
        return Err(format!(
            "LLM API request failed with status {}: {}",
            status, error_body
        ));
    }

    let chat_response = await_or_cancel(
        response.json::<ChatResponse>(),
        cancellation_token,
    )
    .await?
    .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

    info!("🤖 LLM Response received from {}", provider_name(provider));

    chat_response.completion()
}

/// Generate summary using Google Gemini API
async fn generate_gemini_summary(
    client: &Client,
    model_name: &str,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    cancellation_token: Option<&CancellationToken>,
) -> Result<LlmCompletion, String> {
    let api_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name, api_key
    );

    let request_body = serde_json::json!(GeminiRequest {
        system_instruction: Some(GeminiContent {
            role: None,
            parts: vec![GeminiPart {
                text: system_prompt.to_string(),
            }],
        }),
        contents: vec![GeminiContent {
            role: Some("user".to_string()),
            parts: vec![GeminiPart {
                text: user_prompt.to_string(),
            }],
        }],
    });

    info!("🤖 Gemini Request: model={}", model_name);

    let request_future = client
        .post(&api_url)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request_body)
        .timeout(REQUEST_TIMEOUT_DURATION)
        .send();

    let response = await_or_cancel(request_future, cancellation_token)
        .await?
        .map_err(|e| {
            if e.is_timeout() {
                format!(
                    "Gemini request timed out after {} seconds",
                    REQUEST_TIMEOUT_DURATION.as_secs()
                )
            } else {
                format!("Failed to send request to Gemini: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = await_or_cancel(response.text(), cancellation_token)
            .await?
            .unwrap_or_else(|error| format!("Failed to read Gemini error response body: {error}"));
        return Err(format!(
            "Gemini API request failed with status {}: {}",
            status, error_body
        ));
    }

    let gemini_response = await_or_cancel(
        response.json::<GeminiResponse>(),
        cancellation_token,
    )
    .await?
    .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

    info!("🤖 Gemini Response received");

    gemini_response
        .completion()
        .ok_or("No content in Gemini response".to_string())
}

/// Helper function to get provider name for logging
fn provider_name(provider: &LLMProvider) -> &str {
    match provider {
        LLMProvider::Gemini => "Gemini",
        LLMProvider::Groq => "Groq",
        LLMProvider::OpenRouter => "OpenRouter",
        LLMProvider::CustomOpenAI => "Custom OpenAI",
    }
}
