import React from 'react';
import { Lock, Sparkles, Zap } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';

export function WelcomeStep() {
  const { goNext } = useOnboarding();

  const features = [
    {
      icon: Lock,
      title: 'Privacy first: Your recordings stay private on your device',
    },
    {
      icon: Sparkles,
      title: 'Google Gemini for instant summaries & action items',
    },
    {
      icon: Zap,
      title: 'Groq Whisper for ultra-fast, high-accuracy speech-to-text',
    },
  ];

  return (
    <OnboardingContainer
      title="Welcome to MEETIFY"
      description="Record. Transcribe. Summarize. Supercharged with Cloud AI."
      step={1}
      totalSteps={3}
      hideProgress={true}
    >
      <div className="flex flex-col items-center space-y-8">
        {/* Divider */}
        <div className="w-16 h-px bg-gray-300" />

        {/* Features Card */}
        <div className="w-full max-w-md bg-white rounded-xl border border-gray-200 shadow-sm p-6 space-y-4">
          {features.map((feature, index) => {
            const Icon = feature.icon;
            return (
              <div key={index} className="flex items-start gap-3">
                <div className="flex-shrink-0 mt-0.5">
                  <div className="w-6 h-6 rounded-full bg-blue-50 flex items-center justify-center">
                    <Icon className="w-3.5 h-3.5 text-blue-600" />
                  </div>
                </div>
                <p className="text-sm text-gray-700 leading-relaxed font-normal">{feature.title}</p>
              </div>
            );
          })}
        </div>

        {/* CTA Section */}
        <div className="w-full max-w-xs space-y-3">
          <Button
            onClick={goNext}
            className="w-full h-11 bg-gray-900 hover:bg-gray-800 text-white font-medium"
          >
            Get Started
          </Button>
          <p className="text-xs text-center text-gray-500">Takes less than 1 minute to setup</p>
        </div>
      </div>
    </OnboardingContainer>
  );
}
