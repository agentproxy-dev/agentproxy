"use client";

import { PoliciesConfig } from "@/components/policies-config";
import { useServer } from "@/lib/server-context";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { RBACConfig } from "@/lib/types";

export default function PoliciesPage() {
  const { policies, connectionError } = useServer();

  const handleAddPolicy = (_policy: RBACConfig) => {
    // Policy management is now handled through the listener
    console.log("Policy management is now handled through the listener");
  };

  const handleRemovePolicy = (_index: number) => {
    // Policy management is now handled through the listener
    console.log("Policy management is now handled through the listener");
  };

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
            <h1 className="text-2xl font-bold tracking-tight">Policy Configuration</h1>
            <p className="text-lg text-muted-foreground mt-1">
              Configure the policies for your proxy server
            </p>
          </div>

          <div className="mt-4">
            <PoliciesConfig
              policies={policies}
              addPolicy={handleAddPolicy}
              removePolicy={handleRemovePolicy}
            />
          </div>
        </div>
      )}
    </div>
  );
}
