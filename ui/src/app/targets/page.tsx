"use client";

import { TargetsConfig } from "@/components/targets-config";
import { useServer } from "@/lib/server-context";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";

export default function TargetsPage() {
  const { config, setConfig, connectionError } = useServer();

  return (
    <div className="container mx-auto py-6">
      {connectionError ? (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{connectionError}</AlertDescription>
        </Alert>
      ) : (
        <div>
          <div className="mb-6">
            <h1 className="text-2xl font-bold tracking-tight">Target Configuration</h1>
            <p className="text-lg text-muted-foreground mt-1">
              Configure the targets for your proxy server
            </p>
          </div>

          <div className="mt-4">
            <TargetsConfig config={config} onConfigChange={setConfig} />
          </div>
        </div>
      )}
    </div>
  );
}
