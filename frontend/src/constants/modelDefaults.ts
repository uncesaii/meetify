/**
 * Default model names for Meetify.
 */

/**
 * Default Gemini model for summarization.
 */
export const DEFAULT_SUMMARY_MODEL = 'gemini-2.0-flash';

/**
 * Default Groq model for transcription (Whisper via Groq API).
 */
export const DEFAULT_TRANSCRIPTION_MODEL = 'whisper-large-v3';

/**
 * Model defaults by provider type
 */
export const MODEL_DEFAULTS = {
  gemini: DEFAULT_SUMMARY_MODEL,
  groq: 'llama-3.3-70b-versatile',
  transcription: DEFAULT_TRANSCRIPTION_MODEL,
} as const;
