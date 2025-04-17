"use client";

import React, { createContext, useContext, useState, ReactNode } from "react";

interface WizardContextType {
  isWizardVisible: boolean;
  setIsWizardVisible: (visible: boolean) => void;
}

const WizardContext = createContext<WizardContextType | undefined>(undefined);

export function WizardProvider({ children }: { children: ReactNode }) {
  const [isWizardVisible, setIsWizardVisible] = useState(false);

  return (
    <WizardContext.Provider value={{ isWizardVisible, setIsWizardVisible }}>
      {children}
    </WizardContext.Provider>
  );
}

export function useWizard() {
  const context = useContext(WizardContext);
  if (context === undefined) {
    throw new Error("useWizard must be used within a WizardProvider");
  }
  return context;
}
