import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { SSEClientTransport } from "@modelcontextprotocol/sdk/client/sse.js";
import { ClientRequest, ServerCapabilities } from "@modelcontextprotocol/sdk/types.js";
import { RequestOptions } from "@modelcontextprotocol/sdk/shared/protocol.js";
import { useState } from "react";
import { toast } from "sonner";
import { z } from "zod";

type ConnectionStatus = "disconnected" | "connected" | "error" | "error-connecting-to-proxy";

interface UseMCPServerProps {
  sseUrl: string;
  resetTimeoutOnProgress?: boolean;
  timeout?: number;
  maxTotalTimeout?: number;
}

export function useMCPServer({
  sseUrl,
  resetTimeoutOnProgress = false,
  timeout = 30000,
  maxTotalTimeout = 300000,
}: UseMCPServerProps) {
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>("disconnected");
  const [serverCapabilities, setServerCapabilities] = useState<ServerCapabilities | null>(null);
  const [mcpClient, setMcpClient] = useState<Client | null>(null);

  const makeRequest = async <T extends z.ZodType>(
    request: ClientRequest,
    schema: T,
    options?: RequestOptions & { suppressToast?: boolean }
  ): Promise<z.output<T>> => {
    if (!mcpClient) {
      throw new Error("MCP client not connected");
    }
    try {
      const abortController = new AbortController();

      // prepare MCP Client request options
      const mcpRequestOptions: RequestOptions = {
        signal: options?.signal ?? abortController.signal,
        resetTimeoutOnProgress: options?.resetTimeoutOnProgress ?? resetTimeoutOnProgress,
        timeout: options?.timeout ?? timeout,
        maxTotalTimeout: options?.maxTotalTimeout ?? maxTotalTimeout,
      };

      let response;
      try {
        response = await mcpClient.request(request, schema, mcpRequestOptions);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        toast.error(errorMessage);
        throw error;
      }

      return response;
    } catch (e: unknown) {
      if (!options?.suppressToast) {
        const errorString = (e as Error).message ?? String(e);
        toast.error(errorString);
      }
      throw e;
    }
  };

  const connect = async () => {
    if (!sseUrl) {
      toast.error("No SSE URL provided");
      setConnectionStatus("error");
      return;
    }

    console.log("Connecting to MCP Server:", sseUrl);
    const client = new Client(
      {
        name: " mcp-proxy-ui",
        version: "0.1.0",
      },
      {
        capabilities: {
          sampling: {},
          roots: {
            listChanged: true,
          },
        },
      }
    );

    try {
      // Check if the URL is valid
      let url: URL;
      try {
        url = new URL(sseUrl);
      } catch (error) {
        console.error("Invalid URL:", error);
        toast.error("Invalid SSE URL");
        setConnectionStatus("error");
        return;
      }

      const clientTransport = new SSEClientTransport(url);

      try {
        await client.connect(clientTransport);

        const capabilities = client.getServerCapabilities();
        setServerCapabilities(capabilities ?? null);

        setMcpClient(client);
        setConnectionStatus("connected");
      } catch (error) {
        console.error(`Failed to connect to MCP Server: ${sseUrl}:`, error);

        // Check if the error is related to the proxy
        if (
          error instanceof Error &&
          (error.message.includes("Failed to fetch") ||
            error.message.includes("NetworkError") ||
            error.message.includes("CORS"))
        ) {
          setConnectionStatus("error-connecting-to-proxy");
          toast.error(
            "Failed to connect to the proxy server. Please check your network connection."
          );
        } else {
          setConnectionStatus("error");
          toast.error(
            `Failed to connect: ${error instanceof Error ? error.message : String(error)}`
          );
        }
      }
    } catch (e) {
      console.error(e);
      setConnectionStatus("error");
      toast.error(`Connection error: ${e instanceof Error ? e.message : String(e)}`);
    }
  };

  const disconnect = async () => {
    await mcpClient?.close();
    setMcpClient(null);
    setConnectionStatus("disconnected");
    setServerCapabilities(null);
  };

  return {
    connectionStatus,
    serverCapabilities,
    mcpClient,
    makeRequest,
    connect,
    disconnect,
  };
}
