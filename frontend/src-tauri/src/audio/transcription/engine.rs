// audio/transcription/engine.rs
//
// Transcription engine - uses Groq Whisper API for speech-to-text with accuracy optimizations.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tracing::{info, warn};

/// Transcription engine enum
pub enum TranscriptionEngine {
    GroqApi {
        api_key: String,
        model: String,
    },
    /// Legacy variant kept for compatibility - wraps a trait-based provider
    Provider(Arc<dyn super::provider::TranscriptionProvider>),
}

impl Clone for TranscriptionEngine {
    fn clone(&self) -> Self {
        match self {
            TranscriptionEngine::GroqApi { api_key, model } => TranscriptionEngine::GroqApi {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            TranscriptionEngine::Provider(p) => TranscriptionEngine::Provider(p.clone()),
        }
    }
}

impl TranscriptionEngine {
    pub async fn is_model_loaded(&self) -> bool {
        // Groq API is always ready (no local model to load)
        true
    }

    pub fn provider_name(&self) -> &str {
        match self {
            TranscriptionEngine::GroqApi { .. } => "groq",
            TranscriptionEngine::Provider(_) => "provider",
        }
    }

    pub fn get_current_model(&self) -> Option<String> {
        match self {
            TranscriptionEngine::GroqApi { model, .. } => Some(model.clone()),
            TranscriptionEngine::Provider(_) => Some("api-provider".to_string()),
        }
    }

    pub async fn transcribe(&self, audio_samples: &[f32], sample_rate: u32) -> Result<String, String> {
        self.transcribe_with_options(audio_samples, sample_rate, None, None).await
    }

    pub async fn transcribe_with_options(
        &self,
        audio_samples: &[f32],
        sample_rate: u32,
        language: Option<String>,
        prompt: Option<String>,
    ) -> Result<String, String> {
        match self {
            TranscriptionEngine::GroqApi { api_key, model } => {
                transcribe_with_groq(
                    api_key,
                    model,
                    audio_samples,
                    sample_rate,
                    language.as_deref(),
                    prompt.as_deref(),
                )
                .await
            }
            TranscriptionEngine::Provider(provider) => {
                let result = provider
                    .transcribe(audio_samples.to_vec(), language)
                    .await
                    .map_err(|e| format!("Transcription error: {}", e))?;
                Ok(result.text)
            }
        }
    }

    pub async fn unload_model(&self) {
        // No-op for API-based transcription
    }
}

/// Transcribe audio using Groq Whisper API with accuracy-enhancing settings
async fn transcribe_with_groq(
    api_key: &str,
    model: &str,
    audio_samples: &[f32],
    sample_rate: u32,
    language: Option<&str>,
    prompt: Option<&str>,
) -> Result<String, String> {
    // 1. Guard: Skip tiny audio fragments (< 200ms at 16kHz) to avoid Whisper hallucinations
    if audio_samples.len() < 3200 {
        return Ok(String::new());
    }

    // 2. Guard: Skip silence / background noise below speech energy floor
    let raw_peak = audio_samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    if raw_peak < 0.008 {
        return Ok(String::new());
    }

    // Convert samples to 16-bit PCM WAV with dynamic gain normalization & DC offset removal
    let wav_data = samples_to_wav(audio_samples, sample_rate)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let effective_model = if model.trim().is_empty() {
        "whisper-large-v3"
    } else {
        model.trim()
    };

    let mut form = reqwest::multipart::Form::new()
        .text("model", effective_model.to_string())
        .text("response_format", "text")
        // Temperature 0.0 enforces greedy decoding (eliminates hallucinations and creative deviations)
        .text("temperature", "0.0")
        .part(
            "file",
            reqwest::multipart::Part::bytes(wav_data)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(|e| format!("MIME error: {}", e))?,
        );

    // Route endpoint and language according to user selection
    let (endpoint, resolved_lang) = match language {
        Some(l) => {
            let clean = l.trim().to_lowercase();
            if clean == "auto-translate" {
                // OpenAI / Groq audio translations endpoint translates any language directly into English
                ("https://api.groq.com/openai/v1/audio/translations", None)
            } else if clean.is_empty() || clean == "auto" {
                // Auto-detect: do NOT set language param so Whisper's language-identification head auto-detects
                ("https://api.groq.com/openai/v1/audio/transcriptions", None)
            } else if clean == "tl" || clean == "fil" || clean == "tagalog" || clean == "taglish" {
                // Tagalog / Taglish: Whisper uses 'tl' ISO code
                ("https://api.groq.com/openai/v1/audio/transcriptions", Some("tl"))
            } else {
                ("https://api.groq.com/openai/v1/audio/transcriptions", Some(l.trim()))
            }
        }
        None => ("https://api.groq.com/openai/v1/audio/transcriptions", None), // Default: Auto-detect
    };

    if let Some(lang) = resolved_lang {
        form = form.text("language", lang.to_string());
    }

    // Align with Meetily production architecture:
    // Do NOT pass synthetic scripted dialogues in prompt — Whisper hallucinates them!
    // If genuine preceding transcript context exists, pass only recent clean text for natural flow.
    if let Some(p) = prompt {
        let clean_p = p.trim();
        if !clean_p.is_empty() {
            let slice = if clean_p.len() > 150 {
                &clean_p[clean_p.len() - 150..]
            } else {
                clean_p
            };
            form = form.text("prompt", slice.to_string());
        }
    }

    let response = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Groq API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Groq API error {}: {}", status, body));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Apply Meetily production anti-hallucination & anti-repetition cleaning
    let cleaned = clean_repetitive_text(&text);
    Ok(cleaned)
}

/// Clean repetitive text and filter hallucinations (aligned with Meetily production)
pub fn clean_repetitive_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Check for obviously meaningless patterns first
    if is_meaningless_output(trimmed) {
        return String::new();
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() < 3 {
        return trimmed.to_string();
    }

    // Enhanced repetition detection with sliding window
    let cleaned_words = remove_word_repetitions(&words);

    // Remove phrase repetitions with variable length detection
    let cleaned_words = remove_phrase_repetitions(&cleaned_words);

    // Check for overall repetition ratio
    let final_text = cleaned_words.join(" ");
    if calculate_repetition_ratio(&final_text) > 0.60 {
        return String::new();
    }

    final_text
}

// Check for obviously meaningless / hallucinated patterns
fn is_meaningless_output(text: &str) -> bool {
    let text_lower = text.to_lowercase();

    let meaningless_patterns = [
        "thank you for watching",
        "thanks for watching",
        "like and subscribe",
        "please subscribe",
        "subtitles by",
        "translated by",
        "music playing",
        "applause",
        "laughter",
        "um um um",
        "uh uh uh",
        "ah ah ah",
        "salamat sa panonood",
        "maraming salamat sa panonood",
        "[blank_audio]",
        "[silence]",
        "(silence)",
        "[music]",
        "(music)",
        "[applause]",
        "(applause)",
    ];

    for pattern in &meaningless_patterns {
        if text_lower.contains(pattern) {
            return true;
        }
    }

    // Check if text is mostly the same character or very short repetitive patterns
    let unique_chars: HashSet<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if unique_chars.len() <= 3 && text.len() > 10 {
        return true;
    }

    false
}

// Consecutive word repetition removal
fn remove_word_repetitions<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
    let mut cleaned_words = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let current_word = words[i];
        let mut repeat_count = 1;

        while i + repeat_count < words.len() && words[i + repeat_count].eq_ignore_ascii_case(current_word) {
            repeat_count += 1;
        }

        if repeat_count >= 2 {
            cleaned_words.push(current_word);
            i += repeat_count;
        } else {
            cleaned_words.push(current_word);
            i += 1;
        }
    }

    cleaned_words
}

// Consecutive phrase repetition removal (2 to 5 words)
fn remove_phrase_repetitions<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
    if words.len() < 4 {
        return words.to_vec();
    }

    let mut final_words = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let mut phrase_found = false;

        for phrase_len in 2..=std::cmp::min(5, (words.len() - i) / 2) {
            if i + phrase_len * 2 <= words.len() {
                let phrase1 = &words[i..i + phrase_len];
                let phrase2 = &words[i + phrase_len..i + phrase_len * 2];

                let match_phrase = phrase1.iter().zip(phrase2.iter()).all(|(a, b)| a.eq_ignore_ascii_case(b));
                if match_phrase {
                    final_words.extend_from_slice(phrase1);
                    i += phrase_len * 2;
                    phrase_found = true;
                    break;
                }
            }
        }

        if !phrase_found {
            final_words.push(words[i]);
            i += 1;
        }
    }

    final_words
}

// Calculate repetition ratio in text
fn calculate_repetition_ratio(text: &str) -> f32 {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 4 {
        return 0.0;
    }

    let mut word_counts = HashMap::new();
    for word in &words {
        *word_counts.entry(word.to_lowercase()).or_insert(0) += 1;
    }

    let total_words = words.len() as f32;
    let repeated_words: usize = word_counts.values().map(|&count| if count > 1 { count - 1 } else { 0 }).sum();

    repeated_words as f32 / total_words
}

fn samples_to_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, String> {
    if samples.is_empty() {
        return Err("Audio samples empty".to_string());
    }

    // DC offset removal: microphone bias can shift center and degrade log-mel spectrograms
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;

    // Dynamic gain normalization: Whisper log-mel spectrogram features require good signal amplitude.
    let centered_samples: Vec<f32> = samples.iter().map(|&s| s - mean).collect();
    let peak = centered_samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    let gain = if peak > 0.005 && peak < 0.70 {
        // Boost quiet audio up to 3.5x while keeping headroom below clipping
        (0.85 / peak).min(3.5)
    } else {
        1.0
    };

    let num_samples = centered_samples.len();
    let data_size = (num_samples * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_size as usize);

    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes());  // mono
    wav.extend_from_slice(&1u16.to_le_bytes());  // 1 channel
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());

    for &s in &centered_samples {
        let normalized = s * gain;
        let clamped = normalized.max(-1.0).min(1.0);
        wav.extend_from_slice(&((clamped * 32767.0) as i16).to_le_bytes());
    }
    Ok(wav)
}

/// Validate transcription model is ready
pub async fn validate_transcription_model_ready<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(), String> {
    let state = app.state::<crate::state::AppState>();
    let pool = state.db_manager.pool();
    let api_key = match crate::database::repositories::setting::SettingsRepository::get_transcript_api_key(pool, "groq").await {
        Ok(Some(k)) if !k.is_empty() => k,
        _ => match crate::database::repositories::setting::SettingsRepository::get_api_key(pool, "groq").await {
            Ok(Some(k)) => k,
            _ => std::env::var("GROQ_API_KEY").unwrap_or_default(),
        }
    };

    if api_key.is_empty() {
        return Err("Groq API key not configured. Set it in Settings > Transcription.".to_string());
    }
    Ok(())
}

/// Get or initialize the transcription engine
pub async fn get_or_init_transcription_engine<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<TranscriptionEngine, String> {
    let state = app.state::<crate::state::AppState>();
    let pool = state.db_manager.pool();
    let api_key = match crate::database::repositories::setting::SettingsRepository::get_transcript_api_key(pool, "groq").await {
        Ok(Some(k)) if !k.is_empty() => k,
        _ => match crate::database::repositories::setting::SettingsRepository::get_api_key(pool, "groq").await {
            Ok(Some(k)) => k,
            _ => std::env::var("GROQ_API_KEY").unwrap_or_default(),
        }
    };

    if api_key.is_empty() {
        return Err("Groq API key not configured".to_string());
    }

    let model = match crate::api::api::api_get_transcript_config(
        app.clone(), app.clone().state(), None,
    ).await {
        Ok(Some(config)) if !config.model.is_empty() => config.model,
        _ => "whisper-large-v3".to_string(),
    };

    info!("Using Groq transcription: model={}", model);
    Ok(TranscriptionEngine::GroqApi { api_key, model })
}
