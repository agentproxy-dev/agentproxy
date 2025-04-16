"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { ListenerConfig } from "@/components/listener-config";
import { TargetsConfig } from "@/components/targets-config";
import { Config, Target, TargetType, Listener } from "@/lib/types";
import { ArrowRight, ArrowLeft, Info, Globe, Server, Terminal, Trash2 } from "lucide-react";
import { MCPLogo } from "@/components/mcp-logo";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { createMcpTarget, createA2aTarget, fetchListeners, createListener } from "@/lib/api";
import { Badge } from "@/components/ui/badge";

interface SetupWizardProps {
  config: Config;
  onConfigChange: (config: Config) => void;
  onComplete: () => void;
  onSkip: () => void;
  serverAddress?: string;
  serverPort?: number;
}

export function SetupWizard({ 
  config, 
  onConfigChange, 
  onComplete, 
  onSkip,
  serverAddress = "0.0.0.0",
  serverPort = 19000
}: SetupWizardProps) {
  const [step, setStep] = useState(1);
  const totalSteps = 3;
  const [targetCategory, setTargetCategory] = useState<"mcp" | "a2a">("mcp");
  const [targetType, setTargetType] = useState<TargetType>("sse");
  const [targetName, setTargetName] = useState("");
  const [targetHost, setTargetHost] = useState("");
  const [targetPort, setTargetPort] = useState("");
  const [targetPath, setTargetPath] = useState("/");
  const [command, setCommand] = useState("npx");
  const [args, setArgs] = useState("");
  const [isAddingTarget, setIsAddingTarget] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [listenerAddress, setListenerAddress] = useState("0.0.0.0");
  const [listenerPort, setListenerPort] = useState("5555");
  const [isUpdatingListener, setIsUpdatingListener] = useState(false);

  // Function to update the listener configuration
  const updateListenerConfig = async () => {
    setIsUpdatingListener(true);
    setError(null);

    try {
      // Create a new listener configuration
      const newListener: Listener = {
        sse: {
          address: listenerAddress,
          port: parseInt(listenerPort, 10),
          tls: undefined,
          rbac: []
        }
      };

      // Update the config with the new listener
      const newConfig = {
        ...config,
        listeners: [newListener]
      };
      
      // Update the local state
      onConfigChange(newConfig);
      
      // Call the API to create/update the listener
      await createListener(serverAddress, serverPort, newListener);
      
      console.log("Listener configuration updated:", newListener);
      return true;
    } catch (err) {
      console.error("Error updating listener configuration:", err);
      setError(err instanceof Error ? err.message : "Failed to update listener configuration");
      return false;
    } finally {
      setIsUpdatingListener(false);
    }
  };

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

  const handleCreateTarget = async () => {
    if (!targetName) {
      setError("Target name is required");
      return;
    }

    setIsAddingTarget(true);
    setError(null);

    try {
      let newTarget: Target;

      if (targetCategory === "a2a") {
        if (!targetHost || !targetPort) {
          setError("Host and port are required for A2A targets");
          setIsAddingTarget(false);
          return;
        }
        const port = parseInt(targetPort, 10);
        if (isNaN(port)) {
          setError("Port must be a valid number");
          setIsAddingTarget(false);
          return;
        }
        newTarget = {
          name: targetName,
          a2a: {
            host: targetHost,
            port: port,
            path: targetPath,
          },
        };
        
        // Push to proxy server
        await createA2aTarget(serverAddress, serverPort, newTarget);
      } else {
        // MCP target
        if (targetType === "stdio") {
          if (!command) {
            setError("Command is required for stdio targets");
            setIsAddingTarget(false);
            return;
          }
          newTarget = {
            name: targetName,
            stdio: {
              cmd: command,
              args: args.split(" ").filter((arg) => arg.trim() !== ""),
              env: {},
            },
          };
        } else if (targetType === "openapi") {
          if (!targetHost || !targetPort) {
            setError("Host and port are required for OpenAPI targets");
            setIsAddingTarget(false);
            return;
          }
          const port = parseInt(targetPort, 10);
          if (isNaN(port)) {
            setError("Port must be a valid number");
            setIsAddingTarget(false);
            return;
          }
          newTarget = {
            name: targetName,
            openapi: {
              host: targetHost,
              port: port,
              schema: {
                file_path: "",
              },
            },
          };
        } else {
          // Default to SSE
          if (!targetHost || !targetPort) {
            setError("Host and port are required for SSE targets");
            setIsAddingTarget(false);
            return;
          }
          const port = parseInt(targetPort, 10);
          if (isNaN(port)) {
            setError("Port must be a valid number");
            setIsAddingTarget(false);
            return;
          }
          newTarget = {
            name: targetName,
            sse: {
              host: targetHost,
              port: port,
              path: targetPath,
            },
          };
        }
        
        // Push to proxy server
        await createMcpTarget(serverAddress, serverPort, newTarget);
      }

      handleAddTarget(newTarget);
      resetForm();
    } catch (err) {
      console.error("Error creating target:", err);
      setError(err instanceof Error ? err.message : "Failed to create target");
    } finally {
      setIsAddingTarget(false);
    }
  };

  const resetForm = () => {
    setTargetName("");
    setTargetHost("");
    setTargetPort("");
    setTargetPath("/");
    setCommand("npx");
    setArgs("");
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

  const renderStep = () => {
    switch (step) {
      case 1:
        return (
          <Card className="w-full max-w-2xl">
            <CardHeader>
              <div className="flex justify-center mb-6">
                <MCPLogo className="h-12" />
              </div>
              <CardTitle className="text-center">Welcome to MCP Proxy</CardTitle>
              <CardDescription className="text-center">
                Let's get your proxy server up and running in just a few steps
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <h3 className="font-medium">What is MCP Proxy?</h3>
                <p className="text-sm text-muted-foreground">
                  MCP Proxy is a powerful tool that helps you manage and secure your server connections.
                  It allows you to configure listeners, set up target servers, and implement security policies.
                </p>
              </div>
              <div className="space-y-2">
                <h3 className="font-medium">What you'll configure:</h3>
                <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
                  <li>Listener settings for your proxy server</li>
                  <li>Target servers that your proxy will forward requests to</li>
                  <li>Security policies to protect your infrastructure</li>
                </ul>
              </div>
            </CardContent>
            <CardFooter className="flex justify-between">
              <Button variant="outline" onClick={onSkip}>
                Skip Wizard
              </Button>
              <Button onClick={() => setStep(2)}>
                Start Setup
                <ArrowRight className="ml-2 h-4 w-4" />
              </Button>
            </CardFooter>
          </Card>
        );
      case 2:
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
              <Button variant="outline" onClick={() => setStep(1)}>
                <ArrowLeft className="mr-2 h-4 w-4" />
                Back
              </Button>
              <Button 
                onClick={async () => {
                  const success = await updateListenerConfig();
                  if (success) {
                    setStep(3);
                  }
                }}
                disabled={isUpdatingListener}
              >
                {isUpdatingListener ? "Updating..." : "Next"}
                <ArrowRight className="ml-2 h-4 w-4" />
              </Button>
            </CardFooter>
          </Card>
        );
      case 3:
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
                    Targets are the destination servers that your proxy will forward requests to.
                    You can add multiple targets and configure their connection settings.
                  </p>
                </div>

                <Tabs value={targetCategory} onValueChange={(value) => setTargetCategory(value as "mcp" | "a2a")}>
                  <TabsList className="grid w-full grid-cols-2">
                    <TabsTrigger value="mcp">MCP Target</TabsTrigger>
                    <TabsTrigger value="a2a">A2A Target</TabsTrigger>
                  </TabsList>
                  
                  <TabsContent value="mcp" className="space-y-4 pt-4">
                    <Alert>
                      <Info className="h-4 w-4" />
                      <AlertDescription>
                        MCP (Model Control Protocol) targets are used to connect to AI model servers that support the MCP protocol.
                        These are typically used for AI model inference and control.
                      </AlertDescription>
                    </Alert>
                    
                    <div className="space-y-4">
                      <div className="space-y-2">
                        <Label htmlFor="targetName">Target Name</Label>
                        <Input
                          id="targetName"
                          value={targetName}
                          onChange={(e) => setTargetName(e.target.value)}
                          placeholder="e.g., local-model"
                        />
                      </div>
                      
                      <div className="space-y-2">
                        <Label>Target Type</Label>
                        <Tabs value={targetType} onValueChange={(value) => setTargetType(value as TargetType)}>
                          <TabsList className="grid w-full grid-cols-3">
                            <TabsTrigger value="sse" className="flex items-center">
                              <Globe className="h-4 w-4 mr-2" />
                              SSE
                            </TabsTrigger>
                            <TabsTrigger value="stdio" className="flex items-center">
                              <Terminal className="h-4 w-4 mr-2" />
                              stdio
                            </TabsTrigger>
                            <TabsTrigger value="openapi" className="flex items-center">
                              <Server className="h-4 w-4 mr-2" />
                              OpenAPI
                            </TabsTrigger>
                          </TabsList>
                          
                          <TabsContent value="sse" className="space-y-4 pt-4">
                            <div className="space-y-2">
                              <Label htmlFor="sseHost">Host</Label>
                              <Input
                                id="sseHost"
                                value={targetHost}
                                onChange={(e) => setTargetHost(e.target.value)}
                                placeholder="e.g., localhost"
                              />
                            </div>
                            <div className="space-y-2">
                              <Label htmlFor="ssePort">Port</Label>
                              <Input
                                id="ssePort"
                                value={targetPort}
                                onChange={(e) => setTargetPort(e.target.value)}
                                placeholder="e.g., 8080"
                              />
                            </div>
                            <div className="space-y-2">
                              <Label htmlFor="ssePath">Path</Label>
                              <Input
                                id="ssePath"
                                value={targetPath}
                                onChange={(e) => setTargetPath(e.target.value)}
                                placeholder="e.g., /"
                              />
                            </div>
                          </TabsContent>
                          
                          <TabsContent value="stdio" className="space-y-4 pt-4">
                            <div className="space-y-2">
                              <Label htmlFor="command">Command</Label>
                              <Input
                                id="command"
                                value={command}
                                onChange={(e) => setCommand(e.target.value)}
                                placeholder="e.g., npx"
                              />
                            </div>
                            <div className="space-y-2">
                              <Label htmlFor="args">Arguments</Label>
                              <Input
                                id="args"
                                value={args}
                                onChange={(e) => setArgs(e.target.value)}
                                placeholder="e.g., --port 3000"
                              />
                            </div>
                          </TabsContent>
                          
                          <TabsContent value="openapi" className="space-y-4 pt-4">
                            <div className="space-y-2">
                              <Label htmlFor="openapiHost">Host</Label>
                              <Input
                                id="openapiHost"
                                value={targetHost}
                                onChange={(e) => setTargetHost(e.target.value)}
                                placeholder="e.g., localhost"
                              />
                            </div>
                            <div className="space-y-2">
                              <Label htmlFor="openapiPort">Port</Label>
                              <Input
                                id="openapiPort"
                                value={targetPort}
                                onChange={(e) => setTargetPort(e.target.value)}
                                placeholder="e.g., 8080"
                              />
                            </div>
                          </TabsContent>
                        </Tabs>
                      </div>
                      
                      {error && (
                        <Alert variant="destructive">
                          <AlertDescription>{error}</AlertDescription>
                        </Alert>
                      )}
                      
                      <Button 
                        onClick={handleCreateTarget} 
                        className="w-full"
                        disabled={isAddingTarget}
                      >
                        {isAddingTarget ? "Adding Target..." : "Add MCP Target"}
                      </Button>
                    </div>
                  </TabsContent>
                  
                  <TabsContent value="a2a" className="space-y-4 pt-4">
                    <Alert>
                      <Info className="h-4 w-4" />
                      <AlertDescription>
                        A2A (Agent-to-Agent) targets are used to connect to other agent systems that support the A2A protocol.
                        These are typically used for agent-to-agent communication and collaboration.
                      </AlertDescription>
                    </Alert>
                    <div className="space-y-4">
                      <div className="space-y-2">
                        <Label htmlFor="a2aTargetName">Target Name</Label>
                        <Input
                          id="a2aTargetName"
                          value={targetName}
                          onChange={(e) => setTargetName(e.target.value)}
                          placeholder="e.g., agent-server"
                        />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor="a2aTargetHost">Host</Label>
                        <Input
                          id="a2aTargetHost"
                          value={targetHost}
                          onChange={(e) => setTargetHost(e.target.value)}
                          placeholder="e.g., localhost"
                        />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor="a2aTargetPort">Port</Label>
                        <Input
                          id="a2aTargetPort"
                          value={targetPort}
                          onChange={(e) => setTargetPort(e.target.value)}
                          placeholder="e.g., 9090"
                        />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor="a2aTargetPath">Path</Label>
                        <Input
                          id="a2aTargetPath"
                          value={targetPath}
                          onChange={(e) => setTargetPath(e.target.value)}
                          placeholder="e.g., /"
                        />
                      </div>
                      
                      {error && (
                        <Alert variant="destructive">
                          <AlertDescription>{error}</AlertDescription>
                        </Alert>
                      )}
                      
                      <Button 
                        onClick={handleCreateTarget} 
                        className="w-full"
                        disabled={isAddingTarget}
                      >
                        {isAddingTarget ? "Adding Target..." : "Add A2A Target"}
                      </Button>
                    </div>
                  </TabsContent>
                </Tabs>

                {config.targets.length > 0 && (
                  <div className="mt-6">
                    <h3 className="font-medium mb-2">Configured Targets</h3>
                    <div className="space-y-2">
                      {config.targets.map((target, index) => (
                        <div key={index} className="flex items-center justify-between p-3 border rounded-md">
                          <div className="flex items-center space-x-2">
                            {getTargetIcon(getTargetType(target))}
                            <div>
                              <div className="font-medium">{target.name}</div>
                              <div className="text-xs text-muted-foreground">
                                {target.sse && `${target.sse.host}:${target.sse.port}${target.sse.path}`}
                                {target.stdio && `${target.stdio.cmd} ${target.stdio.args?.join(" ")}`}
                                {target.openapi && `${target.openapi.host}:${target.openapi.port}`}
                                {target.a2a && `${target.a2a.host}:${target.a2a.port}${target.a2a.path}`}
                              </div>
                            </div>
                          </div>
                          <div className="flex items-center space-x-2">
                            <Badge variant="outline">
                              {getTargetType(target)}
                            </Badge>
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
              <Button variant="outline" onClick={() => setStep(2)}>
                <ArrowLeft className="mr-2 h-4 w-4" />
                Back
              </Button>
              <Button onClick={onComplete}>
                Complete Setup
                <ArrowRight className="ml-2 h-4 w-4" />
              </Button>
            </CardFooter>
          </Card>
        );
      default:
        return null;
    }
  };

  return (
    <div className="fixed inset-0 flex items-center justify-center bg-gradient-to-br from-background via-background/95 to-muted/30">
      <div className="w-full max-w-2xl px-4">
        {renderStep()}
        <div className="flex justify-center mt-4">
          <div className="flex space-x-2">
            {Array.from({ length: totalSteps }).map((_, i) => (
              <div
                key={i}
                className={`h-2 w-2 rounded-full ${
                  i + 1 === step ? "bg-primary" : "bg-muted"
                }`}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
} 