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

// Connection storage keys
const CONNECTION_STORAGE_KEY = "mcp-proxy-connection"

export default function Home() {
  const { isLoading, setIsLoading } = useLoading()
  const [config, setConfig] = useState<Config>({
    type: "static",
    listener: {
      sse: {
        address: "0.0.0.0",
        port: 3000,
      },
    },
    targets: [],
    policies: [],
  })

  const [isConnected, setIsConnected] = useState(false)
  const [connectionError, setConnectionError] = useState("")
  const [serverAddress, setServerAddress] = useState("")
  const [serverPort, setServerPort] = useState<number>()
  const [isSaving, setIsSaving] = useState(false)
  const [saveSuccess, setSaveSuccess] = useState(false)
  const [activeView, setActiveView] = useState("home")

  // Load saved connection on initial render
  useEffect(() => {
    const loadSavedConnection = async () => {
      try {
        const savedConnection = localStorage.getItem(CONNECTION_STORAGE_KEY)
        if (savedConnection) {
          const { address, port } = JSON.parse(savedConnection)
          if (address && port) {
            await connectToServer(address, port)
          } else {
            setIsLoading(false)
          }
        } else {
          setIsLoading(false)
        }
      } catch (error) {
        console.error("Failed to restore connection:", error)
        setIsLoading(false)
      }
    }

    loadSavedConnection()
  }, [setIsLoading])

  const connectToServer = async (address: string, port: number) => {
    setConnectionError("")
    try {
      const serverConfig = await fetchConfig(address, port)
      setConfig({
        ...serverConfig,
        policies: serverConfig.policies || [],
      })
      setServerAddress(address)
      setServerPort(port)
      setIsConnected(true)
      
      // Save connection to localStorage
      localStorage.setItem(CONNECTION_STORAGE_KEY, JSON.stringify({ address, port }))
      
      return true
    } catch (error) {
      setConnectionError(error instanceof Error ? error.message : "Failed to connect to server")
      setIsConnected(false)
      return false
    } finally {
      // Always set loading to false after a connection attempt
      setIsLoading(false)
    }
  }

  const disconnectFromServer = () => {
    setIsConnected(false)
    setServerAddress("")
    setServerPort(undefined)
    localStorage.removeItem(CONNECTION_STORAGE_KEY)
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
  }

  const removeTarget = (index: number) => {
    const newTargets = [...config.targets]
    newTargets.splice(index, 1)
    setConfig({
      ...config,
      targets: newTargets,
    })
  }

  const updateListener = (listener: Listener) => {
    setConfig({
      ...config,
      listener,
    })
  }

  const addPolicy = (policy: RBACConfig) => {
    setConfig({
      ...config,
      policies: [...(config.policies || []), policy],
    })
  }

  const removePolicy = (index: number) => {
    const newPolicies = [...(config.policies || [])]
    newPolicies.splice(index, 1)
    setConfig({
      ...config,
      policies: newPolicies,
    })
  }

  const renderContent = () => {
    if (!isConnected) {
      return (
        <Card className="mb-6">
          <CardHeader>
            <CardTitle>Connect to MCP Proxy</CardTitle>
            <CardDescription>Enter the address and port of your MCP proxy server</CardDescription>
          </CardHeader>
          <CardContent>
            <ConnectionForm onConnect={connectToServer} />
            {connectionError && (
              <Alert variant="destructive" className="mt-4">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{connectionError}</AlertDescription>
              </Alert>
            )}
          </CardContent>
        </Card>
      )
    }

    switch (activeView) {
      case "listener":
        return <ListenerConfig listener={config.listener} updateListener={updateListener} />
      case "targets":
        return <TargetsConfig targets={config.targets} addTarget={addTarget} removeTarget={removeTarget} />
      case "policies":
        return <PoliciesConfig policies={config.policies || []} addPolicy={addPolicy} removePolicy={removePolicy} />
      case "json":
        return <ConfigEditor config={config} setConfig={setConfig} />
      default:
        return (
          <div className="space-y-6">
            <div className="flex justify-between items-center">
              <h2 className="text-xl font-semibold">Configuration Overview</h2>
              <Button onClick={saveConfiguration} disabled={isSaving} size="sm">
                {isSaving ? "Saving..." : saveSuccess ? "Saved!" : "Save Configuration"}
              </Button>
            </div>
            
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-2">
                <Card>
                  <CardHeader>
                    <CardTitle>Current Configuration</CardTitle>
                    <CardDescription>Summary of your MCP proxy configuration</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-4">
                      <div>
                        <h3 className="text-sm font-medium">Listener</h3>
                        <p className="text-sm text-muted-foreground">
                          {config.listener.sse
                            ? `SSE on ${config.listener.sse.address}:${config.listener.sse.port}`
                            : config.listener.stdio
                              ? "Stdio"
                              : "Not configured"}
                        </p>
                      </div>
                      <div>
                        <h3 className="text-sm font-medium">Target Servers</h3>
                        <p className="text-sm text-muted-foreground">
                          {config.targets.length} server{config.targets.length !== 1 ? "s" : ""} configured
                        </p>
                      </div>
                      <div>
                        <h3 className="text-sm font-medium">Policies</h3>
                        <p className="text-sm text-muted-foreground">
                          {config.policies?.length || 0} polic{config.policies?.length !== 1 ? "ies" : "y"} configured
                        </p>
                      </div>
                      <div className="pt-2">
                        <Button variant="outline" onClick={() => setActiveView("json")}>
                          View JSON Configuration
                        </Button>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              </div>
              <div>
                <Card>
                  <CardHeader>
                    <CardTitle>Quick Actions</CardTitle>
                    <CardDescription>Common configuration tasks</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-2">
                    <Button variant="outline" className="w-full justify-start" onClick={() => setActiveView("listener")}>
                      Configure Listener
                    </Button>
                    <Button variant="outline" className="w-full justify-start" onClick={() => setActiveView("targets")}>
                      Manage Target Servers
                    </Button>
                    <Button variant="outline" className="w-full justify-start" onClick={() => setActiveView("policies")}>
                      Set Up Policies
                    </Button>
                  </CardContent>
                </Card>
              </div>
            </div>
          </div>
        )
    }
  }

  // Show loading state while checking for saved connection
  if (isLoading) {
    return <LoadingState />
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
              addTarget={addTarget}
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
    </SidebarProvider>
  )
}