// audio/transcription/mod.rs
//
// Transcription module: Provider abstraction, engine management, and worker pool.
// Meetify uses Groq Whisper API for transcription (no local models).

pub mod provider;
pub mod engine;
pub mod worker;

// Re-export commonly used types
pub use provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};
pub use engine::{
    TranscriptionEngine,
    validate_transcription_model_ready,
    get_or_init_transcription_engine,
};
pub use worker::{
    start_transcription_task,
    reset_speech_detected_flag,
    TranscriptUpdate
};
