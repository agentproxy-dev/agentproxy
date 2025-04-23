"use client";

import { ListenerInfo } from "@/lib/types";
import { ListenerProtocol } from "@/lib/types";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Loader2 } from "lucide-react";

interface ConnectionSettingsProps {
  listeners: ListenerInfo[];
  selectedEndpoint: string;
  selectedListenerProtocol: ListenerProtocol | null;
  authToken: string;
  isConnected: boolean;
  isConnecting: boolean;
  a2aTargets: string[];
  selectedA2aTarget: string | null;
  isLoadingA2aTargets: boolean;
  onListenerSelect: (endpoint: string) => void;
  onA2aTargetSelect: (target: string | null) => void;
  onAuthTokenChange: (token: string) => void;
  onConnect: () => void;
  onDisconnect: () => void;
}

export function ConnectionSettings({
  listeners,
  selectedEndpoint,
  selectedListenerProtocol,
  authToken,
  isConnected,
  isConnecting,
  a2aTargets,
  selectedA2aTarget,
  isLoadingA2aTargets,
  onListenerSelect,
  onA2aTargetSelect,
  onAuthTokenChange,
  onConnect,
  onDisconnect,
}: ConnectionSettingsProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Connection Settings</CardTitle>
        <CardDescription>Connect to an MCP or A2A server endpoint</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex gap-4 items-start">
          <div className="flex flex-col space-y-2">
            <Label>Listener Endpoint</Label>
            <Select
              disabled={isConnected || isConnecting}
              onValueChange={onListenerSelect}
              value={selectedEndpoint}
            >
              <SelectTrigger className="w-[250px]">
                <SelectValue placeholder="Select endpoint" />
              </SelectTrigger>
              <SelectContent>
                {listeners.length === 0 && (
                  <SelectItem value="__no_listeners_placeholder__" disabled>
                    No listeners configured
                  </SelectItem>
                )}
                {listeners.map((listener) => (
                  <SelectItem key={listener.displayEndpoint} value={listener.displayEndpoint}>
                    {listener.displayEndpoint} (
                    {listener.protocol === ListenerProtocol.A2A ? "A2A" : "MCP"})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {selectedListenerProtocol === ListenerProtocol.A2A && (
            <div className="flex flex-col space-y-2">
              <Label>A2A Target</Label>
              <Select
                disabled={
                  isConnected || isConnecting || isLoadingA2aTargets || a2aTargets.length === 0
                }
                onValueChange={(value) =>
                  onA2aTargetSelect(value === "__no_targets_placeholder__" ? null : value)
                }
                value={selectedA2aTarget || ""}
              >
                <SelectTrigger className="w-[200px]">
                  <SelectValue
                    placeholder={isLoadingA2aTargets ? "Loading targets..." : "Select target"}
                  />
                </SelectTrigger>
                <SelectContent>
                  {!isLoadingA2aTargets && a2aTargets.length === 0 && (
                    <SelectItem value="__no_targets_placeholder__" disabled>
                      No targets found
                    </SelectItem>
                  )}
                  {a2aTargets.map((targetName) => (
                    <SelectItem key={targetName} value={targetName}>
                      {targetName}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          <div className="flex flex-col space-y-2 flex-grow">
            <Label>Bearer Token (Optional)</Label>
            <Input
              placeholder="Enter token if required"
              type="password"
              value={authToken}
              onChange={(e) => onAuthTokenChange(e.target.value)}
              disabled={isConnected || isConnecting}
            />
          </div>

          <div className="flex flex-col justify-end h-full mt-[22px]">
            <Button
              onClick={isConnected ? onDisconnect : onConnect}
              disabled={
                !selectedEndpoint ||
                isConnecting ||
                (selectedListenerProtocol === ListenerProtocol.A2A && !selectedA2aTarget)
              }
              className="w-[130px]"
            >
              {isConnecting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Connecting...
                </>
              ) : isConnected ? (
                "Disconnect"
              ) : (
                "Connect"
              )}
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
