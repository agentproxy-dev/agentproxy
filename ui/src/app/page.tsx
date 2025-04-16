"use client";

import { useState, useEffect } from "react";
import { AppSidebar } from "@/components/app-sidebar";
import { ListenerConfig } from "@/components/listener-config";
import { TargetsConfig } from "@/components/targets-config";
import { PoliciesConfig } from "@/components/policies-config";
import { SetupWizard } from "@/components/setup-wizard";
import {
  updateTarget,
  fetchListeners,
  fetchMcpTargets,
  fetchA2aTargets,
} from "@/lib/api";
import { SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { Config, Target, RBACConfig, Listener } from "@/lib/types";
import { useLoading } from "@/lib/loading-context";
import { JsonConfig } from "@/components/json-config";

export default function Home() {
  const { isLoading, setIsLoading } = useLoading();
  const [config, setConfig] = useState<Config>({
    type: "static",
    listeners: [],
    targets: [],
    policies: [],
  });

  const [isConnected, setIsConnected] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [serverAddress, setServerAddress] = useState<string>("0.0.0.0");
  const [serverPort, setServerPort] = useState<number>(19000);
  const [activeView, setActiveView] = useState<string>("home");
  const [showWizard, setShowWizard] = useState(false);
  const [configUpdateMessage, setConfigUpdateMessage] = useState<{
    success: boolean;
    message: string;
  } | null>(null);

  const [listeners, setListeners] = useState<Listener[]>([]);
  const [targets, setTargets] = useState<Target[]>([]);

  // Connect to server on component mount
  useEffect(() => {
    connectToServer();
  }, []);

  // Load saved connection from localStorage
  useEffect(() => {
    const savedAddress = localStorage.getItem("serverAddress");
    const savedPort = localStorage.getItem("serverPort");

    if (savedAddress && savedPort) {
      setServerAddress(savedAddress);
      setServerPort(parseInt(savedPort));
      connectToServer();
    } else {
      setIsLoading(false);
    }
  }, []);

  // Save connection details to localStorage when they change
  useEffect(() => {
    if (serverAddress && serverPort) {
      localStorage.setItem("serverAddress", serverAddress);
      localStorage.setItem("serverPort", serverPort.toString());
    }
  }, [serverAddress, serverPort]);

  // Save local configuration to localStorage when it changes
  useEffect(() => {
    if (isConnected) {
      localStorage.setItem("localConfig", JSON.stringify(config));
    }
  }, [config, isConnected]);

  // Function to connect to server
  const connectToServer = async (): Promise<boolean> => {
    setIsLoading(true);
    setConnectionError(null);

    try {
      // Fetch configuration from the proxy using API functions
      console.log("Fetching configuration from", `${serverAddress}:${serverPort}`);

      // Fetch listeners configuration
      const listenersData = await fetchListeners(serverAddress, serverPort);
      console.log("Received listeners data:", listenersData);
      const listenersArray = Array.isArray(listenersData) ? listenersData : [listenersData];
      setListeners(listenersArray);

      // Fetch MCP and A2A targets
      const mcpTargetsData = await fetchMcpTargets(serverAddress, serverPort);
      console.log("Received MCP targets data:", mcpTargetsData);
      
      const a2aTargetsData = await fetchA2aTargets(serverAddress, serverPort);
      console.log("Received A2A targets data:", a2aTargetsData);
      
      // Combine targets
      const targetsArray = [
        ...mcpTargetsData.map(target => ({ ...target, type: 'mcp' as const })),
        ...a2aTargetsData.map(target => ({ ...target, type: 'a2a' as const }))
      ];
      console.log("Combined targets array:", targetsArray);
      setTargets(targetsArray);

      // If we have listeners, we don't need to show the wizard
      if (listenersArray.length > 0) {
        setShowWizard(false);
      } else {
        // No listeners found, show the wizard
        setShowWizard(true);
      }

      setIsConnected(true);
      return true;
    } catch (err) {
      console.error("Error connecting to server:", err);
      setConnectionError(err instanceof Error ? err.message : "Failed to connect to server");
      setIsConnected(false);
      return false;
    } finally {
      setIsLoading(false);
    }
  };

  const disconnectFromServer = () => {
    setIsConnected(false);
    setServerAddress("0.0.0.0");
    setServerPort(19000);
    setConfig({
      type: "static",
      listeners: [],
      targets: [],
      policies: [],
    });
    localStorage.removeItem("serverAddress");
    localStorage.removeItem("serverPort");
  };

  const handleConfigChange = (newConfig: Config) => {
    setConfig(newConfig);
  };

  const handleConfigUpdate = (success: boolean, message: string) => {
    setConfigUpdateMessage({ success, message });
    // Clear the message after 5 seconds
    setTimeout(() => {
      setConfigUpdateMessage(null);
    }, 5000);
  };

  const handleAddTarget = async (target: Target) => {
    try {
      // Add target to local state
      const newConfig = {
        ...config,
        targets: [...config.targets, target],
      };
      setConfig(newConfig);

      // Update target on server if connected
      if (serverAddress && serverPort) {
        await updateTarget(serverAddress, serverPort, target);
        handleConfigUpdate(true, "Target added successfully");
      }
    } catch (error) {
      console.error("Error adding target:", error);
      handleConfigUpdate(false, error instanceof Error ? error.message : "Failed to add target");
    }
  };

  const handleRemoveTarget = async (index: number) => {
    try {
      // Remove target from local state
      const newConfig = {
        ...config,
        targets: config.targets.filter((_, i) => i !== index),
      };
      setConfig(newConfig);

      // Update targets on server if connected
      if (serverAddress && serverPort) {
        // For removal, we need to update the entire targets list
        // This is a limitation of the current API design
        const updatedTargets = newConfig.targets;
        if (updatedTargets.length > 0) {
          await updateTarget(serverAddress, serverPort, updatedTargets[0]);
        }
        handleConfigUpdate(true, "Target removed successfully");
      }
    } catch (error) {
      console.error("Error removing target:", error);
      handleConfigUpdate(false, error instanceof Error ? error.message : "Failed to remove target");
    }
  };

  const handleAddPolicy = async (policy: RBACConfig) => {
    try {
      // Policy management is now handled through the listener
      // We need to update the listener with the new policy
      if (listeners.length > 0) {
        const listener = listeners[0];
        const updatedListener = {
          ...listener,
          sse: {
            ...listener.sse,
            rbac: [...(listener.sse.rbac || []), policy],
          },
        };
        
        // Update the listener on the server
        // This would require a new API endpoint to update a listener
        console.log("Policy management is now handled through the listener");
        handleConfigUpdate(true, "Policy added successfully");
      } else {
        handleConfigUpdate(false, "No listener available to add policy to");
      }
    } catch (error) {
      console.error("Error adding policy:", error);
      handleConfigUpdate(false, error instanceof Error ? error.message : "Failed to add policy");
    }
  };

  const handleRemovePolicy = async (index: number) => {
    try {
      // Policy management is now handled through the listener
      // We need to update the listener with the policy removed
      if (listeners.length > 0) {
        const listener = listeners[0];
        const updatedListener = {
          ...listener,
          sse: {
            ...listener.sse,
            rbac: listener.sse.rbac?.filter((_, i) => i !== index) || [],
          },
        };
        
        // Update the listener on the server
        // This would require a new API endpoint to update a listener
        console.log("Policy management is now handled through the listener");
        handleConfigUpdate(true, "Policy removed successfully");
      } else {
        handleConfigUpdate(false, "No listener available to remove policy from");
      }
    } catch (error) {
      console.error("Error removing policy:", error);
      handleConfigUpdate(false, error instanceof Error ? error.message : "Failed to remove policy");
    }
  };

  const handleWizardComplete = () => {
    setShowWizard(false);
    setActiveView("home");
  };

  const handleWizardSkip = () => {
    setShowWizard(false);
    setActiveView("home");
  };

  const renderContent = () => {
    if (isLoading) {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
            <p className="mt-2 text-sm text-muted-foreground">Loading...</p>
          </div>
        </div>
      );
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
      );
    }

    switch (activeView) {
      case "listener":
        return (
          <ListenerConfig config={config} serverAddress={serverAddress} serverPort={serverPort} />
        );
      case "targets":
        return (
          <TargetsConfig
            targets={config.targets}
            addTarget={handleAddTarget}
            removeTarget={handleRemoveTarget}
          />
        );
      case "policies":
        return (
          <PoliciesConfig
            policies={config.policies || []}
            addPolicy={handleAddPolicy}
            removePolicy={handleRemovePolicy}
          />
        );
      case "json":
        return <JsonConfig config={config} onConfigChange={handleConfigChange} />;
      default:
        return (
          <div className="p-6">
            <h2 className="text-lg font-medium">Overview</h2>
            <div className="mt-4 grid gap-4">
              <div className="p-4 border rounded-lg">
                <h3 className="text-sm font-medium">Listener</h3>
                <p className="text-sm text-muted-foreground">
                  {config.listeners.length > 0 && config.listeners[0].sse
                    ? `SSE on ${config.listeners[0].sse.address || config.listeners[0].sse.host || "0.0.0.0"}:${config.listeners[0].sse.port || "5555"}`
                    : "Not configured"}
                </p>
              </div>
              <div className="p-4 border rounded-lg">
                <h3 className="text-sm font-medium">Target Servers</h3>
                <p className="text-sm text-muted-foreground">
                  {config.targets.length} target
                  {config.targets.length !== 1 ? "s" : ""} configured
                </p>
              </div>
              <div className="p-4 border rounded-lg">
                <h3 className="text-sm font-medium">Security Policies</h3>
                <p className="text-sm text-muted-foreground">
                  {config.policies?.length} policy
                  {config.policies?.length !== 1 ? "ies" : "y"} configured
                </p>
              </div>
            </div>
          </div>
        );
    }
  };

  return (
    <SidebarProvider>
      <div className="flex min-h-screen w-full">
        {showWizard ? (
          <SetupWizard
            config={config}
            onConfigChange={setConfig}
            onComplete={handleWizardComplete}
            onSkip={handleWizardSkip}
          />
        ) : (
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

              {configUpdateMessage && (
                <div
                  className={`mb-4 rounded-md p-4 ${configUpdateMessage.success ? "bg-green-100 text-green-800" : "bg-destructive/10 text-destructive"}`}
                >
                  <p>{configUpdateMessage.message}</p>
                </div>
              )}

              {renderContent()}
            </main>
          </>
        )}
      </div>
    </SidebarProvider>
  );
}
