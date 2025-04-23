import { useState, useRef } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { MCPLogo } from "@/components/mcp-logo";
import { ArrowLeft, ArrowRight, Globe, Server, Terminal, Trash2 } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { TooltipProvider, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Config, ListenerProtocol, Target, TargetType, TargetWithType } from "@/lib/types";
import { MCPTargetForm } from "./targets/MCPTargetForm";
import { A2ATargetForm } from "./targets/A2ATargetForm";
import { createMcpTarget, createA2aTarget } from "@/lib/api";
import { getTargetType } from "../target-item";

interface TargetsStepProps {
  onNext: () => void;
  onPrevious: () => void;
  config: Config;
  onConfigChange: (config: Config) => void;
}

export function TargetsStep({ onNext, onPrevious, config, onConfigChange }: TargetsStepProps) {
  const listenerProtocol = config.listeners?.[0]?.protocol ?? ListenerProtocol.MCP;
  const listenerName = config.listeners?.[0]?.name;

  const [targetName, setTargetName] = useState("");
  const [isAddingTarget, setIsAddingTarget] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mcpFormRef = useRef<{ submitForm: () => Promise<void> } | null>(null);
  const a2aFormRef = useRef<{ submitForm: () => Promise<void> } | null>(null);

  const handleAddTarget = (target: TargetWithType) => {
    const newConfig = {
      ...config,
      targets: [...config.targets, target],
    };
    onConfigChange(newConfig);
    setTargetName("");
  };

  const handleRemoveTarget = (index: number) => {
    const newConfig = {
      ...config,
      targets: config.targets.filter((_, i) => i !== index),
    };
    onConfigChange(newConfig);
  };

  const handleCreateTarget = async (targetData: any) => {
    setIsAddingTarget(true);
    setError(null);

    if (!listenerName) {
      setError("Configuration Error: Listener name not found.");
      setIsAddingTarget(false);
      return;
    }

    if (!targetName.trim()) {
      setError("Target Name is required.");
      setIsAddingTarget(false);
      return;
    }

    try {
      let determinedType: TargetType;
      if (listenerProtocol === ListenerProtocol.A2A) {
        determinedType = "a2a";
        if (!targetData?.a2a) {
          throw new Error("A2A target details are missing from the form submission.");
        }
      } else {
        if (targetData?.sse) determinedType = "sse";
        else if (targetData?.stdio) determinedType = "stdio";
        else if (targetData?.openapi) determinedType = "openapi";
        else
          throw new Error(
            "Could not determine MCP target type or details are missing from the form submission."
          );
      }

      const targetWithType: TargetWithType = {
        ...(targetData as Omit<Target, "name">),
        name: targetName,
        listeners: [listenerName],
        type: determinedType as TargetWithType["type"],
      };

      if (listenerProtocol === ListenerProtocol.A2A) {
        await createA2aTarget(targetWithType);
      } else {
        await createMcpTarget(targetWithType);
      }

      handleAddTarget(targetWithType);
      setTargetName("");
    } catch (err) {
      console.error("Error creating target:", err);
      const message = err instanceof Error ? err.message : "Failed to create target";
      setError(message);
      throw new Error(message);
    } finally {
      setIsAddingTarget(false);
    }
  };

  const getTargetIcon = (type: TargetType) => {
    switch (type) {
      case "sse":
        return <Globe className="h-4 w-4" />;
      case "stdio":
        return <Terminal className="h-4 w-4" />;
      case "openapi":
        return <Server className="h-4 w-4" />;
      case "a2a":
        return <Server className="h-4 w-4" />;
      default:
        return <Server className="h-4 w-4" />;
    }
  };

  const handleNext = async () => {
    setError(null);
    let currentError = null;

    try {
      let formSubmitPromise: Promise<void> | null = null;

      if (targetName.trim()) {
        if (listenerProtocol === ListenerProtocol.MCP && mcpFormRef.current) {
          formSubmitPromise = mcpFormRef.current.submitForm();
        } else if (listenerProtocol === ListenerProtocol.A2A && a2aFormRef.current) {
          formSubmitPromise = a2aFormRef.current.submitForm();
        }
      }

      if (formSubmitPromise) {
        await formSubmitPromise;
      }

      if (config.targets.length === 0 && !targetName.trim()) {
        console.log("No targets configured, proceeding...");
      }

      if (!error) {
        onNext();
      }
    } catch (err) {
      console.error("Error during submitForm() call in handleNext:", err);
      if (!error) {
        currentError = err instanceof Error ? err.message : "Failed to save the current target.";
        setError(currentError);
      }
    }
  };

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <div className="flex justify-center mb-6">
          <MCPLogo className="h-12" />
        </div>
        <CardTitle className="text-center">Configure Targets</CardTitle>
        <CardDescription className="text-center">
          Add the {listenerProtocol === ListenerProtocol.A2A ? "A2A" : "MCP"} target(s) that your
          proxy will forward requests to.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          <div className="space-y-4 border rounded-lg p-4">
            <div className="space-y-2">
              <Label htmlFor="targetName">Target Name *</Label>
              <Input
                id="targetName"
                placeholder={`Enter unique name for the ${listenerProtocol} target`}
                value={targetName}
                onChange={(e) => {
                  setTargetName(e.target.value);
                  setError(null);
                }}
                required
              />
              <p className="text-xs text-muted-foreground">
                A unique identifier for this target configuration.
              </p>
            </div>

            {listenerProtocol === ListenerProtocol.MCP && (
              <MCPTargetForm
                targetName={targetName}
                onTargetNameChange={setTargetName}
                onSubmit={handleCreateTarget}
                isLoading={isAddingTarget}
                ref={mcpFormRef}
              />
            )}

            {listenerProtocol === ListenerProtocol.A2A && (
              <A2ATargetForm
                targetName={targetName}
                onSubmit={handleCreateTarget}
                isLoading={isAddingTarget}
                ref={a2aFormRef}
              />
            )}
          </div>

          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error || "An unspecified error occurred."}</AlertDescription>
            </Alert>
          )}

          {config.targets.length > 0 && (
            <div className="space-y-2">
              <h3 className="font-medium">Configured Targets ({config.targets.length})</h3>
              <div className="space-y-2 max-h-60 overflow-y-auto pr-2">
                {config.targets.map((target, index) => (
                  <div
                    key={index}
                    className="flex items-center justify-between p-3 border rounded-md bg-background hover:bg-muted/50"
                  >
                    <div className="flex items-center space-x-3 overflow-hidden">
                      {getTargetIcon(getTargetType(target))}
                      <div className="overflow-hidden">
                        <div className="font-medium truncate">{target.name}</div>
                        <TooltipProvider delayDuration={300}>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <div className="text-xs text-muted-foreground truncate">
                                {getTargetDetailsString(target)}
                              </div>
                            </TooltipTrigger>
                            <TooltipContent>
                              <div>{getTargetDetailsString(target)}</div>
                              {target.listeners && target.listeners.length > 0 && (
                                <div className="mt-1 pt-1 border-t text-xs">
                                  Listener: {target.listeners[0]}
                                </div>
                              )}
                            </TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                      </div>
                    </div>
                    <div className="flex items-center space-x-2 flex-shrink-0">
                      <Badge variant="secondary">{getTargetType(target).toUpperCase()}</Badge>
                      <TooltipProvider delayDuration={300}>
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              onClick={() => handleRemoveTarget(index)}
                              className="h-8 w-8 text-muted-foreground hover:text-destructive"
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>Remove target &apos;{target.name}&apos;</TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </CardContent>
      <CardFooter className="flex justify-between">
        <Button variant="outline" onClick={onPrevious}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back
        </Button>
        <Button onClick={handleNext} disabled={isAddingTarget}>
          {config.targets.length > 0 || targetName.trim()
            ? "Next (Complete Setup)"
            : "Skip & Complete Setup"}
          <ArrowRight className="ml-2 h-4 w-4" />
        </Button>
      </CardFooter>
    </Card>
  );
}

function getTargetDetailsString(target: Target): string {
  if (target.sse) return `${target.sse.host}:${target.sse.port}${target.sse.path || "/"}`;
  if (target.stdio) return `${target.stdio.cmd} ${target.stdio.args?.join(" ") || ""}`;
  if (target.openapi) return `${target.openapi.host}:${target.openapi.port}`;
  if (target.a2a) return `${target.a2a.host}:${target.a2a.port}${target.a2a.path || "/"}`;
  return "Unknown configuration";
}
