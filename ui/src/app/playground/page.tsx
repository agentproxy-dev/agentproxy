"use client";

import { useState } from "react";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { SSEClientTransport } from "@modelcontextprotocol/sdk/client/sse.js";
import {
  ClientRequest,
  Result,
  Request,
  McpError,
  ListToolsResultSchema,
  Tool,
} from "@modelcontextprotocol/sdk/types.js";
import { z } from "zod";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useServer } from "@/lib/server-context";
import { Loader2, Send } from "lucide-react";
import { Listener } from "@/lib/types";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Checkbox } from "@/components/ui/checkbox";

// Schema for tool invocation response
const ToolResponseSchema = z.any();

export default function PlaygroundPage() {
  const { listeners } = useServer();
  const [selectedEndpoint, setSelectedEndpoint] = useState<string>("");
  const [authToken, setAuthToken] = useState<string>("");
  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [isLoadingTools, setIsLoadingTools] = useState(false);
  const [isToolRunning, setIsToolRunning] = useState(false);
  const [tools, setTools] = useState<Tool[]>([]);
  const [selectedTool, setSelectedTool] = useState<Tool | null>(null);
  const [paramValues, setParamValues] = useState<Record<string, any>>({});
  const [response, setResponse] = useState<any>(null);
  const [client, setClient] = useState<Client<Request, any, Result> | null>(null);
  const [error, setError] = useState<string | null>(null);

  const connect = async () => {
    try {
      setIsConnecting(true);
      setError(null);
      const [host, port] = selectedEndpoint.split(":");

      // Create MCP client
      const mcpClient = new Client(
        {
          name: "mcp-playground",
          version: "1.0.0",
        },
        {
          capabilities: {},
        }
      );

      // Create SSE transport through the proxy
      const headers: HeadersInit = {};
      if (authToken) {
        headers["Authorization"] = `Bearer ${authToken}`;
      }

      const transport = new SSEClientTransport(new URL(`http://localhost:${port}/sse`), {
        eventSourceInit: {
          fetch: (url, init) => fetch(url, { ...init, headers }),
        },
        requestInit: {
          headers,
        },
      });

      // Connect the client
      await mcpClient.connect(transport);
      setClient(mcpClient);
      setIsConnected(true);

      // Fetch tools after connection
      setIsLoadingTools(true);
      const listToolsRequest: ClientRequest = {
        method: "tools/list",
        params: {},
      };

      const toolsResponse = await mcpClient.request(listToolsRequest, ListToolsResultSchema);
      setTools(toolsResponse.tools);
    } catch (error) {
      console.error("Failed to connect:", error);
      setError(error instanceof Error ? error.message : "Failed to connect to server");
    } finally {
      setIsConnecting(false);
      setIsLoadingTools(false);
    }
  };

  const disconnect = async () => {
    if (client) {
      await client.close();
      setClient(null);
      setIsConnected(false);
      setTools([]);
      setSelectedTool(null);
      setResponse(null);
      setError(null);
    }
  };

  const handleToolSelect = (tool: Tool) => {
    setSelectedTool(tool);
    setResponse(null);
    // Initialize parameter values with defaults based on schema
    const initialParams: Record<string, any> = {};
    Object.entries(tool.inputSchema.properties || {}).forEach(([key, prop]: [string, any]) => {
      switch (prop.type) {
        case "boolean":
          initialParams[key] = false;
          break;
        case "number":
        case "integer":
          initialParams[key] = 0;
          break;
        case "array":
          initialParams[key] = [];
          break;
        case "object":
          initialParams[key] = {};
          break;
        default:
          initialParams[key] = "";
      }
    });
    setParamValues(initialParams);
  };

  const runTool = async () => {
    if (!client || !selectedTool) return;

    try {
      setError(null);
      setIsToolRunning(true);
      const request: ClientRequest = {
        method: "tools/call",
        params: {
          name: selectedTool.name,
          arguments: paramValues,
        },
      };

      const result = await client.request(request, ToolResponseSchema);
      setResponse(result);
    } catch (error) {
      console.error("Failed to run tool:", error);
      setError(error instanceof McpError ? error.message : "Failed to run tool");
    } finally {
      setIsToolRunning(false);
    }
  };

  const getListenerEndpoint = (listener: Listener) => {
    if (listener.sse) {
      const address = "localhost";
      return `${address}:${listener.sse.port}`;
    }
    return null;
  };

  return (
    <div className="container mx-auto p-4 space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Connection Settings</CardTitle>
          <CardDescription>Connect to an MCP server endpoint</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-4">
            <Select
              disabled={isConnected}
              onValueChange={setSelectedEndpoint}
              value={selectedEndpoint}
            >
              <SelectTrigger className="w-[200px]">
                <SelectValue placeholder="Select endpoint" />
              </SelectTrigger>
              <SelectContent>
                {listeners.map((listener) => {
                  const endpoint = getListenerEndpoint(listener);
                  if (!endpoint) return null;
                  return (
                    <SelectItem key={endpoint} value={endpoint}>
                      {endpoint}
                    </SelectItem>
                  );
                })}
              </SelectContent>
            </Select>
            <Input
              placeholder="Bearer token (optional)"
              type="password"
              value={authToken}
              onChange={(e) => setAuthToken(e.target.value)}
              disabled={isConnected}
              className="flex-1"
            />
            <Button
              onClick={isConnected ? disconnect : connect}
              disabled={!selectedEndpoint || isConnecting}
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
          {error && <div className="text-sm text-red-500">{error}</div>}
        </CardContent>
      </Card>

      {isConnected && (
        <>
          <Card>
            <CardHeader>
              <CardTitle>Available Tools</CardTitle>
              <CardDescription>Select a tool to use</CardDescription>
            </CardHeader>
            <CardContent>
              {isLoadingTools ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                  <span className="ml-3 text-muted-foreground">Loading tools...</span>
                </div>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Name</TableHead>
                      <TableHead>Description</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {tools.map((tool) => (
                      <TableRow
                        key={tool.name}
                        className="cursor-pointer hover:bg-muted/50"
                        onClick={() => handleToolSelect(tool)}
                      >
                        <TableCell className="font-medium">{tool.name}</TableCell>
                        <TableCell>{tool.description}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>

          {selectedTool && (
            <Card>
              <CardHeader>
                <CardTitle>{selectedTool.name}</CardTitle>
                <CardDescription>{selectedTool.description}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {Object.entries(selectedTool.inputSchema.properties || {}).map(
                  ([key, prop]: [string, any]) => (
                    <div key={key} className="space-y-2">
                      <Label htmlFor={key}>
                        {key}
                        {Array.isArray(selectedTool.inputSchema.required) &&
                          selectedTool.inputSchema.required.includes(key) && (
                            <span className="text-red-500 ml-1">*</span>
                          )}
                      </Label>
                      {prop.type === "boolean" ? (
                        <div className="flex items-center space-x-2">
                          <Checkbox
                            id={key}
                            checked={!!paramValues[key]}
                            onCheckedChange={(checked) =>
                              setParamValues({
                                ...paramValues,
                                [key]: checked,
                              })
                            }
                          />
                          <label htmlFor={key} className="text-sm text-muted-foreground">
                            {prop.description || "Toggle this option"}
                          </label>
                        </div>
                      ) : prop.type === "string" && prop.format === "textarea" ? (
                        <Textarea
                          id={key}
                          placeholder={prop.description}
                          value={paramValues[key] || ""}
                          onChange={(e) =>
                            setParamValues({
                              ...paramValues,
                              [key]: e.target.value,
                            })
                          }
                        />
                      ) : prop.type === "number" || prop.type === "integer" ? (
                        <Input
                          type="number"
                          id={key}
                          placeholder={prop.description}
                          value={paramValues[key] || ""}
                          onChange={(e) =>
                            setParamValues({
                              ...paramValues,
                              [key]: Number(e.target.value),
                            })
                          }
                        />
                      ) : (
                        <Input
                          id={key}
                          placeholder={prop.description}
                          value={paramValues[key] || ""}
                          onChange={(e) =>
                            setParamValues({
                              ...paramValues,
                              [key]: e.target.value,
                            })
                          }
                        />
                      )}
                    </div>
                  )
                )}
                <Button onClick={runTool} disabled={isToolRunning} className="w-full">
                  {isToolRunning ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Running...
                    </>
                  ) : (
                    <>
                      <Send className="mr-2 h-4 w-4" />
                      Run Tool
                    </>
                  )}
                </Button>
              </CardContent>
            </Card>
          )}

          {response && (
            <Card>
              <CardHeader>
                <CardTitle>Response</CardTitle>
              </CardHeader>
              <CardContent>
                <pre className="bg-secondary p-4 rounded-lg overflow-auto">
                  {JSON.stringify(response, null, 2)}
                </pre>
              </CardContent>
            </Card>
          )}
        </>
      )}
    </div>
  );
}
