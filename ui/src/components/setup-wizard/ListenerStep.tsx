import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { AgentgatewayLogo } from "@/components/agentgateway-logo";
import { ArrowLeft, ArrowRight } from "lucide-react";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Config, Listener, ListenerProtocol } from "@/lib/types";
import { createListener } from "@/lib/api";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";

interface ListenerStepProps {
  onNext: () => void;
  onPrevious: () => void;
  config: Config;
  onConfigChange: (config: Config) => void;
}

export function ListenerStep({ onNext, onPrevious, config, onConfigChange }: ListenerStepProps) {
  const [listenerAddress, setListenerAddress] = useState("0.0.0.0");
  const [listenerPort, setListenerPort] = useState("5555");
  const [selectedProtocol, setSelectedProtocol] = useState<ListenerProtocol>(ListenerProtocol.MCP);
  const [isUpdatingListener, setIsUpdatingListener] = useState(false);

  const updateListenerConfig = async () => {
    if (!listenerAddress.trim()) {
      toast.error("Listener address is required.");
      return;
    }
    if (!listenerPort.trim()) {
      toast.error("Listener port is required.");
      return;
    }

    const portNumber = parseInt(listenerPort, 10);
    if (isNaN(portNumber) || portNumber <= 0 || portNumber > 65535) {
      toast.error("Invalid port number. Must be between 1 and 65535.");
      return;
    }

    setIsUpdatingListener(true);

    try {
      const newListener: Listener = {
        name: "default",
        protocol: selectedProtocol,
        sse: {
          address: listenerAddress,
          port: portNumber,
          tls: undefined,
          rbac: [],
        },
      };

      const newConfig = {
        ...config,
        listeners: [newListener],
      };

      onConfigChange(newConfig);

      await createListener(newListener);

      onNext();
    } catch (err) {
      console.error("Error updating listener configuration:", err);
      toast.error(err instanceof Error ? err.message : "Failed to update listener configuration");
    } finally {
      setIsUpdatingListener(false);
    }
  };

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <div className="flex justify-center mb-6">
          <AgentgatewayLogo className="h-12" />
        </div>
        <CardTitle className="text-center">Configure Listener</CardTitle>
        <CardDescription className="text-center">
          Set up your first listener to start accepting connections
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          <div className="space-y-2">
            <h3 className="font-medium">What is a Listener?</h3>
            <p className="text-sm text-muted-foreground">
              A listener is a network endpoint that accepts incoming connections. You can configure
              the protocol (MCP or A2A), address, and port. MCP listeners support connections from
              MCP Servers and OpenAPI endpoints, while A2A listeners handle Google&apos;s
              Agent2Agent protocol.
            </p>
          </div>

          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Protocol</Label>

              <RadioGroup
                value={selectedProtocol}
                onValueChange={(value) => setSelectedProtocol(value as ListenerProtocol)}
                className="flex space-x-4"
              >
                <div className="flex items-center space-x-2">
                  <RadioGroupItem value={ListenerProtocol.MCP} id="mcp-protocol" />
                  <Label htmlFor="mcp-protocol">MCP (MCP Server / OpenAPI)</Label>
                </div>
                <div className="flex items-center space-x-2">
                  <RadioGroupItem value={ListenerProtocol.A2A} id="a2a-protocol" />
                  <Label htmlFor="a2a-protocol">A2A (Agent2Agent)</Label>
                </div>
              </RadioGroup>
              <p className="text-xs text-muted-foreground">
                Choose the protocol the listener will handle.
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="listenerAddress">Address</Label>
              <Input
                id="listenerAddress"
                value={listenerAddress}
                onChange={(e) => setListenerAddress(e.target.value)}
                placeholder="e.g., 0.0.0.0"
              />
              <p className="text-xs text-muted-foreground">
                The IP address the listener is bound to.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="listenerPort">Port</Label>
              <Input
                id="listenerPort"
                value={listenerPort}
                onChange={(e) => setListenerPort(e.target.value)}
                placeholder="e.g., 5555"
                type="number"
              />
              <p className="text-xs text-muted-foreground">
                The port number the listener is using.
              </p>
            </div>
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
