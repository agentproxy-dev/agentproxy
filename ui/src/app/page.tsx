"use client";

import { useState, useEffect } from "react";
import { SetupWizard } from "@/components/setup-wizard";
import { useLoading } from "@/lib/loading-context";
import { useServer } from "@/lib/server-context";
import { useWizard } from "@/lib/wizard-context";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";

export default function Home() {
  const { isLoading, setIsLoading } = useLoading();
  const { config, setConfig, isConnected, connectionError, listeners, refreshListeners } =
    useServer();
  const { setIsWizardVisible } = useWizard();

  const [showWizard, setShowWizard] = useState(false);
  const [wizardStarted, setWizardStarted] = useState(false);
  const [configUpdateMessage] = useState<{
    success: boolean;
    message: string;
  } | null>(null);

  // Update loading state based on connection status
  useEffect(() => {
    if (isConnected) {
      setIsLoading(false);
    }
  }, [isConnected, setIsLoading]);

  // Refresh listeners when the component mounts
  useEffect(() => {
    refreshListeners();
  }, []);

  // Effect to control wizard visibility based on listeners and setup status
  useEffect(() => {
    const setupCompleted = localStorage.getItem("agentproxy.setupCompleted");
    
    // If there are no listeners and setup is not completed or explicitly false
    if (listeners.length === 0 && (setupCompleted === null || setupCompleted === "false")) {
      setShowWizard(true);
      setWizardStarted(true);
      setIsWizardVisible(true);
    }
    // If there are listeners and setup status is not set
    else if (listeners.length > 0 && setupCompleted === null) {
      localStorage.setItem("agentproxy.setupCompleted", "true");
      setShowWizard(false);
      setWizardStarted(false);
      setIsWizardVisible(false);
    }
    // If there are no listeners but setup is completed
    else if (listeners.length === 0 && setupCompleted === "true") {
      setShowWizard(false);
      setWizardStarted(false);
      setIsWizardVisible(false);
    }
    // Default case: hide wizard
    else {
      setShowWizard(false);
      setWizardStarted(false);
      setIsWizardVisible(false);
    }
  }, [listeners, setIsWizardVisible]);

  // Effect to handle manual wizard restart
  useEffect(() => {
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === "agentproxy.setupCompleted" && e.newValue === "false") {
        setShowWizard(true);
        setWizardStarted(true);
        setIsWizardVisible(true);
      }
    };

    window.addEventListener("storage", handleStorageChange);
    return () => window.removeEventListener("storage", handleStorageChange);
  }, [setIsWizardVisible]);

  const handleConfigChange = (newConfig: any) => {
    setConfig(newConfig);
  };

  const handleWizardComplete = () => {
    localStorage.setItem("agentproxy.setupCompleted", "true");
    setShowWizard(false);
    setWizardStarted(false);
    setIsWizardVisible(false);
  };

  const handleWizardSkip = () => {
    localStorage.setItem("agentproxy.setupCompleted", "true");
    setShowWizard(false);
    setWizardStarted(false);
    setIsWizardVisible(false);
  };

  const renderContent = () => {
    if (isLoading) {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
            <p className="mt-2 text-sm text-muted-foreground">Loading...</p>
          </div>
        </div>
      );
    }

    if (showWizard) {
      return (
        <SetupWizard
          config={config}
          onConfigChange={handleConfigChange}
          onComplete={handleWizardComplete}
          onSkip={handleWizardSkip}
          serverAddress="localhost"
          serverPort={19000}
        />
      );
    }

    if (!isConnected) {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <h2 className="text-lg font-medium">Welcome to MCP Proxy</h2>
            <p className="mt-2 text-sm text-muted-foreground">
              Connecting to server at localhost:19000...
            </p>
          </div>
        </div>
      );
    }

    return (
      <div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="p-6 bg-card rounded-lg shadow-sm">
            <h3 className="text-lg font-medium mb-2">Listener</h3>
            <p className="text-muted-foreground">
              {config.listeners.length > 0 && config.listeners[0].sse
                ? `SSE on ${config.listeners[0].sse.address || config.listeners[0].sse.host || "0.0.0.0"}:${config.listeners[0].sse.port || "5555"}`
                : "Not configured"}
            </p>
          </div>
          <div className="p-6 bg-card rounded-lg shadow-sm">
            <h3 className="text-lg font-medium mb-2">Target Servers</h3>
            <p className="text-muted-foreground">
              {config.targets.length} target
              {config.targets.length !== 1 ? "s" : ""} configured
            </p>
          </div>
          <div className="p-6 bg-card rounded-lg shadow-sm">
            <h3 className="text-lg font-medium mb-2">Security Policies</h3>
            <p className="text-muted-foreground">
              {config.policies?.length} polic
              {config.policies?.length !== 1 ? "ies" : "y"} configured
            </p>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="container mx-auto py-8 px-4">
      {configUpdateMessage && (
        <div
          className={`mb-4 rounded-md p-4 ${configUpdateMessage.success ? "bg-green-100 text-green-800" : "bg-destructive/10 text-destructive"}`}
        >
          <p>{configUpdateMessage.message}</p>
        </div>
      )}

      {connectionError && (
        <Alert variant="destructive" className="mb-4">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{connectionError}</AlertDescription>
        </Alert>
      )}

      {!isLoading && !showWizard && isConnected && (
        <div className="flex justify-between items-center mb-6">
          <div>
            <h1 className="text-3xl font-bold tracking-tight">Overview</h1>
            <p className="text-muted-foreground mt-1">
              Monitor your proxy server's configuration and status
            </p>
          </div>
        </div>
      )}

      {renderContent()}
    </div>
  );
}
