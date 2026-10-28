import React, { useState, useEffect } from 'react';
import { Eye, EyeOff, Sparkles, ExternalLink, Check, ArrowRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

export function GeminiSetupStep() {
  const { goNext } = useOnboarding();
  const [apiKey, setApiKey] = useState('');
  const [showKey, setShowKey] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    // Check if a Gemini API key is already configured
    const checkExistingKey = async () => {
      try {
        const key = await invoke<string | null>('api_get_api_key', { provider: 'gemini' });
        if (key) {
          setApiKey(key);
        }
      } catch (err) {
        console.log('No existing Gemini key found:', err);
      }
    };
    checkExistingKey();
  }, []);

  const handleSaveAndContinue = async () => {
    const trimmedKey = apiKey.trim();

    if (!trimmedKey) {
      toast.error('Please enter your Gemini API key or click Skip below');
      return;
    }

    setIsSaving(true);
    try {
      await invoke('api_save_model_config', {
        provider: 'gemini',
        model: 'gemini-2.0-flash',
        whisperModel: 'whisper-large-v3',
        apiKey: trimmedKey,
        customEndpoint: null,
        authToken: null,
      });
      toast.success('Gemini API key saved!');
      goNext();
    } catch (err) {
      console.error('Failed to save Gemini key:', err);
      // Still proceed if user confirmed key
      goNext();
    } finally {
      setIsSaving(false);
    }
  };

  const handleSkip = () => {
    goNext();
  };

  const hasKey = apiKey.trim().length > 0;

  return (
    <OnboardingContainer
      title="Connect Gemini AI"
      description="Power meeting summaries, key takeaways, and action items with Google Gemini."
      step={2}
      totalSteps={3}
    >
      <div className="flex flex-col items-center space-y-6 max-w-md mx-auto">
        {/* Feature Highlights */}
        <div className="w-full bg-blue-50/70 border border-blue-100 rounded-xl p-4 space-y-2">
          <div className="flex items-center gap-2 text-blue-900 font-semibold text-sm">
            <Sparkles className="w-4 h-4 text-blue-600" />
            <span>Why Google Gemini?</span>
          </div>
          <ul className="text-xs text-blue-800 space-y-1 list-disc list-inside">
            <li>Lightning-fast meeting minutes & takeaways</li>
            <li>Free tier with generous daily limits</li>
            <li>Detects action items, decisions, and follow-ups</li>
          </ul>
        </div>

        {/* Input Card */}
        <div className="w-full bg-white rounded-xl border border-gray-200 p-5 space-y-4 shadow-sm">
          <div>
            <div className="flex justify-between items-center mb-1.5">
              <label className="text-xs font-semibold text-gray-700 uppercase tracking-wider">
                Gemini API Key <span className="text-red-500">*</span>
              </label>
              <a
                href="https://aistudio.google.com/apikey"
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-blue-600 hover:text-blue-700 flex items-center gap-1 font-medium hover:underline"
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
                placeholder="Paste AIzaSy... key here"
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
            onClick={handleSaveAndContinue}
            disabled={isSaving || !hasKey}
            className={`w-full h-11 flex items-center justify-center gap-2 font-medium transition-all ${
              hasKey
                ? 'bg-blue-600 hover:bg-blue-700 text-white cursor-pointer shadow-sm'
                : 'bg-gray-200 text-gray-400 cursor-not-allowed hover:bg-gray-200'
            }`}
          >
            {hasKey ? (
              <>
                <span>Save & Continue</span>
                <Check className="w-4 h-4" />
              </>
            ) : (
              <>
                <span>Enter API Key to Continue</span>
                <ArrowRight className="w-4 h-4" />
              </>
            )}
          </Button>

          <button
            type="button"
            onClick={handleSkip}
            className="w-full text-center text-xs text-gray-500 hover:text-gray-800 transition-colors py-1 hover:underline"
          >
            Skip for now (I'll set it up later in Settings)
          </button>
        </div>
      </div>
    </OnboardingContainer>
  );
}
