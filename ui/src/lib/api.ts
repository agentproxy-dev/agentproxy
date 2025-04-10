const mockConfigurations = {
  "localhost:3000": {
    type: "static",
    listener: {
      sse: {
        host: "0.0.0.0",
        port: 3000,
      },
    },
    targets: [
      {
        name: "everything",
        stdio: {
          cmd: "npx",
          args: ["@modelcontextprotocol/server-everything"],
        },
      },
    ],
  },
  "localhost:3001": {
    type: "static",
    listener: {
      sse: {
        host: "0.0.0.0",
        port: 3001,
      },
    },
    targets: [
      {
        name: "petstore",
        openapi: {
          host: "petstore3.swagger.io",
          port: 443,
          schema: {
            file_path: "examples/openapi/openapi.json",
          },
        },
      },
    ],
  },
  "localhost:3002": {
    type: "static",
    listener: {
      sse: {
        host: "0.0.0.0",
        port: 3002,
        authn: {
          jwt: {
            issuer: ["me"],
            audience: ["me.com"],
            jwks: {
              local: {
                file: "manifests/jwt/pub-key",
              },
            },
          },
        },
      },
    },
    policies: [
      {
        key: "sub",
        value: "me",
        resource: {
          tool: {
            id: "everything:echo",
          },
        },
        matcher: {
          equals: {},
        },
      },
    ],
    targets: [
      {
        name: "everything",
        stdio: {
          cmd: "npx",
          args: ["@modelcontextprotocol/server-everything"],
        },
      },
      {
        name: "everything_else",
        stdio: {
          cmd: "npx",
          args: ["@modelcontextprotocol/server-everything"],
        },
      },
    ],
  },
}

// Store updated configurations
const updatedConfigs = { ...mockConfigurations }

// Mock available servers
const mockAvailableServers = [
  { name: "Local Development", address: "localhost", port: "3000" },
  { name: "Petstore API", address: "localhost", port: "3001" },
  { name: "Auth Server", address: "localhost", port: "3002" },
  { name: "Production Server", address: "mcp-proxy.example.com", port: "443" },
  { name: "Staging Server", address: "staging.mcp-proxy.example.com", port: "443" },
]

/**
 * Simulates fetching the configuration from the MCP proxy server
 */
export async function fetchConfig(address: string, port: number): Promise<any> {
  // Simulate network delay
  await new Promise((resolve) => setTimeout(resolve, 800))

  const key = `${address}:${port}`

  // Check if we have this configuration
  if (updatedConfigs[key]) {
    return { ...updatedConfigs[key] }
  }

  // Check if we have a mock configuration for this address:port
  if (mockConfigurations[key]) {
    return { ...mockConfigurations[key] }
  }

  // Simulate connection error for unknown servers
  if (Math.random() > 0.8) {
    throw new Error("Connection timed out")
  }

  throw new Error("Server not found or configuration unavailable")
}

/**
 * Simulates updating the configuration on the MCP proxy server
 */
export async function updateConfig(address: string, port: number, config: any): Promise<void> {
  // Simulate network delay
  await new Promise((resolve) => setTimeout(resolve, 1000))

  const key = `${address}:${port}`

  // Randomly fail sometimes to simulate network issues
  if (Math.random() > 0.9) {
    throw new Error("Failed to update configuration: Network error")
  }

  // Store the updated configuration
  updatedConfigs[key] = { ...config }
}

/**
 * Simulates fetching all available servers
 */
export async function getAllServers(): Promise<any[]> {
  // Simulate network delay
  await new Promise((resolve) => setTimeout(resolve, 500))

  return [...mockAvailableServers]
}
