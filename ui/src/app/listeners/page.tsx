"use client";

import { ListenerConfig } from "@/components/listener-config";
import { useServer } from "@/lib/server-context";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useState } from "react";

export default function ListenersPage() {
  const { connectionError } = useServer();
  const [isAddingListener, setIsAddingListener] = useState(false);

  return (
    <div className="container mx-auto py-8 px-4">
      <div className="flex flex-row items-center justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Listeners</h1>
          <p className="text-muted-foreground mt-1">
            Configure and manage listeners for your gateway
          </p>
        </div>
        <Button onClick={() => setIsAddingListener(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Add Listener
        </Button>
      </div>

      {connectionError ? (
        <Alert variant="destructive" className="mb-6">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{connectionError}</AlertDescription>
        </Alert>
      ) : (
        <ListenerConfig
          isAddingListener={isAddingListener}
          setIsAddingListener={setIsAddingListener}
        />
      )}
    </div>
  );
}
