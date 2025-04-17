import { Target, Listener, Config } from "./types";

/**
 * Updates a single target on the MCP proxy server
 */
export async function updateTarget(address: string, port: number, target: Target): Promise<void> {
  try {
    const response = await fetch(`http://${address}:${port}/targets`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(target),
    });

    if (!response.ok) {
      throw new Error(`Failed to update target: ${response.status} ${response.statusText}`);
    }
  } catch (error) {
    console.error("Error updating target:", error);
    throw error;
  }
}

/**
 * Fetches the listener configuration from the MCP proxy server
 */
export async function fetchListeners(address: string, port: number): Promise<Listener[]> {
  try {
    const response = await fetch(`http://${address}:${port}/listeners`, {});

    if (!response.ok) {
      console.error("Failed to fetch listeners:", response);
      throw new Error(`Failed to fetch listeners: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();

    // The API will return an array of listeners
    if (Array.isArray(data)) {
      return data.map((listener) => {
        if (listener.sse && listener.sse.host !== undefined && listener.sse.address === undefined) {
          return {
            ...listener,
            sse: {
              address: listener.sse.host,
              port: listener.sse.port,
              tls: listener.sse.tls,
              rbac: listener.sse.rbac,
            },
          };
        }
        return listener;
      });
    } else {
      // If the API returns a single object instead of an array, wrap it
      return [data];
    }
  } catch (error) {
    console.error("Error fetching listeners:", error);
    throw error;
  }
}

/**
 * Fetches all MCP targets from the proxy server
 */
export async function fetchMcpTargets(address: string, port: number): Promise<any[]> {
  try {
    const response = await fetch(`http://${address}:${port}/targets/mcp`, {});

    if (!response.ok) {
      throw new Error(`Failed to fetch MCP targets: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching MCP targets:", error);
    throw error;
  }
}

/**
 * Creates or updates an MCP target on the proxy server
 */
export async function createMcpTarget(address: string, port: number, target: any): Promise<void> {
  try {
    const response = await fetch(`http://${address}:${port}/targets/mcp`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(target),
    });

    if (!response.ok) {
      throw new Error(`Failed to create MCP target: ${response.status} ${response.statusText}`);
    }
  } catch (error) {
    console.error("Error creating MCP target:", error);
    throw error;
  }
}

/**
 * Fetches a specific MCP target by name
 */
export async function getMcpTarget(address: string, port: number, name: string): Promise<any> {
  try {
    const response = await fetch(`http://${address}:${port}/targets/mcp/${name}`, {});

    if (!response.ok) {
      throw new Error(`Failed to fetch MCP target: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching MCP target:", error);
    throw error;
  }
}

/**
 * Deletes an MCP target by name
 */
export async function deleteMcpTarget(address: string, port: number, name: string): Promise<void> {
  try {
    const response = await fetch(`http://${address}:${port}/targets/mcp/${name}`, {
      method: "DELETE",
    });

    if (!response.ok) {
      throw new Error(`Failed to delete MCP target: ${response.status} ${response.statusText}`);
    }
  } catch (error) {
    console.error("Error deleting MCP target:", error);
    throw error;
  }
}

/**
 * Fetches all A2A targets from the proxy server
 */
export async function fetchA2aTargets(address: string, port: number): Promise<any[]> {
  try {
    const response = await fetch(`http://${address}:${port}/targets/a2a`, {});

    if (!response.ok) {
      throw new Error(`Failed to fetch A2A targets: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching A2A targets:", error);
    throw error;
  }
}

/**
 * Creates or updates an A2A target on the proxy server
 */
export async function createA2aTarget(address: string, port: number, target: any): Promise<void> {
  try {
    const response = await fetch(`http://${address}:${port}/targets/a2a`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(target),
    });

    if (!response.ok) {
      throw new Error(`Failed to create A2A target: ${response.status} ${response.statusText}`);
    }
  } catch (error) {
    console.error("Error creating A2A target:", error);
    throw error;
  }
}

/**
 * Fetches a specific A2A target by name
 */
export async function getA2aTarget(address: string, port: number, name: string): Promise<any> {
  try {
    const response = await fetch(`http://${address}:${port}/targets/a2a/${name}`, {});

    if (!response.ok) {
      throw new Error(`Failed to fetch A2A target: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching A2A target:", error);
    throw error;
  }
}

/**
 * Deletes an A2A target by name
 */
export async function deleteA2aTarget(address: string, port: number, name: string): Promise<void> {
  try {
    const response = await fetch(`http://${address}:${port}/targets/a2a/${name}`, {
      method: "DELETE",
    });

    if (!response.ok) {
      throw new Error(`Failed to delete A2A target: ${response.status} ${response.statusText}`);
    }
  } catch (error) {
    console.error("Error deleting A2A target:", error);
    throw error;
  }
}

/**
 * Fetches targets associated with a specific listener
 */
export async function fetchListenerTargets(
  address: string,
  port: number,
  listenerName: string
): Promise<any[]> {
  try {
    const response = await fetch(`http://${address}:${port}/listeners/${listenerName}/targets`, {});

    if (!response.ok) {
      throw new Error(
        `Failed to fetch listener targets: ${response.status} ${response.statusText}`
      );
    }

    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching listener targets:", error);
    throw error;
  }
}

/**
 * Fetches a specific listener by name
 */
export async function getListener(address: string, port: number, name: string): Promise<Listener> {
  try {
    const response = await fetch(`http://${address}:${port}/listeners/${name}`, {});

    if (!response.ok) {
      throw new Error(`Failed to fetch listener: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching listener:", error);
    throw error;
  }
}

/**
 * Creates or updates a listener on the proxy server
 */
export async function createListener(
  address: string,
  port: number,
  listener: Listener
): Promise<void> {
  try {
    const response = await fetch(`http://${address}:${port}/listeners`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(listener),
    });

    if (!response.ok) {
      throw new Error(`Failed to create listener: ${response.status} ${response.statusText}`);
    }
  } catch (error) {
    console.error("Error creating listener:", error);
    throw error;
  }
}

/**
 * Creates a new listener on the MCP proxy server
 */
export async function addListener(
  address: string,
  port: number,
  listener: Listener
): Promise<void> {
  try {
    const response = await fetch(`http://${address}:${port}/listeners`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(listener),
    });

    if (!response.ok) {
      throw new Error(`Failed to add listener: ${response.status} ${response.statusText}`);
    }
  } catch (error) {
    console.error("Error adding listener:", error);
    throw error;
  }
}

/**
 * Deletes a listener from the MCP proxy server
 */
export async function deleteListener(
  address: string,
  port: number,
  listener: Listener
): Promise<void> {
  try {
    // Extract the listener name or use a default if not available
    const listenerName = listener.name || "default";

    const response = await fetch(`http://${address}:${port}/listeners/${listenerName}`, {
      method: "DELETE",
    });

    if (!response.ok) {
      throw new Error(`Failed to delete listener: ${response.status} ${response.statusText}`);
    }
  } catch (error) {
    console.error("Error deleting listener:", error);
    throw error;
  }
}

export async function getConfig(): Promise<Config> {
  const response = await fetch("/api/config");
  if (!response.ok) {
    throw new Error("Failed to fetch configuration");
  }
  return response.json();
}

export async function updateConfig(config: Config): Promise<void> {
  const response = await fetch("/api/config", {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(config),
  });
  if (!response.ok) {
    throw new Error("Failed to update configuration");
  }
}
