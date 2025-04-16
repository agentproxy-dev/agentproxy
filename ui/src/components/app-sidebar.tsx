"use client";

import { useState } from "react";
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
  useSidebar,
} from "@/components/ui/sidebar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Loader2,
  Home,
  Shield,
  Headphones,
  Server,
  Code,
  LogOut,
  ChevronsUpDown,
} from "lucide-react";
import { MCPLogo } from "@/components/mcp-logo";
import { ThemeToggle } from "@/components/theme-toggle";
import { Target } from "@/lib/types";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

interface AppSidebarProps {
  isConnected: boolean;
  serverAddress?: string;
  serverPort?: number;
  onConnect: (address: string, port: number) => Promise<boolean>;
  onDisconnect: () => void;
  targets: any[];
  activeView: string;
  setActiveView: (view: string) => void;
  addTarget: (target: Target) => void;
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
}: AppSidebarProps) {
  const [quickConnectAddress, setQuickConnectAddress] = useState("localhost");
  const [quickConnectPort, setQuickConnectPort] = useState(3000);
  const [isConnecting, setIsConnecting] = useState(false);
  const { isMobile } = useSidebar();

  const handleQuickConnect = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setIsConnecting(true);

    try {
      await onConnect(quickConnectAddress, quickConnectPort);
    } finally {
      setIsConnecting(false);
    }
  };

  // const handleAddNewTarget = () => {
  //   setActiveView("targets")
  //   // Focus on the add target form
  //   setTimeout(() => {
  //     const addTargetForm = document.getElementById("add-target-form")
  //     if (addTargetForm) {
  //       addTargetForm.scrollIntoView({ behavior: "smooth" })
  //     }
  //   }, 100)
  // }

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
                  aria-label="Server address"
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
                  aria-label="Server port"
                />
              </div>
              <Button type="submit" size="sm" className="w-full" disabled={isConnecting}>
                {isConnecting ? "Connecting..." : "Connect"}
              </Button>
            </form>
          ) : (
            <div className="mt-4 space-y-2">
              <SidebarMenu>
                <SidebarMenuItem>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <SidebarMenuButton
                        size="lg"
                        className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
                        aria-label="Server actions"
                      >
                        <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
                          <Server className="size-4" />
                        </div>
                        <div className="grid flex-1 text-left text-sm leading-tight">
                          <span className="truncate font-semibold">
                            {serverAddress}:{serverPort}
                          </span>
                          <span className="truncate text-xs">Connected</span>
                        </div>
                        <ChevronsUpDown className="ml-auto" />
                      </SidebarMenuButton>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent
                      className="w-[--radix-dropdown-menu-trigger-width] min-w-56 rounded-lg"
                      align="start"
                      side={isMobile ? "bottom" : "right"}
                      sideOffset={4}
                    >
                      <DropdownMenuLabel className="text-xs text-muted-foreground">
                        Server Actions
                      </DropdownMenuLabel>
                      <DropdownMenuItem
                        onClick={() => {
                          if (serverAddress && serverPort) {
                            onConnect(serverAddress, serverPort);
                          }
                        }}
                        className="gap-2 p-2"
                      >
                        <Loader2 className="size-4 animate-spin" />
                        Refresh Connection
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem
                        onClick={onDisconnect}
                        className="gap-2 p-2 text-destructive"
                      >
                        <LogOut className="size-4" />
                        Disconnect
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </SidebarMenuItem>
              </SidebarMenu>
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
                      aria-label="Home"
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
                      aria-label="Listener Settings"
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
                      aria-label="Target Servers"
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
                      aria-label="Security Policies"
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
                      aria-label="JSON Configuration"
                    >
                      <Code className="h-4 w-4" />
                      <span>JSON View</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>

            <SidebarSeparator />
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
  );
}
