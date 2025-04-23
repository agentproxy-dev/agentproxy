"use client";

import { useState, useEffect } from "react";
import { Client as McpClient } from "@modelcontextprotocol/sdk/client/index.js";
import { SSEClientTransport as McpSseTransport } from "@modelcontextprotocol/sdk/client/sse.js";
import {
  ClientRequest as McpClientRequest,
  Result as McpResult,
  Request as McpRequest,
  McpError,
  ListToolsResultSchema as McpListToolsResultSchema,
  Tool as McpTool,
} from "@modelcontextprotocol/sdk/types.js";
import { z } from "zod";
import { A2AClient, RpcError } from "@/lib/a2a-client";
import { AgentSkill, Task, TaskSendParams, Message } from "@/lib/a2a-schema";
import { useServer } from "@/lib/server-context";
import { fetchListenerTargets } from "@/lib/api";
import { ListenerInfo, ListenerProtocol } from "@/lib/types";
import { toast } from "sonner";
import { v4 as uuidv4 } from "uuid";

// Import the playground components
import { ConnectionSettings } from "@/components/playground/ConnectionSettings";
import { CapabilitiesList } from "@/components/playground/CapabilitiesList";
import { ActionPanel } from "@/components/playground/ActionPanel";
import { ResponseDisplay } from "@/components/playground/ResponseDisplay";

// Schema for MCP tool invocation response (kept for MCP)
const McpToolResponseSchema = z.any();

// Define state interfaces
interface ConnectionState {
  selectedEndpoint: string;
  selectedListenerName: string | null;
  selectedListenerProtocol: ListenerProtocol | null;
  authToken: string;
  connectionType: "mcp" | "a2a" | null;
  isConnected: boolean;
  isConnecting: boolean;
  isLoadingA2aTargets: boolean;
}

interface McpState {
  client: McpClient<McpRequest, any, McpResult> | null;
  tools: McpTool[];
  selectedTool: McpTool | null;
  paramValues: Record<string, any>;
  response: any;
}

interface A2aState {
  client: A2AClient | null;
  targets: string[];
  selectedTarget: string | null;
  skills: AgentSkill[];
  selectedSkill: AgentSkill | null;
  message: string;
  response: Task | any | null;
}

interface UiState {
  isRequestRunning: boolean;
  isLoadingCapabilities: boolean;
}

export default function PlaygroundPage() {
  const { listeners: rawListeners } = useServer();
  const [listeners, setListeners] = useState<ListenerInfo[]>([]);

  const [connectionState, setConnectionState] = useState<ConnectionState>({
    selectedEndpoint: "",
    selectedListenerName: null,
    selectedListenerProtocol: null,
    authToken: "",
    connectionType: null,
    isConnected: false,
    isConnecting: false,
    isLoadingA2aTargets: false,
  });

  const [mcpState, setMcpState] = useState<McpState>({
    client: null,
    tools: [],
    selectedTool: null,
    paramValues: {},
    response: null,
  });

  const [a2aState, setA2aState] = useState<A2aState>({
    client: null,
    targets: [],
    selectedTarget: null,
    skills: [],
    selectedSkill: null,
    message: "",
    response: null,
  });

  const [uiState, setUiState] = useState<UiState>({
    isRequestRunning: false,
    isLoadingCapabilities: false,
  });

  // Process rawListeners on mount or when they change
  useEffect(() => {
    const processedListeners = rawListeners
      .map((listener) => {
        let displayEndpoint = "Unknown";
        if (listener.sse) {
          displayEndpoint = `localhost:${listener.sse.port}`;
        }
        return { ...listener, displayEndpoint };
      })
      .filter((l) => l.displayEndpoint !== "Unknown");

    setListeners(processedListeners);
  }, [rawListeners]);

  // Handle Listener Endpoint Selection Change
  const handleListenerSelect = async (endpoint: string) => {
    setConnectionState((prev) => ({
      ...prev,
      selectedEndpoint: endpoint,
      isConnected: false,
    }));
    setA2aState((prev) => ({
      ...prev,
      selectedTarget: null,
      targets: [],
    }));
    resetClientState();

    const listener = listeners.find((l) => l.displayEndpoint === endpoint);
    if (!listener) {
      setConnectionState((prev) => ({
        ...prev,
        selectedListenerName: null,
        selectedListenerProtocol: null,
      }));
      return;
    }

    setConnectionState((prev) => ({
      ...prev,
      selectedListenerName: listener.name,
      selectedListenerProtocol: listener.protocol,
    }));

    if (listener.protocol === ListenerProtocol.A2A) {
      setConnectionState((prev) => ({ ...prev, isLoadingA2aTargets: true }));
      try {
        const targets = await fetchListenerTargets(listener.name);
        const targetNames = targets.map((t) => t.name);
        setA2aState((prev) => ({ ...prev, targets: targetNames }));
        if (targets.length === 0) {
          toast.info(`A2A listener ${listener.name} has no targets configured.`);
        }
      } catch (error) {
        console.error("Failed to fetch A2A targets:", error);
        toast.error("Failed to fetch A2A targets for this listener.");
        setA2aState((prev) => ({ ...prev, targets: [] }));
      } finally {
        setConnectionState((prev) => ({ ...prev, isLoadingA2aTargets: false }));
      }
    } else {
      setA2aState((prev) => ({ ...prev, targets: [], selectedTarget: null }));
    }
  };

  // Clear client-specific state without clearing endpoint/token
  const resetClientState = () => {
    setConnectionState((prev) => ({ ...prev, connectionType: null }));
    setMcpState((prev) => ({
      ...prev,
      client: null,
      tools: [],
      selectedTool: null,
      paramValues: {},
      response: null,
    }));
    setA2aState((prev) => ({
      ...prev,
      client: null,
      skills: [],
      selectedSkill: null,
      message: "",
      response: null,
    }));
    setUiState({
      isLoadingCapabilities: false,
      isRequestRunning: false,
    });
  };

  const connect = async () => {
    const selectedListener = listeners.find(
      (l) => l.displayEndpoint === connectionState.selectedEndpoint
    );

    if (!selectedListener || !selectedListener.sse) {
      toast.error("Listener details not found or invalid.");
      return;
    }

    const protocol = selectedListener.protocol;

    if (protocol === ListenerProtocol.A2A && !a2aState.selectedTarget) {
      toast.warning("Please select an A2A target to connect to.");
      return;
    }

    setConnectionState((prev) => ({ ...prev, isConnecting: true }));
    resetClientState();

    const port = selectedListener.sse.port;
    const useTls = !!selectedListener.sse.tls;
    const httpProtocol = useTls ? "https" : "http";

    const headers: HeadersInit = {};
    if (connectionState.authToken) {
      headers["Authorization"] = `Bearer ${connectionState.authToken}`;
    }

    try {
      if (protocol === ListenerProtocol.MCP || !protocol) {
        setConnectionState((prev) => ({ ...prev, connectionType: "mcp" }));
        const connectUrl = `${httpProtocol}://localhost:${port}/sse`;
        console.log(`Connecting to MCP endpoint: ${connectUrl}`);

        const client = new McpClient(
          { name: "mcp-playground", version: "1.0.0" },
          { capabilities: {} }
        );

        const transport = new McpSseTransport(new URL(connectUrl), {
          eventSourceInit: {
            fetch: (url, init) => fetch(url, { ...init, headers }),
          },
          requestInit: { headers },
        });

        await client.connect(transport);
        setMcpState((prev) => ({ ...prev, client }));
        setConnectionState((prev) => ({ ...prev, isConnected: true }));
        toast.success("Connected to MCP endpoint");

        setUiState((prev) => ({ ...prev, isLoadingCapabilities: true }));
        const listToolsRequest: McpClientRequest = { method: "tools/list", params: {} };
        const toolsResponse = await client.request(listToolsRequest, McpListToolsResultSchema);
        setMcpState((prev) => ({ ...prev, tools: toolsResponse.tools }));
        console.log("MCP Tools:", toolsResponse.tools);
      } else if (protocol === ListenerProtocol.A2A) {
        setConnectionState((prev) => ({ ...prev, connectionType: "a2a" }));
        const baseUrl = `${httpProtocol}://localhost:${port}/${a2aState.selectedTarget}`;
        console.log(`Connecting to A2A endpoint: ${baseUrl}`);

        const client = new A2AClient(baseUrl, headers);
        setA2aState((prev) => ({ ...prev, client }));
        setConnectionState((prev) => ({ ...prev, isConnected: true }));
        toast.success(
          `Initialized A2A client for target ${a2aState.selectedTarget} (fetching Agent Card...)`
        );

        setUiState((prev) => ({ ...prev, isLoadingCapabilities: true }));
        const agentCard = await client.agentCard();
        setA2aState((prev) => ({ ...prev, skills: agentCard.skills || [] }));
        console.log("A2A Agent Card:", agentCard);
        console.log("A2A Skills:", agentCard.skills);
        toast.success(`Connected to A2A Agent: ${agentCard.name}`);
      } else {
        toast.error("Unknown listener protocol.");
        setConnectionState((prev) => ({ ...prev, connectionType: null }));
      }
    } catch (error: any) {
      console.error("Failed to connect:", error);
      const errorMessage =
        error instanceof RpcError || error instanceof McpError || error instanceof Error
          ? error.message
          : "Unknown connection error";

      if (errorMessage.includes("401") || (error instanceof RpcError && error.code === 401)) {
        toast.error("Unauthorized: Check bearer token");
      } else if (
        errorMessage.includes("Not Found") ||
        (error instanceof Error && error.message.includes("404"))
      ) {
        toast.error(`Connection failed: Endpoint or target not found (${errorMessage})`);
      } else if (errorMessage.includes("Failed to fetch")) {
        toast.error("Connection failed: Server unreachable or refused connection.");
      } else {
        toast.error(`Connection failed: ${errorMessage}`);
      }
      resetFullStateAfterDisconnect(); // Full reset on connection failure
    } finally {
      setConnectionState((prev) => ({ ...prev, isConnecting: false }));
      setUiState((prev) => ({ ...prev, isLoadingCapabilities: false }));
    }
  };

  const disconnect = async () => {
    console.log("Disconnecting...");
    if (connectionState.connectionType === "mcp" && mcpState.client) {
      try {
        await mcpState.client.close();
        console.log("MCP client closed.");
      } catch (e) {
        console.error("Error closing MCP client:", e);
      }
    }
    resetFullStateAfterDisconnect();
    toast.info("Disconnected");
  };

  // Resets everything including endpoint/token selections
  const resetFullStateAfterDisconnect = () => {
    setConnectionState({
      // Reset full connection state
      selectedEndpoint: "",
      selectedListenerName: null,
      selectedListenerProtocol: null,
      authToken: connectionState.authToken, // Keep token maybe? Or reset? Let's keep it for now.
      connectionType: null,
      isConnected: false,
      isConnecting: false,
      isLoadingA2aTargets: false,
    });
    setMcpState({
      // Reset full MCP state
      client: null,
      tools: [],
      selectedTool: null,
      paramValues: {},
      response: null,
    });
    setA2aState({
      // Reset full A2A state
      client: null,
      targets: [],
      selectedTarget: null,
      skills: [],
      selectedSkill: null,
      message: "",
      response: null,
    });
    setUiState({
      isRequestRunning: false,
      isLoadingCapabilities: false,
    });
  };

  const handleMcpToolSelect = (tool: McpTool) => {
    setMcpState((prev) => ({ ...prev, selectedTool: tool, response: null }));
    setA2aState((prev) => ({ ...prev, response: null })); // Clear other response type

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
    setMcpState((prev) => ({ ...prev, paramValues: initialParams }));
  };

  const runMcpTool = async () => {
    if (!mcpState.client || !mcpState.selectedTool) return;

    setUiState((prev) => ({ ...prev, isRequestRunning: true }));
    setMcpState((prev) => ({ ...prev, response: null }));
    setA2aState((prev) => ({ ...prev, response: null }));

    try {
      const request: McpClientRequest = {
        method: "tools/call",
        params: {
          name: mcpState.selectedTool.name,
          arguments: mcpState.paramValues,
        },
      };
      const result = await mcpState.client.request(request, McpToolResponseSchema);
      setMcpState((prev) => ({ ...prev, response: result }));
      toast.success(`Tool ${mcpState.selectedTool?.name} executed.`);
    } catch (error: any) {
      const message = error instanceof McpError ? error.message : "Failed to run tool";
      setMcpState((prev) => ({ ...prev, response: { error: message, details: error } }));
      toast.error(message);
    } finally {
      setUiState((prev) => ({ ...prev, isRequestRunning: false }));
    }
  };

  const handleA2aSkillSelect = (skill: AgentSkill) => {
    setA2aState((prev) => ({ ...prev, selectedSkill: skill, response: null, message: "" }));
    setMcpState((prev) => ({ ...prev, response: null }));
  };

  const runA2aSkill = async () => {
    if (!a2aState.client || !a2aState.selectedSkill || !a2aState.message.trim()) {
      if (!a2aState.message.trim()) toast.warning("Please enter a message for the agent.");
      return;
    }

    setUiState((prev) => ({ ...prev, isRequestRunning: true }));
    setA2aState((prev) => ({ ...prev, response: null }));
    setMcpState((prev) => ({ ...prev, response: null }));

    try {
      const message: Message = {
        role: "user",
        parts: [{ type: "text", text: a2aState.message }],
      };

      const params: TaskSendParams = {
        id: uuidv4(),
        message: message,
      };

      const taskResult = await a2aState.client.sendTask(params);
      setA2aState((prev) => ({ ...prev, response: taskResult }));
      toast.success(`Task sent to agent using skill ${a2aState.selectedSkill?.name}.`);
    } catch (error: any) {
      console.error("Failed to run A2A skill:", error);
      const message =
        error instanceof RpcError ? `Error ${error.code}: ${error.message}` : "Failed to send task";
      setA2aState((prev) => ({ ...prev, response: { error: message, details: error } }));
      toast.error(message);
    } finally {
      setUiState((prev) => ({ ...prev, isRequestRunning: false }));
    }
  };

  const handleMcpParamChange = (key: string, value: any) => {
    setMcpState((prev) => ({
      ...prev,
      paramValues: { ...prev.paramValues, [key]: value },
    }));
  };

  const handleAuthTokenChange = (token: string) => {
    setConnectionState((prev) => ({ ...prev, authToken: token }));
  };

  const handleA2aTargetSelect = (target: string | null) => {
    setA2aState((prev) => ({ ...prev, selectedTarget: target }));
  };

  const handleA2aMessageChange = (message: string) => {
    setA2aState((prev) => ({ ...prev, message }));
  };

  return (
    <div className="container mx-auto py-8 px-4 space-y-6">
      <div className="mb-6">
        <h1 className="text-3xl font-bold tracking-tight">Playground</h1>
        <p className="text-muted-foreground mt-1">Test your MCP and A2A server endpoints</p>
      </div>

      <ConnectionSettings
        listeners={listeners}
        a2aTargets={a2aState.targets}
        isLoadingA2aTargets={connectionState.isLoadingA2aTargets}
        selectedEndpoint={connectionState.selectedEndpoint}
        selectedA2aTarget={a2aState.selectedTarget}
        authToken={connectionState.authToken}
        isConnected={connectionState.isConnected}
        isConnecting={connectionState.isConnecting}
        selectedListenerProtocol={connectionState.selectedListenerProtocol}
        onListenerSelect={handleListenerSelect}
        onA2aTargetSelect={handleA2aTargetSelect}
        onAuthTokenChange={handleAuthTokenChange}
        onConnect={connect}
        onDisconnect={disconnect}
      />

      {connectionState.isConnected && (
        <>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <CapabilitiesList
              mcpTools={mcpState.tools}
              a2aSkills={a2aState.skills}
              connectionType={connectionState.connectionType}
              isLoading={uiState.isLoadingCapabilities}
              selectedMcpToolName={mcpState.selectedTool?.name ?? null}
              selectedA2aSkillId={a2aState.selectedSkill?.id ?? null}
              onMcpToolSelect={handleMcpToolSelect}
              onA2aSkillSelect={handleA2aSkillSelect}
            />

            <ActionPanel
              connectionType={connectionState.connectionType}
              mcpSelectedTool={mcpState.selectedTool}
              a2aSelectedSkill={a2aState.selectedSkill}
              mcpParamValues={mcpState.paramValues}
              a2aMessage={a2aState.message}
              isRequestRunning={uiState.isRequestRunning}
              onMcpParamChange={handleMcpParamChange}
              onA2aMessageChange={handleA2aMessageChange}
              onRunMcpTool={runMcpTool}
              onRunA2aSkill={runA2aSkill}
            />
          </div>

          {(mcpState.response || a2aState.response) && (
            <ResponseDisplay
              mcpResponse={mcpState.response}
              a2aResponse={a2aState.response}
              connectionType={connectionState.connectionType}
            />
          )}
        </>
      )}
    </div>
  );
}
