"use client";

import { useState, useEffect } from "react";
import { SetupWizard } from "@/components/setup-wizard";
import { useLoading } from "@/lib/loading-context";
import { useServer } from "@/lib/server-context";
import { useWizard } from "@/lib/wizard-context";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, Radio, Server, Shield, ArrowRight } from "lucide-react";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import Link from "next/link";
import { LoadingState } from "@/components/loading-state";

export default function Home() {
  const { isLoading, setIsLoading } = useLoading();
  const { config, setConfig, isConnected, connectionError } = useServer();
  const { showWizard, handleWizardComplete, handleWizardSkip } = useWizard();

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

  const handleConfigChange = (newConfig: any) => {
    setConfig(newConfig);
  };

  const renderContent = () => {
    if (isLoading) {
      return <LoadingState />;
    }
    if (showWizard) {
      return (
        <SetupWizard
          config={config}
          onConfigChange={handleConfigChange}
          onComplete={handleWizardComplete}
          onSkip={handleWizardSkip}
        />
      );
    }

    return (
      <div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <Card className="@container/card">
            <CardHeader>
              <CardDescription className="flex items-center gap-2 text-xs uppercase tracking-wider font-medium text-muted-foreground/80">
                <Radio className="h-4 w-4 text-blue-500" />
                Listeners
              </CardDescription>
              <CardTitle className="text-3xl font-semibold mt-2">
                {config.listeners.length}
              </CardTitle>
              <Link
                href="/listeners"
                className="mt-3 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1 w-fit"
              >
                View all listeners
                <ArrowRight className="h-3 w-3" />
              </Link>
            </CardHeader>
          </Card>

          <Card className="@container/card">
            <CardHeader>
              <CardDescription className="flex items-center gap-2 text-xs uppercase tracking-wider font-medium text-muted-foreground/80">
                <Server className="h-4 w-4 text-purple-500" />
                Targets
              </CardDescription>
              <CardTitle className="text-3xl font-semibold mt-2">{config.targets.length}</CardTitle>
              <Link
                href="/targets"
                className="mt-3 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1 w-fit"
              >
                View all targets
                <ArrowRight className="h-3 w-3" />
              </Link>
            </CardHeader>
          </Card>

          <Card className="@container/card">
            <CardHeader>
              <CardDescription className="flex items-center gap-2 text-xs uppercase tracking-wider font-medium text-muted-foreground/80">
                <Shield className="h-4 w-4 text-green-500" />
                Security Policies
              </CardDescription>
              <CardTitle className="text-3xl font-semibold mt-2">
                {config.policies?.length || 0}
              </CardTitle>
              <Link
                href="/policies"
                className="mt-3 text-xs text-muted-foreground hover:text-foreground flex items-center gap-1 w-fit"
              >
                View all policies
                <ArrowRight className="h-3 w-3" />
              </Link>
            </CardHeader>
          </Card>
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
              Monitor your proxy server&apos;s configuration and status
            </p>
          </div>
        </div>
      )}

      {renderContent()}
    </div>
  );
}
