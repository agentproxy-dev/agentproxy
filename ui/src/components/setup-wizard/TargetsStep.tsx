import { useState, useEffect } from "react";
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { TooltipProvider, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Config, Target, TargetType } from "@/lib/types";
import { MCPTargetForm } from "./targets/MCPTargetForm";
import { A2ATargetForm } from "./targets/A2ATargetForm";
import { createMcpTarget, createA2aTarget, fetchListeners } from "@/lib/api";
import { ListenerSelect } from "./targets/ListenerSelect";

interface TargetsStepProps {
  onNext: () => void;
  onPrevious: () => void;
  config: Config;
  onConfigChange: (config: Config) => void;
}

export function TargetsStep({ onNext, onPrevious, config, onConfigChange }: TargetsStepProps) {
  const [targetCategory, setTargetCategory] = useState<"mcp" | "a2a">("mcp");
  const [targetName, setTargetName] = useState("");
  const [selectedListeners, setSelectedListeners] = useState<string[]>([]);
  const [isAddingTarget, setIsAddingTarget] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mcpFormRef, setMcpFormRef] = useState<{ submitForm: () => Promise<void> } | null>(null);
  const [a2aFormRef, setA2aFormRef] = useState<{ submitForm: () => Promise<void> } | null>(null);

  useEffect(() => {
    const loadListeners = async () => {
      try {
        await fetchListeners();
      } catch (err) {
        console.error("Error fetching listeners:", err);
        setError("Failed to load listeners. Please try again.");
      }
    };

    loadListeners();
  }, []);

  const handleAddTarget = (target: Target) => {
    const newConfig = {
      ...config,
      targets: [...config.targets, target],
    };
    onConfigChange(newConfig);
  };

  const handleRemoveTarget = (index: number) => {
    const newConfig = {
      ...config,
      targets: config.targets.filter((_, i) => i !== index),
    };
    onConfigChange(newConfig);
  };

  const handleCreateTarget = async (target: Target) => {
    setIsAddingTarget(true);
    setError(null);

    try {
      const targetWithListeners = {
        ...target,
        listeners: selectedListeners,
      };

      if (targetCategory === "a2a") {
        await createA2aTarget(targetWithListeners);
      } else {
        await createMcpTarget(targetWithListeners);
      }

      handleAddTarget(targetWithListeners);
      setTargetName("");
      setSelectedListeners([]);
    } catch (err) {
      console.error("Error creating target:", err);
      setError(err instanceof Error ? err.message : "Failed to create target");
      throw err;
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

  const getTargetType = (target: Target): TargetType => {
    if (target.stdio) return "stdio";
    if (target.sse) return "sse";
    if (target.openapi) return "openapi";
    if (target.a2a) return "a2a";
    return "sse";
  };

  const handleNext = async () => {
    try {
      // Check if there's an incomplete target being added
      if (targetName.trim()) {
        // If there's a target name entered, try to submit the current form
        if (targetCategory === "mcp" && mcpFormRef) {
          await mcpFormRef.submitForm();
        } else if (targetCategory === "a2a" && a2aFormRef) {
          await a2aFormRef.submitForm();
        }
      } else if (config.targets.length === 0) {
        // If no targets are configured and no target is being added, show error
        setError("Please add at least one target before proceeding");
        return;
      }
      onNext();
    } catch (err) {
      console.error("Error submitting target:", err);
      setError(err instanceof Error ? err.message : "Failed to create target");
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
          Add and configure the servers that your proxy will forward requests to
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          {/* Target Configuration Form */}
          <div className="space-y-6 border rounded-lg p-4">
            <Tabs
              value={targetCategory}
              onValueChange={(value) => setTargetCategory(value as "mcp" | "a2a")}
            >
              <div className="space-y-4">
                <div className="space-y-2">
                  <Label>Target Type</Label>
                  <TabsList className="grid w-full grid-cols-2">
                    <TabsTrigger value="mcp">MCP Target</TabsTrigger>
                    <TabsTrigger value="a2a">A2A Target</TabsTrigger>
                  </TabsList>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="targetName">Target Name *</Label>
                  <Input
                    id="targetName"
                    placeholder="Enter target name"
                    value={targetName}
                    onChange={(e) => {
                      setTargetName(e.target.value);
                      setError(null); // Clear error when user starts typing
                    }}
                    required
                  />
                </div>

                <div className="space-y-2">
                  <ListenerSelect
                    selectedListeners={selectedListeners}
                    onListenersChange={setSelectedListeners}
                  />
                </div>

                <TabsContent value="mcp">
                  <MCPTargetForm
                    targetName={targetName}
                    onTargetNameChange={setTargetName}
                    onSubmit={handleCreateTarget}
                    isLoading={isAddingTarget}
                    ref={setMcpFormRef}
                  />
                </TabsContent>

                <TabsContent value="a2a">
                  <A2ATargetForm
                    targetName={targetName}
                    onSubmit={handleCreateTarget}
                    isLoading={isAddingTarget}
                    ref={setA2aFormRef}
                  />
                </TabsContent>
              </div>
            </Tabs>
          </div>

          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {/* Configured Targets List */}
          {config.targets.length > 0 && (
            <div className="space-y-2">
              <h3 className="font-medium">Configured Targets</h3>
              <div className="space-y-2">
                {config.targets.map((target, index) => (
                  <div
                    key={index}
                    className="flex items-center justify-between p-3 border rounded-md"
                  >
                    <div className="flex items-center space-x-2">
                      {getTargetIcon(getTargetType(target))}
                      <div>
                        <div className="font-medium">{target.name}</div>
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <div className="text-xs text-muted-foreground truncate max-w-[400px]">
                                {target.sse &&
                                  `${target.sse.host}:${target.sse.port}${target.sse.path}`}
                                {target.stdio &&
                                  `${target.stdio.cmd} ${target.stdio.args?.join(" ")}`}
                                {target.openapi && `${target.openapi.host}:${target.openapi.port}`}
                                {target.a2a &&
                                  `${target.a2a.host}:${target.a2a.port}${target.a2a.path}`}
                              </div>
                            </TooltipTrigger>
                            <TooltipContent>
                              {target.sse &&
                                `${target.sse.host}:${target.sse.port}${target.sse.path}`}
                              {target.stdio &&
                                `${target.stdio.cmd} ${target.stdio.args?.join(" ")}`}
                              {target.openapi && `${target.openapi.host}:${target.openapi.port}`}
                              {target.a2a &&
                                `${target.a2a.host}:${target.a2a.port}${target.a2a.path}`}
                            </TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                      </div>
                    </div>
                    <div className="flex items-center space-x-2">
                      <Badge variant="outline">{getTargetType(target)}</Badge>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleRemoveTarget(index)}
                        className="h-8 w-8"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
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
        <Button onClick={handleNext}>
          Complete Setup
          <ArrowRight className="ml-2 h-4 w-4" />
        </Button>
      </CardFooter>
    </Card>
  );
}
