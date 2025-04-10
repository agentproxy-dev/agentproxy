"use client"

import { useState } from "react"
import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarMenuBadge,
  SidebarSeparator,
} from "@/components/ui/sidebar"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { PlusCircle } from "lucide-react"
import { Server, Settings, Shield, Headphones, Globe, Terminal, FileJson, HelpCircle, Plug, Home, RefreshCw, LogOut } from "lucide-react"
import { MCPLogo } from "@/components/mcp-logo"
import { ThemeToggle } from "@/components/theme-toggle"
import { Target } from "@/lib/types"

interface AppSidebarProps {
  isConnected: boolean
  serverAddress?: string
  serverPort?: number
  onConnect: (address: string, port: number) => Promise<boolean>
  onDisconnect: () => void
  targets: any[]
  activeView: string
  setActiveView: (view: string) => void
  addTarget: (target: Target) => void
}

export function AppSidebar({
  isConnected,
  serverAddress,
  serverPort,
  onConnect,
  onDisconnect,
  targets,
  activeView,
  setActiveView,
  addTarget,
}: AppSidebarProps) {
  const [quickConnectAddress, setQuickConnectAddress] = useState("localhost")
  const [quickConnectPort, setQuickConnectPort] = useState(3000)
  const [isConnecting, setIsConnecting] = useState(false)

  const handleQuickConnect = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    setIsConnecting(true)

    try {
      await onConnect(quickConnectAddress, quickConnectPort)
    } finally {
      setIsConnecting(false)
    }
  }

  const handleAddNewTarget = () => {
    setActiveView("targets")
    // Focus on the add target form
    setTimeout(() => {
      const addTargetForm = document.getElementById("add-target-form")
      if (addTargetForm) {
        addTargetForm.scrollIntoView({ behavior: "smooth" })
      }
    }, 100)
  }

  return (
    <Sidebar>
      <SidebarHeader className="border-b">
        <div className="p-2">
          <div className="flex items-center justify-center mb-2">
            <MCPLogo className="h-10 w-auto" />
          </div>

          {!isConnected ? (
            <form onSubmit={handleQuickConnect} className="space-y-2 mt-4">
              <div className="space-y-1">
                <Label htmlFor="quick-address" className="text-xs">
                  Server
                </Label>
                <Input
                  id="quick-address"
                  value={quickConnectAddress}
                  onChange={(e) => setQuickConnectAddress(e.target.value)}
                  placeholder="localhost"
                  className="h-7"
                  required
                />
              </div>
              <div className="space-y-1">
                <Label htmlFor="quick-port" className="text-xs">
                  Port
                </Label>
                <Input
                  id="quick-port"
                  value={quickConnectPort}
                  onChange={(e) => setQuickConnectPort(parseInt(e.target.value))}
                  placeholder="3000"
                  className="h-7"
                  required
                />
              </div>
              <Button type="submit" size="sm" className="w-full" disabled={isConnecting}>
                {isConnecting ? "Connecting..." : "Connect"}
              </Button>
            </form>
          ) : (
            <div className="mt-4 space-y-2">
              <div className="text-sm px-2 py-1 bg-secondary text-secondary-foreground rounded-md">
                <div className="font-medium flex items-center">
                  <Plug className="h-3 w-3 mr-1" /> Connected
                </div>
                <div className="text-xs mt-1">
                  {serverAddress}:{serverPort}
                </div>
              </div>
              <div className="flex space-x-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="flex-1 text-xs"
                  onClick={() => {
                    if (serverAddress && serverPort) {
                      onConnect(serverAddress, serverPort)
                    } else {
                      console.error("Server address or port is undefined.")
                    }
                  }}
                >
                  <RefreshCw className="h-3 w-3 mr-1" />
                  Refresh
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="flex-1 text-xs text-destructive hover:text-destructive"
                  onClick={onDisconnect}
                >
                  <LogOut className="h-3 w-3 mr-1" />
                  Disconnect
                </Button>
              </div>
            </div>
          )}
        </div>
      </SidebarHeader>

      <SidebarContent>
        {isConnected && (
          <>
            <SidebarGroup>
              <SidebarGroupLabel>Navigation</SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      tooltip="Home"
                      isActive={activeView === "home"}
                      onClick={() => setActiveView("home")}
                    >
                      <Home className="h-4 w-4" />
                      <span>Home</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      tooltip="Listener Settings"
                      isActive={activeView === "listener"}
                      onClick={() => setActiveView("listener")}
                    >
                      <Headphones className="h-4 w-4" />
                      <span>Listener</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      tooltip="Target Servers"
                      isActive={activeView === "targets"}
                      onClick={() => setActiveView("targets")}
                    >
                      <Server className="h-4 w-4" />
                      <span>Targets</span>
                      <SidebarMenuBadge>{targets.length}</SidebarMenuBadge>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      tooltip="Security Policies"
                      isActive={activeView === "policies"}
                      onClick={() => setActiveView("policies")}
                    >
                      <Shield className="h-4 w-4" />
                      <span>Policies</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      tooltip="JSON Configuration"
                      isActive={activeView === "json"}
                      onClick={() => setActiveView("json")}
                    >
                      <FileJson className="h-4 w-4" />
                      <span>JSON View</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>

            <SidebarSeparator />

            <SidebarGroup>
              <SidebarGroupLabel className="flex justify-between items-center">
                <span>Target Servers</span>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-5 w-5 rounded-full"
                  onClick={handleAddNewTarget}
                  title="Add New Target"
                >
                  <PlusCircle className="h-4 w-4" />
                </Button>
              </SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  {targets.length > 0 ? (
                    targets.map((target, index) => (
                      <SidebarMenuItem key={index}>
                        <SidebarMenuButton
                          tooltip={`View ${target.name}`}
                          onClick={() => {
                            setActiveView("targets")
                            // Scroll to the target in the list
                            setTimeout(() => {
                              const targetElement = document.getElementById(`target-${index}`)
                              if (targetElement) {
                                targetElement.scrollIntoView({ behavior: "smooth" })
                              }
                            }, 100)
                          }}
                        >
                          {getTargetIcon(target)}
                          <span>{target.name}</span>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    ))
                  ) : (
                    <div className="text-xs text-muted-foreground px-2 py-1 flex items-center justify-between">
                      <span>No target servers</span>
                      <Button variant="ghost" size="sm" className="h-6 text-xs" onClick={handleAddNewTarget}>
                        Add Server
                      </Button>
                    </div>
                  )}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          </>
        )}
      </SidebarContent>

      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton tooltip="Theme">
              <ThemeToggle asChild className="flex items-center gap-2" />
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  )
}

function getTargetIcon(target: Target) {
  if (target.stdio) {
    return <Terminal className="h-4 w-4" />
  }
  if (target.sse) {
    return <Globe className="h-4 w-4" />
  }
  if (target.openapi) {
    return <Globe className="h-4 w-4" />
  }
  return <Server className="h-4 w-4" />
}
