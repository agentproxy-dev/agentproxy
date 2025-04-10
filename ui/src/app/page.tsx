"use client"

import { useState, useEffect } from "react"
import { ConfigEditor } from "@/components/config-editor"
import { ConnectionForm } from "@/components/connection-form"
import { AppSidebar } from "@/components/app-sidebar"
import { ListenerConfig } from "@/components/listener-config"
import { TargetsConfig } from "@/components/targets-config"
import { PoliciesConfig } from "@/components/policies-config"
import { fetchConfig, updateConfig } from "@/lib/api"
import { Button } from "@/components/ui/button"
import { SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar"
import {
  Config,
  Listener,
  Target,
  RBACConfig
} from "@/lib/types"
import { MCPLogo } from "@/components/mcp-logo"
import { NotConnectedState } from "@/components/not-connected-state"
import { SocialLinks } from "@/components/social-links"
import { useLoading } from "@/lib/loading-context"
import { LoadingState } from "@/components/loading-state"
import { ConfigDiffDialog } from "@/components/config-diff-dialog"
import { JsonConfig } from "@/components/json-config"
import { Loader2 } from "lucide-react"
import { ConnectionError } from "@/components/connection-error"

// Connection storage keys
const CONNECTION_STORAGE_KEY = "mcp-proxy-connection"

export default function Home() {
  const { isLoading, setIsLoading } = useLoading()
  const [config, setConfig] = useState<Config>({
    type: "static",
    listeners: [],
    targets: [],
    policies: [],
  })

  const [isConnected, setIsConnected] = useState(false)
  const [connectionError, setConnectionError] = useState<string | null>(null)
  const [serverAddress, setServerAddress] = useState<string>("")
  const [serverPort, setServerPort] = useState<number>(19000)
  const [isSaving, setIsSaving] = useState(false)
  const [saveSuccess, setSaveSuccess] = useState(false)
  const [activeView, setActiveView] = useState<string>("home")
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false)
  const [isPushingConfig, setIsPushingConfig] = useState(false)
  const [showConfigDiff, setShowConfigDiff] = useState(false)
  const [currentProxyConfig, setCurrentProxyConfig] = useState<Config | null>(null)
  const [error, setError] = useState<string | null>(null)

  // Load saved connection from localStorage
  useEffect(() => {
    const savedAddress = localStorage.getItem("serverAddress")
    const savedPort = localStorage.getItem("serverPort")

    if (savedAddress && savedPort) {
      setServerAddress(savedAddress)
      setServerPort(parseInt(savedPort))
      connectToServer(savedAddress, parseInt(savedPort))
    } else {
      setIsLoading(false)
    }
  }, [])

  // Save connection details to localStorage when they change
  useEffect(() => {
    if (serverAddress && serverPort) {
      localStorage.setItem("serverAddress", serverAddress)
      localStorage.setItem("serverPort", serverPort.toString())
    }
  }, [serverAddress, serverPort])

  // Save local configuration to localStorage when it changes
  useEffect(() => {
    if (isConnected) {
      localStorage.setItem("localConfig", JSON.stringify(config))
    }
  }, [config, isConnected])

  const connectToServer = async (address: string, port: number) => {
    if (!address || !port) {
      setConnectionError("Please enter a valid server address and port")
      setIsLoading(false)
      return false
    }

    setIsLoading(true)
    setConnectionError(null)

    try {
      // Fetch configuration from the proxy
      const baseUrl = `http://${address}:${port}`;

      // Fetch listeners configuration
      let listenersResponse;
      try {
        console.log('Fetching listeners configuration from', `${baseUrl}/listeners`)
        listenersResponse = await fetch(`${baseUrl}/listeners`)
      } catch (error) {
        throw new Error(`Unable to connect to server at ${address}:${port}. Please check if the server is running and accessible.`)
      }

      if (!listenersResponse.ok) {
        throw new Error(`Server returned an error: ${listenersResponse.status} ${listenersResponse.statusText}`)
      }

      const listenersData = await listenersResponse.json()
      console.log('Received listeners data:', listenersData)

      // Fetch targets configuration
      const targetsResponse = await fetch(`${baseUrl}/targets`)
      if (!targetsResponse.ok) {
        throw new Error(`Failed to fetch targets: ${targetsResponse.status} ${targetsResponse.statusText}`)
      }
      const targetsData = await targetsResponse.json()
      console.log('Received targets data:', targetsData)

      // Convert targets object to array
      const targetsArray = Object.entries(targetsData).map(([name, targetData]) => {
        return {
          name,
          ...(targetData as any)
        }
      })
      console.log('Converted targets array:', targetsArray)

      // Fetch RBAC policies
      const rbacResponse = await fetch(`${baseUrl}/rbac`)
      if (!rbacResponse.ok) {
        throw new Error(`Failed to fetch RBAC policies: ${rbacResponse.status} ${rbacResponse.statusText}`)
      }
      const rbacData = await rbacResponse.json()

      // Update state with fetched configuration
      const newConfig = {
        type: "static" as const,
        listeners: [listenersData], // Use the entire listeners response as the first listener
        targets: targetsArray,
        policies: rbacData.policies || [],
      }

      setConfig(newConfig)
      setCurrentProxyConfig(newConfig)
      setServerAddress(address)
      setServerPort(port)
      setIsConnected(true)
      setHasUnsavedChanges(false)
      return true
     
    } catch (error) {
      console.error("Error connecting to server:", error)
      setConnectionError(error instanceof Error ? error.message : "Failed to connect to server")
      setIsConnected(false)
      return false
    } finally {
      setIsLoading(false)
    }
  }

  const disconnectFromServer = () => {
    setIsConnected(false)
    setServerAddress("")
    setServerPort(19000)
    setConfig({
      type: "static",
      listeners: [],
      targets: [],
      policies: [],
    })
    setCurrentProxyConfig(null)
    setHasUnsavedChanges(false)
    localStorage.removeItem("serverAddress")
    localStorage.removeItem("serverPort")
  }

  const handleConfigChange = (newConfig: Config) => {
    setConfig(newConfig)
    setHasUnsavedChanges(true)
  }

  const pushConfiguration = async () => {
    if (!isConnected || !serverAddress || !serverPort) {
      setError("Not connected to a server")
      return
    }

    setIsPushingConfig(true)
    setError(null)

    try {
      const baseUrl = `http://${serverAddress}:${serverPort}`

      // Update listeners
      const listenersResponse = await fetch(`${baseUrl}/listeners`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ listeners: config.listeners }),
      })

      if (!listenersResponse.ok) {
        throw new Error(`Failed to update listeners: ${listenersResponse.statusText}`)
      }

      // Update targets - first delete all existing targets
      const existingTargetsResponse = await fetch(`${baseUrl}/targets`)
      if (existingTargetsResponse.ok) {
        const existingTargets = await existingTargetsResponse.json()
        for (const target of existingTargets.targets || []) {
          await fetch(`${baseUrl}/targets/${target.name}`, {
            method: "DELETE",
          })
        }
      }

      // Then add all targets from the new configuration
      for (const target of config.targets) {
        const targetResponse = await fetch(`${baseUrl}/targets`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(target),
        })

        if (!targetResponse.ok) {
          throw new Error(`Failed to update target ${target.name}: ${targetResponse.statusText}`)
        }
      }

      // Update RBAC policies - first delete all existing policies
      const existingPoliciesResponse = await fetch(`${baseUrl}/rbac`)
      if (existingPoliciesResponse.ok) {
        const existingPolicies = await existingPoliciesResponse.json()
        for (const policy of existingPolicies.policies || []) {
          await fetch(`${baseUrl}/rbac/${policy.name}`, {
            method: "DELETE",
          })
        }
      }

      // Then add all policies from the new configuration
      for (const policy of config.policies || []) {
        const policyResponse = await fetch(`${baseUrl}/rbac`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(policy),
        })

        if (!policyResponse.ok) {
          throw new Error(`Failed to update policy ${policy.name}: ${policyResponse.statusText}`)
        }
      }

      // Update the server config with the local config
      setConfig(config)
      setHasUnsavedChanges(false)

      // Refresh the configuration from the server
      await connectToServer(serverAddress, serverPort)
    } catch (error) {
      console.error("Error pushing configuration:", error)
      setError(error instanceof Error ? error.message : "Failed to push configuration")
    } finally {
      setIsPushingConfig(false)
    }
  }

  const handleAddTarget = (target: Target) => {
    const newConfig = {
      ...config,
      targets: [...config.targets, target],
    }
    handleConfigChange(newConfig)
  }

  const handleRemoveTarget = (index: number) => {
    const newConfig = {
      ...config,
      targets: config.targets.filter((_, i) => i !== index),
    }
    handleConfigChange(newConfig)
  }

  const handleAddPolicy = (policy: RBACConfig) => {
    const newConfig = {
      ...config,
      policies: [...(config.policies || []), policy],
    }
    handleConfigChange(newConfig)
  }

  const handleRetry = () => {
    if (serverAddress && serverPort) {
      connectToServer(serverAddress, serverPort)
    }
  }

  const renderContent = () => {
    if (isLoading) {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
            <p className="mt-2 text-sm text-muted-foreground">Loading...</p>
          </div>
        </div>
      )
    }

    if (!isConnected) {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <h2 className="text-lg font-medium">Welcome to MCP Proxy</h2>
            <p className="mt-2 text-sm text-muted-foreground">
              Connect to a proxy server to get started
            </p>
          </div>
        </div>
      )
    }

    switch (activeView) {
      case "listener":
        return (
          <ListenerConfig
            config={config}
            onChange={handleConfigChange}
            serverAddress={serverAddress}
            serverPort={serverPort}
          />
        )
      case "targets":
        return <TargetsConfig 
          targets={config.targets} 
          addTarget={handleAddTarget}
          removeTarget={handleRemoveTarget}
        />
      case "policies":
        return <PoliciesConfig policies={config.policies || []} addPolicy={handleAddPolicy} removePolicy={handleRemovePolicy} />
      case "json":
        return <JsonConfig config={config} onConfigChange={handleConfigChange} />
      default:
        return (
          <div className="p-6">
            <h2 className="text-lg font-medium">Overview</h2>
            <div className="mt-4 grid gap-4">
              <div className="p-4 border rounded-lg">
                <h3 className="text-sm font-medium">Listener</h3>
                <p className="text-sm text-muted-foreground">
                  {config.listeners.length > 0 && config.listeners[0].sse
                    ? `SSE on ${config.listeners[0].sse.address}:${config.listeners[0].sse.port}`
                    : "Not configured"}
                </p>
              </div>
              <div className="p-4 border rounded-lg">
                <h3 className="text-sm font-medium">Target Servers</h3>
                <p className="text-sm text-muted-foreground">
                  {config.targets.length} target{config.targets.length !== 1 ? "s" : ""} configured
                </p>
              </div>
              <div className="p-4 border rounded-lg">
                <h3 className="text-sm font-medium">Security Policies</h3>
                <p className="text-sm text-muted-foreground">
                  {config.policies?.length} policy{config.policies?.length !== 1 ? "ies" : "y"} configured
                </p>
              </div>
            </div>
          </div>
        )
    }
  }

  return (
    <SidebarProvider>
      <div className="flex min-h-screen w-full">
        {isConnected ? (
          <>
            <AppSidebar
              isConnected={isConnected}
              serverAddress={serverAddress}
              serverPort={serverPort}
              onConnect={connectToServer}
              onDisconnect={disconnectFromServer}
              targets={config.targets}
              activeView={activeView}
              setActiveView={setActiveView}
              addTarget={handleAddTarget}
              hasUnsavedChanges={hasUnsavedChanges}
              onPushConfig={pushConfiguration}
              isPushingConfig={isPushingConfig}
            />

            <main className="flex-1 p-6">
              <div className="flex items-center justify-between mb-6">
                <div className="flex items-center">
                  <SidebarTrigger className="mr-4" />
                  <h1 className="text-3xl font-bold">
                    {activeView === "home"
                      ? "MCP Proxy Configuration"
                      : activeView === "listener"
                        ? "Listener Configuration"
                        : activeView === "targets"
                          ? "Target Servers"
                          : activeView === "policies"
                            ? "Security Policies"
                            : activeView === "json"
                              ? "JSON Configuration"
                              : "MCP Proxy Configuration"}
                  </h1>
                </div>
              </div>

              {error && (
                <div className="mb-4 rounded-md bg-destructive/10 p-4 text-destructive">
                  <p>{error}</p>
                </div>
              )}

              {renderContent()}
            </main>
          </>
        ) : (
          <NotConnectedState
            onConnect={connectToServer}
            connectionError={connectionError || ""}
          />
        )}
      </div>

      <ConfigDiffDialog
        open={showConfigDiff}
        onOpenChange={setShowConfigDiff}
        currentConfig={currentProxyConfig}
        newConfig={config}
        onConfirm={pushConfiguration}
        isPushing={isPushingConfig}
      />
    </SidebarProvider>
  )
}