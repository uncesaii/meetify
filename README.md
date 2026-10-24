# Meetify

**AI-powered meeting transcription and summarization.**

Meetify captures your meetings in real-time, transcribes audio using Groq's Whisper API, and generates intelligent summaries using Gemini, Groq, OpenRouter, or any OpenAI-compatible endpoint.

## Features

- **Real-time Audio Capture** — Record system audio and microphone simultaneously
- **AI Transcription** — Powered by Groq's Whisper API for fast, accurate speech-to-text
- **Smart Summarization** — Generate structured meeting summaries with action items, key decisions, and next steps
- **Multiple LLM Providers** — Support for Gemini, Groq, OpenRouter, and Custom OpenAI-compatible endpoints
- **Meeting Management** — Browse, search, and organize your meeting history
- **Audio Playback** — Re-listen to recordings with synchronized transcript highlighting
- **Cross-Platform** — Available on Windows, macOS, and Linux

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/) (latest stable)
- [pnpm](https://pnpm.io/)
- Python 3.10+ (for the backend API)

### API Keys

You'll need at least one of these:
- **Groq API Key** (free) — for transcription and/or summarization: [console.groq.com](https://console.groq.com)
- **Gemini API Key** (free) — for summarization: [aistudio.google.com](https://aistudio.google.com/apikey)

### Setup

`ash
# Clone the repository
git clone <your-repo-url> meetify
cd meetify

# Install frontend dependencies
cd frontend
pnpm install

# Start development
pnpm tauri dev
`

### Backend (optional, for Python-based processing)

`ash
cd backend
pip install -r requirements.txt
python -m uvicorn app.main:app --host 0.0.0.0 --port 5167
`

## Configuration

Open Settings in the app to configure:

1. **Summary Model** — Choose between Gemini, Groq, OpenRouter, or a Custom OpenAI endpoint
2. **Transcription** — Uses Groq Whisper API (requires Groq API key)

## Tech Stack

- **Frontend**: Next.js, React, TypeScript, Tailwind CSS
- **Desktop**: Tauri 2.0 (Rust)
- **Backend**: FastAPI (Python)
- **AI**: Gemini API, Groq API, OpenRouter, OpenAI-compatible endpoints

## License

MIT License — See [LICENSE.md](LICENSE.md)

This project is based on open-source work. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for attribution.
