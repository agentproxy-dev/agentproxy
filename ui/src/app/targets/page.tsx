"use client";

import { TargetsConfig } from "@/components/targets-config";
import { useServer } from "@/lib/server-context";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useRef } from "react";

export default function TargetsPage() {
  const { config, connectionError } = useServer();
  const targetsConfigRef = useRef<{ openAddTargetDialog: () => void } | null>(null);

  const handleAddTarget = () => {
    if (targetsConfigRef.current) {
      targetsConfigRef.current.openAddTargetDialog();
    }
  };

  return (
    <div className="container mx-auto py-8 px-4">
      <div className="flex flex-row items-center justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Targets</h1>
          <p className="text-muted-foreground mt-1">
            Configure and manage targets for your gateway
          </p>
        </div>
        <Button onClick={handleAddTarget}>
          <Plus className="mr-2 h-4 w-4" />
          Add Target
        </Button>
      </div>

      {connectionError ? (
        <Alert variant="destructive" className="mb-6">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{connectionError}</AlertDescription>
        </Alert>
      ) : (
        <TargetsConfig ref={targetsConfigRef} config={config} />
      )}
    </div>
  );
}
