"use client";

import { useState } from "react";
import { Config } from "@/lib/types";
import { WelcomeStep } from "./WelcomeStep";
import { ListenerStep } from "./ListenerStep";
import { TargetsStep } from "./TargetsStep";

interface SetupWizardProps {
  config: Config;
  onConfigChange: (config: Config) => void;
  onComplete: () => void;
  onSkip: () => void;
}

export function SetupWizard({ config, onConfigChange, onComplete, onSkip }: SetupWizardProps) {
  const [step, setStep] = useState(1);
  const totalSteps = 3;

  const renderStep = () => {
    switch (step) {
      case 1:
        return <WelcomeStep onNext={() => setStep(2)} onSkip={onSkip} />;
      case 2:
        return (
          <ListenerStep
            onNext={() => setStep(3)}
            onPrevious={() => setStep(1)}
            config={config}
            onConfigChange={onConfigChange}
          />
        );
      case 3:
        return (
          <TargetsStep
            onNext={onComplete}
            onPrevious={() => setStep(2)}
            config={config}
            onConfigChange={onConfigChange}
          />
        );
      default:
        return null;
    }
  };

  return (
    <div className="fixed inset-0 flex items-center justify-center bg-gradient-to-br from-background via-background/95 to-muted/30">
      <div className="w-full max-w-2xl px-4">
        {renderStep()}
        <div className="flex justify-center mt-4">
          <div className="flex space-x-2">
            {Array.from({ length: totalSteps }).map((_, i) => (
              <div
                key={i}
                className={`h-2 w-2 rounded-full ${i + 1 === step ? "bg-primary" : "bg-muted"}`}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
