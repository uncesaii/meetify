import React from 'react';
import { useOnboarding } from '@/contexts/OnboardingContext';
import {
  WelcomeStep,
  GeminiSetupStep,
  GroqSetupStep,
} from './steps';

interface OnboardingFlowProps {
  onComplete: () => void;
}

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const { currentStep } = useOnboarding();

  // 3-Step Cloud AI Onboarding Flow:
  // Step 1: Welcome - Introduce MEETIFY features
  // Step 2: Gemini Setup - Connect Gemini API key for summarization & insights (or skip)
  // Step 3: Groq Setup - Connect Groq API key for fast Whisper transcription (or skip)

  return (
    <div className="onboarding-flow">
      {currentStep === 1 && <WelcomeStep />}
      {currentStep === 2 && <GeminiSetupStep />}
      {currentStep >= 3 && <GroqSetupStep onComplete={onComplete} />}
    </div>
  );
}
