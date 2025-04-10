"use client"

import { useState, useEffect } from "react"
import { ConfigEditor } from "@/components/config-editor"
import { ConnectionForm } from "@/components/connection-form"
import { AppSidebar } from "@/components/app-sidebar"
import { ListenerConfig } from "@/components/listener-config"
import { TargetsConfig } from "@/components/targets-config"
import { PoliciesConfig } from "@/components/policies-config"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { AlertCircle, CheckCircle2, Info } from "lucide-react"
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
  const [connectionError, setConnectionError] = useState("")
  const [serverAddress, setServerAddress] = useState<string>()
  const [serverPort, setServerPort] = useState<number>()
  const [isSaving, setIsSaving] = useState(false)
  const [saveSuccess, setSaveSuccess] = useState(false)
  const [activeView, setActiveView] = useState("home")
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false)
  const [isPushingConfig, setIsPushingConfig] = useState(false)
  const [showConfigDiff, setShowConfigDiff] = useState(false)
  const [currentProxyConfig, setCurrentProxyConfig] = useState<Config | null>(null)

  // Load saved connection from localStorage
  useEffect(() => {
    const savedConnection = localStorage.getItem("mcpProxyConnection")
    if (savedConnection) {
      const { address, port } = JSON.parse(savedConnection)
      setServerAddress(address)
      setServerPort(port)
      connectToServer(address, port)
    } else {
      setIsLoading(false)
    }
  }, [])

  // Save configuration to localStorage whenever it changes
  useEffect(() => {
    if (config) {
      localStorage.setItem("mcpProxyConfig", JSON.stringify(config))
    }
  }, [config])

  const connectToServer = async (address: string, port: number): Promise<boolean> => {
    if (!address || !port) return false
    setIsLoading(true)
    try {
      // Simulate successful connection
      setIsConnected(true)
      setServerAddress(address)
      setServerPort(port)
      localStorage.setItem("mcpProxyConnection", JSON.stringify({ address, port }))

      // Simulate initial configuration
      const initialConfig: Config = {
        type: "static",
        listeners: [
          {
            sse: {
              address: "0.0.0.0",
              port: 3000
            }
          }
        ],
        targets: [],
        policies: []
      }
      setConfig(initialConfig)
      setCurrentProxyConfig(initialConfig)
      setHasUnsavedChanges(false)
      return true
    } catch (error) {
      console.error("Connection error:", error)
      setIsConnected(false)
      setServerAddress(undefined)
      setServerPort(undefined)
      localStorage.removeItem("mcpProxyConnection")
      return false
    } finally {
      setIsLoading(false)
    }
  }

  const pullConfiguration = async () => {
    if (!serverAddress || !serverPort) return

    try {
      const response = await fetch(`http://${serverAddress}:${serverPort}/api/config`)
      if (!response.ok) {
        throw new Error("Failed to fetch configuration")
      }

      const proxyConfig = await response.json()
      setCurrentProxyConfig(proxyConfig)
      setConfig(proxyConfig)
      setHasUnsavedChanges(false)
    } catch (error) {
      console.error("Failed to pull configuration:", error)
    }
  }

  const pushConfiguration = async () => {
    if (!serverAddress || !serverPort) return

    setIsPushingConfig(true)
    try {
      // Simulate successful push
      setCurrentProxyConfig(config)
      setHasUnsavedChanges(false)
      setShowConfigDiff(false)
      return true
    } catch (error) {
      console.error("Failed to push configuration:", error)
      return false
    } finally {
      setIsPushingConfig(false)
    }
  }

  const disconnect = async () => {
    if (!serverAddress || !serverPort) return

    try {
      const response = await fetch(`http://${serverAddress}:${serverPort}/api/disconnect`, {
        method: "POST",
      })

      if (!response.ok) {
        throw new Error("Failed to disconnect from server")
      }

      setIsConnected(false)
      setServerAddress(undefined)
      setServerPort(undefined)
      localStorage.removeItem("mcpProxyConnection")
    } catch (error) {
      console.error("Disconnection error:", error)
    }
  }

  const handleConfigChange = (newConfig: Config) => {
    setConfig(newConfig)
    setHasUnsavedChanges(true)
  }

  const saveConfiguration = async () => {
    if (!isConnected) return

    setIsSaving(true)
    setSaveSuccess(false)

    try {
      if (serverPort !== undefined) {
        await updateConfig(serverAddress, serverPort, config)
      } else {
        setConnectionError("Server port is undefined. Cannot save configuration.")
      }
      setSaveSuccess(true)
      setTimeout(() => setSaveSuccess(false), 3000)
    } catch (error) {
      setConnectionError(error instanceof Error ? error.message : "Failed to save configuration")
    } finally {
      setIsSaving(false)
    }
  }

  const addTarget = (target: Target) => {
    setConfig({
      ...config,
      targets: [...config.targets, target],
    })
    setHasUnsavedChanges(true)
  }

  const removeTarget = (index: number) => {
    setConfig({
      ...config,
      targets: config.targets.filter((_, i) => i !== index),
    })
    setHasUnsavedChanges(true)
  }

  const updateListener = (listener: Listener) => {
    setConfig({
      ...config,
      listeners: [listener],
    })
    setHasUnsavedChanges(true)
  }

  const addPolicy = (policy: RBACConfig) => {
    setConfig({
      ...config,
      policies: [...(config.policies || []), policy],
    })
    setHasUnsavedChanges(true)
  }

  const removePolicy = (index: number) => {
    setConfig({
      ...config,
      policies: config.policies?.filter((_, i) => i !== index) || [],
    })
    setHasUnsavedChanges(true)
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
            listener={config.listeners[0] || { sse: { address: "0.0.0.0", port: 3000 } }}
            updateListener={updateListener}
          />
        )
      case "targets":
        return <TargetsConfig targets={config.targets} addTarget={addTarget} removeTarget={removeTarget} />
      case "policies":
        return <PoliciesConfig policies={config.policies || []} addPolicy={addPolicy} removePolicy={removePolicy} />
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
                  {config.policies?.length || 0} polic{config.policies?.length !== 1 ? "ies" : "y"} configured
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
              onDisconnect={disconnect}
              targets={config.targets}
              activeView={activeView}
              setActiveView={setActiveView}
              addTarget={addTarget}
              hasUnsavedChanges={hasUnsavedChanges}
              onPushConfig={() => setShowConfigDiff(true)}
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

              {renderContent()}
            </main>
          </>
        ) : (
          <NotConnectedState onConnect={connectToServer} connectionError={connectionError} />
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