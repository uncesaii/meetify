from pydantic import BaseModel
from typing import List, Tuple, Literal
import logging
import os
import httpx
import json
from dotenv import load_dotenv
from db import DatabaseManager


# Set up logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(levelname)s - [%(filename)s:%(lineno)d] - %(message)s'
)
logger = logging.getLogger(__name__)

load_dotenv()

db = DatabaseManager()

class Block(BaseModel):
    id: str
    type: Literal['bullet', 'heading1', 'heading2', 'text']
    content: str
    color: str

class Section(BaseModel):
    title: str
    blocks: List[Block]

class MeetingNotes(BaseModel):
    meeting_name: str
    sections: List[Section]

class People(BaseModel):
    title: str
    blocks: List[Block]

class SummaryResponse(BaseModel):
    MeetingName: str
    People: People
    SessionSummary: Section
    CriticalDeadlines: Section
    KeyItemsDecisions: Section
    ImmediateActionItems: Section
    NextSteps: Section
    MeetingNotes: MeetingNotes


class TranscriptProcessor:
    def __init__(self):
        logger.info("TranscriptProcessor initialized.")
        self.db = DatabaseManager()

    async def process_transcript(self, text, model, model_name, chunk_size=5000, overlap=1000, custom_prompt=""):
        logger.info(f"Processing transcript (length {len(text)}) with provider={model}, model={model_name}")
        all_json_data = []

        try:
            step = chunk_size - overlap
            if step <= 0:
                overlap = max(0, chunk_size - 100)
                step = chunk_size - overlap

            chunks = [text[i:i+chunk_size] for i in range(0, len(text), step)]
            num_chunks = len(chunks)
            logger.info(f"Split transcript into {num_chunks} chunks.")

            schema_json = json.dumps(SummaryResponse.model_json_schema(), indent=2)

            for i, chunk in enumerate(chunks):
                logger.info(f"Processing chunk {i+1}/{num_chunks}...")
                try:
                    prompt = f"Given the following meeting transcript chunk, extract the relevant information according to the required JSON structure. If a specific section has no relevant info, return an empty list for its blocks.\n\nBlock types must be one of: text, bullet, heading1, heading2\nFor color, use gray for less important content or empty string for default.\n\nJSON schema:\n{schema_json}\n\nTranscript Chunk:\n---\n{chunk}\n---\n\nAdditional context: {custom_prompt}\n\nOutput ONLY the JSON data."

                    if model == "gemini":
                        response_text = await self._call_gemini(model_name, prompt)
                    elif model == "groq":
                        response_text = await self._call_groq(model_name, prompt)
                    elif model == "openrouter":
                        response_text = await self._call_openrouter(model_name, prompt)
                    elif model == "custom-openai":
                        response_text = await self._call_custom_openai(model_name, prompt)
                    else:
                        raise ValueError(f"Unsupported model provider: {model}")

                    summary = SummaryResponse.model_validate_json(response_text)
                    all_json_data.append(summary.model_dump_json())
                    logger.info(f"Successfully generated summary for chunk {i+1}.")

                except Exception as chunk_error:
                    logger.error(f"Error processing chunk {i+1}: {chunk_error}", exc_info=True)

            return num_chunks, all_json_data
        except Exception as e:
            logger.error(f"Error during transcript processing: {str(e)}", exc_info=True)
            raise

    async def _call_gemini(self, model_name, prompt):
        api_key = await db.get_api_key("gemini")
        if not api_key:
            raise ValueError("Gemini API key not configured")
        url = f"https://generativelanguage.googleapis.com/v1beta/models/{model_name}:generateContent?key={api_key}"
        payload = {"contents": [{"parts": [{"text": prompt}]}], "generationConfig": {"responseMimeType": "application/json"}}
        async with httpx.AsyncClient(timeout=300) as client:
            response = await client.post(url, json=payload)
            response.raise_for_status()
            data = response.json()
            return data["candidates"][0]["content"]["parts"][0]["text"]

    async def _call_groq(self, model_name, prompt):
        api_key = await db.get_api_key("groq")
        if not api_key:
            raise ValueError("Groq API key not configured")
        url = "https://api.groq.com/openai/v1/chat/completions"
        headers = {"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"}
        payload = {"model": model_name, "messages": [{"role": "user", "content": prompt}], "response_format": {"type": "json_object"}}
        async with httpx.AsyncClient(timeout=300) as client:
            response = await client.post(url, json=payload, headers=headers)
            response.raise_for_status()
            data = response.json()
            return data["choices"][0]["message"]["content"]

    async def _call_openrouter(self, model_name, prompt):
        api_key = await db.get_api_key("openrouter")
        if not api_key:
            raise ValueError("OpenRouter API key not configured")
        url = "https://openrouter.ai/api/v1/chat/completions"
        headers = {"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"}
        payload = {"model": model_name, "messages": [{"role": "user", "content": prompt}]}
        async with httpx.AsyncClient(timeout=300) as client:
            response = await client.post(url, json=payload, headers=headers)
            response.raise_for_status()
            data = response.json()
            return data["choices"][0]["message"]["content"]

    async def _call_custom_openai(self, model_name, prompt):
        api_key = await db.get_api_key("custom-openai")
        endpoint = os.getenv("CUSTOM_OPENAI_ENDPOINT", "")
        if not endpoint:
            raise ValueError("Custom OpenAI endpoint not configured")
        url = f"{endpoint.rstrip('/')}/chat/completions"
        headers = {"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"}
        payload = {"model": model_name, "messages": [{"role": "user", "content": prompt}]}
        async with httpx.AsyncClient(timeout=300) as client:
            response = await client.post(url, json=payload, headers=headers)
            response.raise_for_status()
            data = response.json()
            return data["choices"][0]["message"]["content"]

    def cleanup(self):
        logger.info("Cleaning up TranscriptProcessor resources")