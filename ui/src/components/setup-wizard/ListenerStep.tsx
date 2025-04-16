import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { MCPLogo } from "@/components/mcp-logo";
import { ArrowLeft, ArrowRight } from "lucide-react";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Config, Listener } from "@/lib/types";
import { createListener } from "@/lib/api";

interface ListenerStepProps {
  onNext: () => void;
  onPrevious: () => void;
  config: Config;
  onConfigChange: (config: Config) => void;
  serverAddress?: string;
  serverPort?: number;
}

export function ListenerStep({
  onNext,
  onPrevious,
  config,
  onConfigChange,
  serverAddress = "0.0.0.0",
  serverPort = 19000,
}: ListenerStepProps) {
  const [listenerAddress, setListenerAddress] = useState("0.0.0.0");
  const [listenerPort, setListenerPort] = useState("5555");
  const [isUpdatingListener, setIsUpdatingListener] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const updateListenerConfig = async () => {
    setIsUpdatingListener(true);
    setError(null);

    try {
      // Create a new listener configuration
      const newListener: Listener = {
        name: "default",
        sse: {
          address: listenerAddress,
          port: parseInt(listenerPort, 10),
          tls: undefined,
          rbac: [],
        },
      };

      // Update the config with the new listener
      const newConfig = {
        ...config,
        listeners: [newListener],
      };

      // Update the local state
      onConfigChange(newConfig);

      // Call the API to create/update the listener
      await createListener(serverAddress, serverPort, newListener);

      console.log("Listener configuration updated:", newListener);
      onNext();
      return true;
    } catch (err) {
      console.error("Error updating listener configuration:", err);
      setError(err instanceof Error ? err.message : "Failed to update listener configuration");
      return false;
    } finally {
      setIsUpdatingListener(false);
    }
  };

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <div className="flex justify-center mb-6">
          <MCPLogo className="h-12" />
        </div>
        <CardTitle className="text-center">Configure Listener</CardTitle>
        <CardDescription className="text-center">
          Set up your first listener to start accepting connections
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          <div className="space-y-2">
            <h3 className="font-medium">What is a Listener?</h3>
            <p className="text-sm text-muted-foreground">
              A listener is a network endpoint that accepts incoming connections. You can configure
              the address, port, and protocol for your listener.
            </p>
          </div>

          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="listenerAddress">Address</Label>
              <Input
                id="listenerAddress"
                value={listenerAddress}
                onChange={(e) => setListenerAddress(e.target.value)}
                placeholder="e.g., 0.0.0.0"
              />
              <p className="text-xs text-muted-foreground">
                The IP address the listener is bound to. 0.0.0.0 means it&apos;s listening on all
                interfaces.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="listenerPort">Port</Label>
              <Input
                id="listenerPort"
                value={listenerPort}
                onChange={(e) => setListenerPort(e.target.value)}
                placeholder="e.g., 5555"
              />
              <p className="text-xs text-muted-foreground">
                The port number the listener is using.
              </p>
            </div>

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
          </div>
        </div>
      </CardContent>
      <CardFooter className="flex justify-between">
        <Button variant="outline" onClick={onPrevious}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back
        </Button>
        <Button onClick={updateListenerConfig} disabled={isUpdatingListener}>
          {isUpdatingListener ? "Updating..." : "Next"}
          <ArrowRight className="ml-2 h-4 w-4" />
        </Button>
      </CardFooter>
    </Card>
  );
}
