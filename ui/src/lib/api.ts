import { Target, Listener } from "./types";

const API_URL = "http://localhost:19000";

/**
 * Updates a single target on the MCP proxy server
 */
export async function updateTarget(target: Target): Promise<void> {
  try {
    // Check if it's an mcp or a2a target
    if (target.sse || target.stdio || target.openapi) {
      const response = await fetch(`${API_URL}/targets/mcp`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(target),
      });
      if (!response.ok) {
        throw new Error(`Failed to update target: ${response.status} ${response.statusText}`);
      }
    } else if (target.a2a) {
      // Convert the A2a target to the correct format
      const a2aTarget = {
        name: target.name,
        listeners: target.listeners,
        ...target.a2a,
      };

      const response = await fetch(`${API_URL}/targets/a2a`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(a2aTarget),
      });
      if (!response.ok) {
        throw new Error(`Failed to update target: ${response.status} ${response.statusText}`);
      }
    } else {
      throw new Error("Invalid target type");
    }
  } catch (error) {
    console.error("Error updating target:", error);
    throw error;
  }
}

/**
 * Fetches the listener configuration from the MCP proxy server
 */
export async function fetchListeners(): Promise<Listener[]> {
  try {
    const response = await fetch(`${API_URL}/listeners`);

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
export async function fetchMcpTargets(): Promise<any[]> {
  try {
    const response = await fetch(`${API_URL}/targets/mcp`, {});

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
export async function createMcpTarget(target: Target): Promise<void> {
  try {
    // remove the type from the target
    const targetWithoutType = { ...target, type: undefined };
    const response = await fetch(`${API_URL}/targets/mcp`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(targetWithoutType),
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
export async function getMcpTarget(name: string): Promise<any> {
  try {
    const response = await fetch(`${API_URL}/targets/mcp/${name}`, {});

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
export async function deleteMcpTarget(name: string): Promise<void> {
  try {
    const response = await fetch(`${API_URL}/targets/mcp/${name}`, {
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
export async function fetchA2aTargets(): Promise<any[]> {
  try {
    const response = await fetch(`${API_URL}/targets/a2a`, {});

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
export async function createA2aTarget(target: Target): Promise<void> {
  try {
    // Convert the A2a target to the correct format
    const a2aTarget = {
      name: target.name,
      listeners: target.listeners,
      ...target.a2a,
    };

    const response = await fetch(`${API_URL}/targets/a2a`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(a2aTarget),
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
export async function getA2aTarget(name: string): Promise<any> {
  try {
    const response = await fetch(`${API_URL}/targets/a2a/${name}`, {});

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
export async function deleteA2aTarget(name: string): Promise<void> {
  try {
    const response = await fetch(`${API_URL}/targets/a2a/${name}`, {
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
export async function fetchListenerTargets(listenerName: string): Promise<any[]> {
  try {
    const response = await fetch(`${API_URL}/listeners/${listenerName}/targets`, {});

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
export async function getListener(name: string): Promise<Listener> {
  try {
    const response = await fetch(`${API_URL}/listeners/${name}`, {});

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
export async function createListener(listener: Listener): Promise<void> {
  try {
    const response = await fetch(`${API_URL}/listeners`, {
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
export async function addListener(listener: Listener): Promise<void> {
  try {
    const response = await fetch(`${API_URL}/listeners`, {
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
export async function deleteListener(listener: Listener): Promise<void> {
  try {
    // Extract the listener name or use a default if not available
    const listenerName = listener.name || "default";

    const response = await fetch(`${API_URL}/listeners/${listenerName}`, {
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

/**
 * Deletes all targets and listeners from the agentproxy server
 */
export async function deleteEverything(): Promise<void> {
  const mcpTargets = await fetchMcpTargets();
  for (const target of mcpTargets) {
    await deleteMcpTarget(target.name);
  }

  const a2aTargets = await fetchA2aTargets();
  for (const target of a2aTargets) {
    await deleteA2aTarget(target.name);
  }
  // Fetch all listeners
  const listeners = await fetchListeners();

  // Delete each listener
  for (const listener of listeners) {
    await deleteListener(listener);
  }
}
