import React, { useState, useEffect } from 'react';
import { Eye, EyeOff, Mic, ExternalLink, Check, ArrowRight, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

interface GroqSetupStepProps {
  onComplete?: () => void;
}

export function GroqSetupStep({ onComplete }: GroqSetupStepProps) {
  const { completeOnboarding } = useOnboarding();
  const [apiKey, setApiKey] = useState('');
  const [showKey, setShowKey] = useState(false);
  const [isFinishing, setIsFinishing] = useState(false);

  useEffect(() => {
    // Check if a Groq API key is already configured
    const checkExistingKey = async () => {
      try {
        const key = await invoke<string | null>('api_get_api_key', { provider: 'groq' });
        if (key) {
          setApiKey(key);
        }
      } catch (err) {
        console.log('No existing Groq key found:', err);
      }
    };
    checkExistingKey();
  }, []);

  const handleFinish = async () => {
    const trimmedKey = apiKey.trim();

    if (!trimmedKey) {
      toast.error('Please enter your Groq API key or click Skip below');
      return;
    }

    setIsFinishing(true);
    try {
      await invoke('api_save_transcript_config', {
        provider: 'groq',
        model: 'whisper-large-v3',
        apiKey: trimmedKey,
        authToken: null,
      });
      toast.success('Groq API key saved!');

      await completeOnboarding();
      if (onComplete) {
        onComplete();
      }
    } catch (err) {
      console.error('Failed to complete onboarding:', err);
      if (onComplete) {
        onComplete();
      }
    } finally {
      setIsFinishing(false);
    }
  };

  const handleSkip = async () => {
    setIsFinishing(true);
    try {
      await completeOnboarding();
      if (onComplete) {
        onComplete();
      }
    } catch (err) {
      console.error('Failed to complete onboarding:', err);
      if (onComplete) {
        onComplete();
      }
    } finally {
      setIsFinishing(false);
    }
  };

  const hasKey = apiKey.trim().length > 0;

  return (
    <OnboardingContainer
      title="Connect Groq Transcription"
      description="Transcribe meetings in real-time with Whisper Large v3 powered by Groq."
      step={3}
      totalSteps={3}
    >
      <div className="flex flex-col items-center space-y-6 max-w-md mx-auto">
        {/* Feature Highlights */}
        <div className="w-full bg-emerald-50/70 border border-emerald-100 rounded-xl p-4 space-y-2">
          <div className="flex items-center gap-2 text-emerald-900 font-semibold text-sm">
            <Mic className="w-4 h-4 text-emerald-600" />
            <span>Why Groq Whisper?</span>
          </div>
          <ul className="text-xs text-emerald-800 space-y-1 list-disc list-inside">
            <li>High-accuracy speech recognition across languages</li>
            <li>Ultra-fast transcription (transcribes 1 hr of audio in seconds)</li>
            <li>Free tier available with high rate limits</li>
          </ul>
        </div>

        {/* Input Card */}
        <div className="w-full bg-white rounded-xl border border-gray-200 p-5 space-y-4 shadow-sm">
          <div>
            <div className="flex justify-between items-center mb-1.5">
              <label className="text-xs font-semibold text-gray-700 uppercase tracking-wider">
                Groq API Key <span className="text-red-500">*</span>
              </label>
              <a
                href="https://console.groq.com/keys"
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-emerald-600 hover:text-emerald-700 flex items-center gap-1 font-medium hover:underline"
              >
                <span>Get free key</span>
                <ExternalLink className="w-3 h-3" />
              </a>
            </div>

            <div className="relative">
              <Input
                type={showKey ? 'text' : 'password'}
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="Paste gsk_... key here"
                className="pr-10 font-mono text-sm h-11"
              />
              <button
                type="button"
                onClick={() => setShowKey(!showKey)}
                className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
              >
                {showKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            </div>
            <p className="text-xs text-gray-500 mt-1.5">
              Your key is stored securely on your local device.
            </p>
          </div>
        </div>

        {/* Actions */}
        <div className="w-full space-y-3 pt-2">
          <Button
            onClick={handleFinish}
            disabled={isFinishing || !hasKey}
            className={`w-full h-11 flex items-center justify-center gap-2 font-medium transition-all ${
              hasKey
                ? 'bg-emerald-600 hover:bg-emerald-700 text-white cursor-pointer shadow-sm'
                : 'bg-gray-200 text-gray-400 cursor-not-allowed hover:bg-gray-200'
            }`}
          >
            {isFinishing ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>Finishing Setup...</span>
              </>
            ) : hasKey ? (
              <>
                <span>Save & Start MEETIFY</span>
                <Check className="w-4 h-4" />
              </>
            ) : (
              <>
                <span>Enter API Key to Complete</span>
                <ArrowRight className="w-4 h-4" />
              </>
            )}
          </Button>

          <button
            type="button"
            onClick={handleSkip}
            disabled={isFinishing}
            className="w-full text-center text-xs text-gray-500 hover:text-gray-800 transition-colors py-1 hover:underline"
          >
            Skip for now (I'll set it up later in Settings)
          </button>
        </div>
      </div>
    </OnboardingContainer>
  );
}
