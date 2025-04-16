import { Target, TargetType } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Trash2, Globe, Terminal, Server, Power, Wrench, Network } from "lucide-react";
import { useMCPServer } from "@/hooks/use-mcp-server";
import { useState, useRef } from "react";
import {
  ListToolsResultSchema,
  CompatibilityCallToolResultSchema,
  Tool as MCPTool,
  CompatibilityCallToolResult,
} from "@modelcontextprotocol/sdk/types.js";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface TargetItemProps {
  target: Target;
  index: number;
  onDelete: (index: number) => void;
  isUpdating: boolean;
}

// Define a type for tool parameters
interface ToolParameter {
  name: string;
  description?: string;
  type?: string;
}

const getTargetIcon = (type: TargetType) => {
  switch (type) {
    case "sse":
      return <Globe className="h-4 w-4" />;
    case "stdio":
      return <Terminal className="h-4 w-4" />;
    case "openapi":
      return <Server className="h-4 w-4" />;
    case "a2a":
      return <Network className="h-4 w-4" />;
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

export default function TargetItem({ target, index, onDelete, isUpdating }: TargetItemProps) {
  // Create the proxy URL for SSE targets
  const getProxyUrl = (target: Target) => {
    if (target.sse) {
      // construct the url from the target (if the port is 80 or 443, don't include it, just make sure the scheme is correct)
      const scheme = target.sse.port === 80 || target.sse.port === 443 ? "https" : "http";
      return `${scheme}://${target.sse.host}:${target.sse.port}${target.sse.path}`;
    } else if (target.a2a) {
      // construct the url from the A2A target
      const scheme = target.a2a.port === 80 || target.a2a.port === 443 ? "https" : "http";
      return `${scheme}://${target.a2a.host}:${target.a2a.port}${target.a2a.path}`;
    }
    return "";
  };

  const { mcpClient, connectionStatus, makeRequest, connect, disconnect } = useMCPServer({
    sseUrl: getProxyUrl(target),
  });

  const [tools, setTools] = useState<MCPTool[]>([]);
  const [nextToolCursor, setNextToolCursor] = useState<string | null>(null);
  const [toolResults, setToolResults] = useState<Record<string, CompatibilityCallToolResult>>({});
  const [toolParams, setToolParams] = useState<Record<string, Record<string, string>>>({});
  const progressTokenRef = useRef<number>(0);

  const listTools = async () => {
    if (!mcpClient) return;

    try {
      const response = await makeRequest(
        {
          method: "tools/list" as const,
          params: nextToolCursor ? { cursor: nextToolCursor } : {},
        },
        ListToolsResultSchema
      );
      setTools(response.tools);
      setNextToolCursor(response.nextCursor || null);

      // Initialize parameters for each tool
      const newToolParams: Record<string, Record<string, string>> = {};
      response.tools.forEach((tool) => {
        if (hasValidParameters(tool)) {
          newToolParams[tool.name] = {};
        }
      });
      setToolParams(newToolParams);
    } catch (error) {
      console.error("Failed to list tools:", error);
    }
  };

  const callTool = async (name: string, params: Record<string, unknown>) => {
    if (!mcpClient) return;

    try {
      const response = await makeRequest(
        {
          method: "tools/call" as const,
          params: {
            name,
            arguments: params,
            _meta: {
              progressToken: progressTokenRef.current++,
            },
          },
        },
        CompatibilityCallToolResultSchema
      );
      setToolResults((prev) => ({
        ...prev,
        [name]: response,
      }));
    } catch (e) {
      const toolResult: CompatibilityCallToolResult = {
        content: [
          {
            type: "text",
            text: (e as Error).message ?? String(e),
          },
        ],
        isError: true,
      };
      setToolResults((prev) => ({
        ...prev,
        [name]: toolResult,
      }));
    }
  };

  const handleConnect = async () => {
    if (target.sse) {
      await connect();
      await listTools();
    }
  };

  const handleDisconnect = async () => {
    await disconnect();
    setTools([]);
    setNextToolCursor(null);
    setToolResults({});
    setToolParams({});
  };

  const handleToolParamChange = (toolName: string, paramName: string, value: string) => {
    setToolParams((prev) => ({
      ...prev,
      [toolName]: {
        ...(prev[toolName] || {}),
        [paramName]: value,
      },
    }));
  };

  const handleCallTool = (tool: MCPTool) => {
    if (hasValidParameters(tool)) {
      callTool(tool.name, toolParams[tool.name] || {});
    } else {
      callTool(tool.name, {});
    }
  };

  const renderTargetDetails = (target: Target) => {
    if (target.stdio) {
      return (
        <div className="text-sm text-muted-foreground">
          <p>
            Command: {target.stdio.cmd} {target.stdio.args?.join(" ")}
          </p>
        </div>
      );
    }

    if (target.sse) {
      const path = target.sse.path || "/";
      const truncatedPath = path.length > 30 ? path.substring(0, 27) + "..." : path;
      return (
        <div className="text-sm text-muted-foreground">
          <p>
            Host: {target.sse.host}:{target.sse.port}
          </p>
          <p>Path: {truncatedPath}</p>
        </div>
      );
    }

    if (target.openapi) {
      return (
        <div className="text-sm text-muted-foreground">
          <p>
            Host: {target.openapi.host}:{target.openapi.port}
          </p>
          <p>Schema: {target.openapi.schema?.file_path || "Inline schema"}</p>
        </div>
      );
    }

    return null;
  };

  const targetType = getTargetType(target);
  const isSSE = targetType === "sse";

  // Helper function to check if parameters exist and are valid
  const hasValidParameters = (tool: MCPTool | null): boolean => {
    return Boolean(
      tool && tool.parameters && Array.isArray(tool.parameters) && tool.parameters.length > 0
    );
  };

  // Helper function to get parameters as the correct type
  const getParameters = (tool: MCPTool): ToolParameter[] => {
    return tool.parameters as unknown as ToolParameter[];
  };

  return (
    <div id={`target-${index}`} className="border rounded-lg p-4 space-y-4">
      <div className="flex justify-between items-start">
        <div>
          <h4 className="font-medium">{target.name}</h4>
          <div className="flex items-center mt-1">
            <Badge variant="outline" className="mr-2 flex items-center">
              {getTargetIcon(targetType)}
              <span className="ml-1">{targetType}</span>
            </Badge>
            {renderTargetDetails(target)}
          </div>
        </div>
        <div className="flex space-x-2">
          {isSSE && (
            <>
              {connectionStatus === "disconnected" ? (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleConnect}
                  className="flex items-center"
                  disabled={isUpdating}
                >
                  <Power className="h-4 w-4 mr-1" />
                  Connect
                </Button>
              ) : (
                <>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleDisconnect}
                    className="flex items-center"
                    disabled={isUpdating}
                  >
                    <Power className="h-4 w-4 mr-1" />
                    Disconnect
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={listTools}
                    className="flex items-center"
                    disabled={isUpdating}
                  >
                    <Wrench className="h-4 w-4 mr-1" />
                    List Tools
                  </Button>
                </>
              )}
              {connectionStatus !== "disconnected" && (
                <Badge variant={connectionStatus === "connected" ? "default" : "destructive"}>
                  {connectionStatus}
                </Badge>
              )}
            </>
          )}
          <Button
            variant="ghost"
            size="icon"
            onClick={() => onDelete(index)}
            className="text-muted-foreground hover:text-destructive"
            disabled={isUpdating}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {isSSE && connectionStatus === "connected" && (
        <div className="mt-4">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center">
                <Wrench className="h-4 w-4 mr-1" />
                Available Tools
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {tools.length === 0 ? (
                <div className="text-center py-4 text-muted-foreground">
                  <p>No tools loaded yet. Click &quot;List Tools&quot; to load available tools.</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 gap-4">
                  {tools.map((tool) => (
                    <div key={tool.name} className="border rounded-md p-3">
                      <div className="flex justify-between items-start">
                        <div>
                          <div className="font-medium">{tool.name}</div>
                          <div className="text-xs text-muted-foreground">{tool.description}</div>
                        </div>
                        <Button size="sm" onClick={() => handleCallTool(tool)} className="ml-2">
                          Invoke
                        </Button>
                      </div>

                      {hasValidParameters(tool) && (
                        <div className="mt-3 space-y-2">
                          <h4 className="text-xs font-medium">Parameters</h4>
                          {getParameters(tool).map((param) => (
                            <div key={param.name} className="space-y-1">
                              <Label htmlFor={`${tool.name}-${param.name}`} className="text-xs">
                                {param.name}
                              </Label>
                              <Input
                                id={`${tool.name}-${param.name}`}
                                value={toolParams[tool.name]?.[param.name] || ""}
                                onChange={(e) =>
                                  handleToolParamChange(tool.name, param.name, e.target.value)
                                }
                                placeholder={param.description}
                                className="h-8 text-xs"
                              />
                            </div>
                          ))}
                        </div>
                      )}

                      {toolResults[tool.name] && (
                        <div className="mt-3">
                          <h4 className="text-xs font-medium">Result</h4>
                          <div
                            className={`p-2 mt-1 rounded-md text-xs ${
                              toolResults[tool.name].isError
                                ? "bg-destructive/10 text-destructive"
                                : "bg-muted"
                            }`}
                          >
                            {Array.isArray(toolResults[tool.name].content) &&
                              (
                                toolResults[tool.name].content as Array<{ text: string } | string>
                              ).map((item, i) => {
                                if (typeof item === "object" && item !== null && "text" in item) {
                                  return <div key={i}>{item.text}</div>;
                                }
                                return <div key={i}>{String(item)}</div>;
                              })}
                          </div>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
