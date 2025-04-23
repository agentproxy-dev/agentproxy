import {
  // Core types
  AgentCard,
  JSONRPCRequest,
  JSONRPCResponse,
  JSONRPCError,
  A2ARequest,
  // Full Request types (needed for internal generics)
  SendTaskRequest,
  GetTaskRequest,
  CancelTaskRequest,
  SendTaskStreamingRequest,
  TaskResubscriptionRequest,
  SetTaskPushNotificationRequest,
  GetTaskPushNotificationRequest,
  // Specific Params types (used directly in public method signatures)
  TaskSendParams,
  TaskQueryParams, // Used by get, resubscribe
  TaskIdParams, // Used by cancel, getTaskPushNotificationConfig
  TaskPushNotificationConfig, // Used by setTaskPushNotificationConfig
  // Full Response types (needed for internal generics and result extraction)
  SendTaskResponse,
  GetTaskResponse,
  CancelTaskResponse,
  SendTaskStreamingResponse,
  SetTaskPushNotificationResponse,
  GetTaskPushNotificationResponse,
  // Response Payload types (used in public method return signatures)
  Task,
  // TaskHistory, // Not currently implemented
  // Streaming Payload types (used in public method yield signatures)
  TaskStatusUpdateEvent,
  TaskArtifactUpdateEvent,
} from "./a2a-schema"; // Use relative path for local file

// Simple error class for client-side representation of JSON-RPC errors
export class RpcError extends Error {
  // Ensure RpcError is exported
  code: number;
  data?: unknown;

  constructor(code: number, message: string, data?: unknown) {
    super(message);
    this.name = "RpcError";
    this.code = code;
    this.data = data;
  }
}

/**
 * A client implementation for the A2A protocol that communicates
 * with an A2A server over HTTP using JSON-RPC.
 */
export class A2AClient {
  private baseUrl: string;
  private cachedAgentCard: AgentCard | null = null;
  private headers: HeadersInit;

  /**
   * Creates an instance of A2AClient.
   * @param baseUrl The base URL of the A2A server endpoint (e.g., http://localhost:port).
   * @param headers Optional headers to include in requests (e.g., for authentication).
   */
  constructor(baseUrl: string, headers: HeadersInit = {}) {
    this.baseUrl = baseUrl.endsWith("/") ? baseUrl.slice(0, -1) : baseUrl;
    this.headers = headers;
  }

  private _generateRequestId(): string | number {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    } else {
      return Date.now();
    }
  }

  private async _makeHttpRequest<Req extends A2ARequest>(
    method: Req["method"],
    params: Req["params"],
    acceptHeader: "application/json" | "text/event-stream" = "application/json"
  ): Promise<Response> {
    const requestId = this._generateRequestId();
    const requestBody: JSONRPCRequest = {
      jsonrpc: "2.0",
      id: requestId,
      method: method,
      params: params,
    };

    // Construct Headers object explicitly
    const requestHeaders = new Headers({
      "Content-Type": "application/json",
      Accept: acceptHeader,
    });
    // Append instance headers
    new Headers(this.headers).forEach((value, key) => {
      requestHeaders.append(key, value);
    });

    try {
      const response = await fetch(this.baseUrl, {
        method: "POST",
        headers: requestHeaders, // Use Headers object
        body: JSON.stringify(requestBody),
      });
      return response;
    } catch (networkError) {
      console.error("Network error during RPC call:", networkError);
      throw new RpcError(
        -32603,
        `Network error: ${networkError instanceof Error ? networkError.message : String(networkError)}`,
        networkError
      );
    }
  }

  private async _handleJsonResponse<Res extends JSONRPCResponse>(
    response: Response,
    expectedMethod?: string
  ): Promise<Res["result"]> {
    let responseBody: string | null = null;
    try {
      if (!response.ok) {
        responseBody = await response.text();
        let errorData: JSONRPCError | null = null;
        try {
          const parsedError = JSON.parse(responseBody) as JSONRPCResponse;
          if (parsedError.error) {
            errorData = parsedError.error;
            throw new RpcError(errorData.code, errorData.message, errorData.data);
          }
        } catch {
          // Ignore
        }
        throw new Error(
          `HTTP error ${response.status}: ${response.statusText}${responseBody ? ` - ${responseBody}` : ""}`
        );
      }

      responseBody = await response.text();
      if (!responseBody) {
        return undefined as Res["result"];
      }
      const jsonResponse = JSON.parse(responseBody) as Res;

      if (
        typeof jsonResponse !== "object" ||
        jsonResponse === null ||
        jsonResponse.jsonrpc !== "2.0"
      ) {
        throw new RpcError(-32603, "Invalid JSON-RPC response structure received from server.");
      }

      if (jsonResponse.error) {
        throw new RpcError(
          jsonResponse.error.code,
          jsonResponse.error.message,
          jsonResponse.error.data
        );
      }

      return jsonResponse.result;
    } catch (error) {
      console.error(
        `Error processing RPC response for method ${expectedMethod || "unknown"}:`,
        error,
        responseBody ? `\nResponse Body: ${responseBody}` : ""
      );
      if (error instanceof RpcError) {
        throw error;
      } else {
        throw new RpcError(
          -32603,
          `Failed to process response: ${error instanceof Error ? error.message : String(error)}`,
          error
        );
      }
    }
  }

  private async *_handleStreamingResponse<StreamRes extends JSONRPCResponse>(
    response: Response,
    expectedMethod?: string
  ): AsyncIterable<StreamRes["result"]> {
    if (!response.ok || !response.body) {
      let errorText: string | null = null;
      try {
        errorText = await response.text();
      } catch {
        /* Ignore */
      }
      console.error(
        `HTTP error ${response.status} received for streaming method ${expectedMethod || "unknown"}.`,
        errorText ? `Response: ${errorText}` : ""
      );
      throw new Error(
        `HTTP error ${response.status}: ${response.statusText} - Failed to establish stream.`
      );
    }

    const reader = response.body.pipeThrough(new TextDecoderStream()).getReader();
    let buffer = "";

    try {
      while (true) {
        const { done, value } = await reader.read();

        if (done) {
          if (buffer.trim()) {
            console.warn(
              `SSE stream ended with partial data in buffer for method ${expectedMethod}: ${buffer}`
            );
            if (buffer.startsWith("data: ")) {
              const dataLine = buffer.substring("data: ".length).trim();
              try {
                const parsedData = JSON.parse(dataLine) as StreamRes;
                if (parsedData.error) {
                  throw new RpcError(
                    parsedData.error.code,
                    parsedData.error.message,
                    parsedData.error.data
                  );
                } else if (parsedData.result !== undefined) {
                  yield parsedData.result as StreamRes["result"];
                }
              } catch (e) {
                console.error(
                  `Failed to parse final SSE data chunk for method ${expectedMethod}:`,
                  dataLine,
                  e
                );
              }
            }
          }
          break;
        }

        buffer += value;
        let terminatorIndex;
        while ((terminatorIndex = buffer.indexOf("\n\n")) >= 0) {
          const message = buffer.substring(0, terminatorIndex);
          buffer = buffer.substring(terminatorIndex + 2);

          if (message.startsWith("data: ")) {
            const dataLine = message.substring("data: ".length).trim();
            if (dataLine) {
              try {
                const parsedData = JSON.parse(dataLine) as StreamRes;
                if (
                  typeof parsedData !== "object" ||
                  parsedData === null ||
                  !("jsonrpc" in parsedData && parsedData.jsonrpc === "2.0")
                ) {
                  console.error(
                    `Invalid SSE data structure received for method ${expectedMethod}:`,
                    dataLine
                  );
                  continue;
                }
                if (parsedData.error) {
                  console.error(
                    `Error received in SSE stream for method ${expectedMethod}:`,
                    parsedData.error
                  );
                  throw new RpcError(
                    parsedData.error.code,
                    parsedData.error.message,
                    parsedData.error.data
                  );
                } else if (parsedData.result !== undefined) {
                  yield parsedData.result as StreamRes["result"];
                } else {
                  console.warn(
                    `SSE data for ${expectedMethod} has neither result nor error:`,
                    parsedData
                  );
                }
              } catch (e) {
                console.error(
                  `Failed to parse SSE data line for method ${expectedMethod}:`,
                  dataLine,
                  e
                );
              }
            }
          } else if (message.trim()) {
            // console.debug(`Received non-data SSE line: ${message}`);
          }
        }
      }
    } catch (error) {
      console.error(`Error reading SSE stream for method ${expectedMethod}:`, error);
      try {
        reader.releaseLock();
      } catch {
        /* Ignore */
      }
      throw error;
    } finally {
      try {
        reader.releaseLock();
      } catch {
        /* Ignore */
      }
      console.log(`SSE stream finished for method ${expectedMethod}.`);
    }
  }

  async agentCard(): Promise<AgentCard> {
    if (this.cachedAgentCard) {
      return this.cachedAgentCard;
    }
    const cardUrl = `${this.baseUrl}/.well-known/agent.json`;
    try {
      // Construct Headers object explicitly for GET request
      const requestHeaders = new Headers({ Accept: "application/json" });
      // Append instance headers, excluding Content-Type
      new Headers(this.headers).forEach((value, key) => {
        if (key.toLowerCase() !== "content-type") {
          requestHeaders.append(key, value);
        }
      });

      console.log(`Fetching agent card from ${cardUrl}`);
      const response = await fetch(cardUrl, {
        method: "GET",
        headers: requestHeaders, // Use Headers object
      });

      if (!response.ok) {
        let errorBody: string | null = null;
        try {
          errorBody = await response.text();
        } catch {
          /* Ignore */
        }
        throw new Error(
          `HTTP error ${response.status} fetching agent card from ${cardUrl}: ${response.statusText}${errorBody ? ` - ${errorBody}` : ""}`
        );
      }

      const card = await response.json();
      this.cachedAgentCard = card as AgentCard;
      return this.cachedAgentCard;
    } catch (error) {
      console.error("Failed to fetch or parse agent card:", error);
      throw new RpcError(
        -32603,
        `Could not retrieve agent card: ${error instanceof Error ? error.message : String(error)}`,
        error
      );
    }
  }

  async sendTask(params: TaskSendParams): Promise<Task | null | undefined> {
    const httpResponse = await this._makeHttpRequest<SendTaskRequest>("tasks/send", params);
    return this._handleJsonResponse<SendTaskResponse>(httpResponse, "tasks/send");
  }

  sendTaskSubscribe(
    params: TaskSendParams
  ): AsyncIterable<TaskStatusUpdateEvent | TaskArtifactUpdateEvent> {
    const streamGenerator = async function* (
      this: A2AClient
    ): AsyncIterable<TaskStatusUpdateEvent | TaskArtifactUpdateEvent> {
      const httpResponse = await this._makeHttpRequest<SendTaskStreamingRequest>(
        "tasks/sendSubscribe",
        params,
        "text/event-stream"
      );
      yield* this._handleStreamingResponse<SendTaskStreamingResponse>(
        httpResponse,
        "tasks/sendSubscribe"
      ) as AsyncIterable<TaskStatusUpdateEvent | TaskArtifactUpdateEvent>;
    }.bind(this)();
    return streamGenerator;
  }

  async getTask(params: TaskQueryParams): Promise<Task | null | undefined> {
    const httpResponse = await this._makeHttpRequest<GetTaskRequest>("tasks/get", params);
    return this._handleJsonResponse<GetTaskResponse>(httpResponse, "tasks/get");
  }

  async cancelTask(params: TaskIdParams): Promise<Task | null | undefined> {
    const httpResponse = await this._makeHttpRequest<CancelTaskRequest>("tasks/cancel", params);
    return this._handleJsonResponse<CancelTaskResponse>(httpResponse, "tasks/cancel");
  }

  async setTaskPushNotification(
    params: TaskPushNotificationConfig
  ): Promise<TaskPushNotificationConfig | null | undefined> {
    const httpResponse = await this._makeHttpRequest<SetTaskPushNotificationRequest>(
      "tasks/pushNotification/set",
      params
    );
    return this._handleJsonResponse<SetTaskPushNotificationResponse>(
      httpResponse,
      "tasks/pushNotification/set"
    );
  }

  async getTaskPushNotification(
    params: TaskIdParams
  ): Promise<TaskPushNotificationConfig | null | undefined> {
    const httpResponse = await this._makeHttpRequest<GetTaskPushNotificationRequest>(
      "tasks/pushNotification/get",
      params
    );
    return this._handleJsonResponse<GetTaskPushNotificationResponse>(
      httpResponse,
      "tasks/pushNotification/get"
    );
  }

  resubscribeTask(
    params: TaskQueryParams
  ): AsyncIterable<TaskStatusUpdateEvent | TaskArtifactUpdateEvent> {
    const streamGenerator = async function* (
      this: A2AClient
    ): AsyncIterable<TaskStatusUpdateEvent | TaskArtifactUpdateEvent> {
      const httpResponse = await this._makeHttpRequest<TaskResubscriptionRequest>(
        "tasks/resubscribe",
        params,
        "text/event-stream"
      );
      yield* this._handleStreamingResponse<SendTaskStreamingResponse>(
        httpResponse,
        "tasks/resubscribe"
      ) as AsyncIterable<TaskStatusUpdateEvent | TaskArtifactUpdateEvent>;
    }.bind(this)();
    return streamGenerator;
  }

  async supports(capability: "streaming" | "pushNotifications"): Promise<boolean> {
    try {
      const card = await this.agentCard();
      switch (capability) {
        case "streaming":
          return !!card.capabilities?.streaming;
        case "pushNotifications":
          return !!card.capabilities?.pushNotifications;
        default:
          return false;
      }
    } catch (error) {
      console.error(`Failed to determine support for capability '${capability}':`, error);
      return false;
    }
  }
}
