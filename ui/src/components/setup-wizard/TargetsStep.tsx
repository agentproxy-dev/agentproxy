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
import { ArrowLeft, ArrowRight, Globe, Server, Terminal, Trash2 } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Info } from "lucide-react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { TooltipProvider, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Config, Target, TargetType } from "@/lib/types";
import { MCPTargetForm } from "./targets/MCPTargetForm";
import { A2ATargetForm } from "./targets/A2ATargetForm";
import { createMcpTarget, createA2aTarget } from "@/lib/api";

interface TargetsStepProps {
  onNext: () => void;
  onPrevious: () => void;
  config: Config;
  onConfigChange: (config: Config) => void;
  serverAddress?: string;
  serverPort?: number;
}

export function TargetsStep({
  onNext,
  onPrevious,
  config,
  onConfigChange,
  serverAddress = "0.0.0.0",
  serverPort = 19000,
}: TargetsStepProps) {
  const [targetCategory, setTargetCategory] = useState<"mcp" | "a2a">("mcp");
  const [targetName, setTargetName] = useState("");
  const [isAddingTarget, setIsAddingTarget] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
      if (targetCategory === "a2a") {
        await createA2aTarget(serverAddress, serverPort, target);
      } else {
        await createMcpTarget(serverAddress, serverPort, target);
      }

      handleAddTarget(target);
      setTargetName("");
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

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <div className="flex justify-center mb-6">
          <MCPLogo className="h-12" />
        </div>
        <CardTitle className="text-center">Configure Targets</CardTitle>
        <CardDescription className="text-center">
          Add the servers that your proxy will forward requests to
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          <div className="space-y-2">
            <h3 className="font-medium">What are Targets?</h3>
            <p className="text-sm text-muted-foreground">
              Targets are the destination servers that your proxy will forward requests to. You can
              add multiple targets and configure their connection settings.
            </p>
          </div>

          <Tabs
            value={targetCategory}
            onValueChange={(value) => setTargetCategory(value as "mcp" | "a2a")}
          >
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="mcp">MCP Target</TabsTrigger>
              <TabsTrigger value="a2a">A2A Target</TabsTrigger>
            </TabsList>

            <TabsContent value="mcp" className="space-y-4 pt-4">
              <Alert>
                <Info className="h-4 w-4" />
                <AlertDescription>
                  MCP (Model Control Protocol) targets are used to connect to AI model servers that
                  support the MCP protocol. These are typically used for AI model inference and
                  control.
                </AlertDescription>
              </Alert>

              <MCPTargetForm
                targetName={targetName}
                onTargetNameChange={setTargetName}
                onSubmit={handleCreateTarget}
                isLoading={isAddingTarget}
              />
            </TabsContent>

            <TabsContent value="a2a" className="space-y-4 pt-4">
              <Alert>
                <Info className="h-4 w-4" />
                <AlertDescription>
                  A2A (Agent-to-Agent) targets are used to connect to other agent systems that
                  support the A2A protocol. These are typically used for agent-to-agent
                  communication and collaboration.
                </AlertDescription>
              </Alert>

              <A2ATargetForm
                targetName={targetName}
                onSubmit={handleCreateTarget}
                isLoading={isAddingTarget}
              />
            </TabsContent>
          </Tabs>

          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {config.targets.length > 0 && (
            <div className="mt-6">
              <h3 className="font-medium mb-2">Configured Targets</h3>
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
        <Button onClick={onNext}>
          Complete Setup
          <ArrowRight className="ml-2 h-4 w-4" />
        </Button>
      </CardFooter>
    </Card>
  );
}
