import { Target, RBACConfig, Listener } from "./types";

/**
 * Fetches the targets configuration from the MCP proxy server
 */
export async function fetchTargets(address: string, port: number): Promise<Target[]> {
  try {
    const response = await fetch(`http://${address}:${port}/targets`);

    if (!response.ok) {
      throw new Error(`Failed to fetch targets: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching targets:", error);
    throw error;
  }
}

/**
 * Updates a single target on the MCP proxy server
 */
export async function updateTarget(address: string, port: number, target: Target): Promise<void> {
  console.log("Updating target:", target);
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
 * Fetches the RBAC policies from the MCP proxy server
 */
export async function fetchPolicies(address: string, port: number): Promise<RBACConfig[]> {
  try {
    const response = await fetch(`http://${address}:${port}/rbac`);

    if (!response.ok) {
      throw new Error(`Failed to fetch policies: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return data.policies || [];
  } catch (error) {
    console.error("Error fetching policies:", error);
    throw error;
  }
}

/**
 * Updates the RBAC policies on the MCP proxy server
 */
export async function updatePolicies(
  address: string,
  port: number,
  policies: RBACConfig[]
): Promise<void> {
  try {
    const response = await fetch(`http://${address}:${port}/rbac`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(policies),
    });

    if (!response.ok) {
      throw new Error(`Failed to update policies: ${response.status} ${response.statusText}`);
    }
  } catch (error) {
    console.error("Error updating policies:", error);
    throw error;
  }
}

/**
 * Fetches the listener configuration from the MCP proxy server
 */
export async function fetchListeners(address: string, port: number): Promise<Listener> {
  try {
    const response = await fetch(`http://${address}:${port}/listeners`);

    if (!response.ok) {
      throw new Error(`Failed to fetch listeners: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    console.log("Raw listeners data from API:", data);
    
    // Ensure the data has the correct structure
    if (data && typeof data === 'object') {
      // If the response already has the SSE property, return it directly
      if (data.sse) {
        console.log("API returned data with SSE property:", data.sse);
        
        // Check if the SSE data has 'host' instead of 'address'
        if (data.sse.host !== undefined && data.sse.address === undefined) {
          console.log("Converting 'host' to 'address' in SSE data");
          return {
            sse: {
              address: data.sse.host,
              port: data.sse.port,
              tls: data.sse.tls
            }
          };
        }
        
        return data as Listener;
      } else {
        // Otherwise, create a proper structure
        console.log("API returned data without SSE property, creating structure");
        return {
          sse: {
            address: data.address || data.host || "0.0.0.0",
            port: data.port || 5555
          }
        };
      }
    } else {
      // Fallback if the response is not in the expected format
      console.log("API returned unexpected data format, using fallback");
      return {
        sse: {
          address: "0.0.0.0",
          port: 5555
        }
      };
    }
  } catch (error) {
    console.error("Error fetching listeners:", error);
    throw error;
  }
}
