"use client";

import { useState, useEffect } from "react";
import { AppSidebar } from "@/components/app-sidebar";
import { ListenerConfig } from "@/components/listener-config";
import { TargetsConfig } from "@/components/targets-config";
import { PoliciesConfig } from "@/components/policies-config";
import {
  updateTarget,
  updatePolicies,
  fetchListeners,
  fetchTargets,
  fetchPolicies,
} from "@/lib/api";
import { SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { Config, Target, RBACConfig } from "@/lib/types";
import { NotConnectedState } from "@/components/not-connected-state";
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
  const [serverAddress, setServerAddress] = useState<string>("");
  const [serverPort, setServerPort] = useState<number>(19000);
  const [activeView, setActiveView] = useState<string>("home");
  const [configUpdateMessage, setConfigUpdateMessage] = useState<{
    success: boolean;
    message: string;
  } | null>(null);

  // Load saved connection from localStorage
  useEffect(() => {
    const savedAddress = localStorage.getItem("serverAddress");
    const savedPort = localStorage.getItem("serverPort");

    if (savedAddress && savedPort) {
      setServerAddress(savedAddress);
      setServerPort(parseInt(savedPort));
      connectToServer(savedAddress, parseInt(savedPort));
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

  const connectToServer = async (address: string, port: number) => {
    if (!address || !port) {
      setConnectionError("Please enter a valid server address and port");
      setIsLoading(false);
      return false;
    }

    setIsLoading(true);
    setConnectionError(null);

    try {
      // Fetch configuration from the proxy using API functions
      console.log("Fetching configuration from", `${address}:${port}`);

      // Fetch listeners configuration
      const listenersData = await fetchListeners(address, port);
      console.log("Received listeners data:", listenersData);

      // Fetch targets configuration
      const targetsData = await fetchTargets(address, port);
      console.log("Received targets data:", targetsData);

      // Convert targets object to array
      const targetsArray = Object.entries(targetsData).map(([_name, targetData]) => {
        return {
          ...(targetData as Target),
        };
      });
      console.log("Converted targets array:", targetsArray);

      // Fetch RBAC policies
      const rbacData = await fetchPolicies(address, port);
      console.log("Received RBAC data:", rbacData);

      // Update state with fetched configuration
      const newConfig = {
        type: "static" as const,
        listeners: [listenersData], // Use the listeners data directly
        targets: targetsArray,
        policies: rbacData || [],
      };

      setConfig(newConfig);
      setServerAddress(address);
      setServerPort(port);
      setIsConnected(true);
      return true;
    } catch (error) {
      console.error("Error connecting to server:", error);
      setConnectionError(error instanceof Error ? error.message : "Failed to connect to server");
      setIsConnected(false);
      return false;
    } finally {
      setIsLoading(false);
    }
  };

  const disconnectFromServer = () => {
    setIsConnected(false);
    setServerAddress("");
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
      // Add policy to local state
      const newConfig = {
        ...config,
        policies: [...(config.policies || []), policy],
      };
      setConfig(newConfig);

      // Update policies on server if connected
      if (serverAddress && serverPort) {
        // Convert RBACConfig to RBACPolicy format
        const rbacPolicies =
          newConfig.policies?.map((p) => ({
            name: p.name,
            namespace: p.namespace,
            rules: p.rules.map((rule) => ({
              key: rule.key,
              value: rule.value,
              resource: {
                type: rule.resource.type,
                id: rule.resource.id,
              },
              matcher: rule.matcher,
            })),
          })) || [];

        await updatePolicies(serverAddress, serverPort, rbacPolicies);
        handleConfigUpdate(true, "Policy added successfully");
      }
    } catch (error) {
      console.error("Error adding policy:", error);
      handleConfigUpdate(false, error instanceof Error ? error.message : "Failed to add policy");
    }
  };

  const handleRemovePolicy = async (index: number) => {
    try {
      // Remove policy from local state
      const newConfig = {
        ...config,
        policies: config.policies?.filter((_, i) => i !== index) || [],
      };
      setConfig(newConfig);

      // Update policies on server if connected
      if (serverAddress && serverPort) {
        // Convert RBACConfig to RBACPolicy format
        const rbacPolicies =
          newConfig.policies?.map((p) => ({
            name: p.name,
            namespace: p.namespace,
            rules: p.rules.map((rule) => ({
              key: rule.key,
              value: rule.value,
              resource: {
                type: rule.resource.type,
                id: rule.resource.id,
              },
              matcher: rule.matcher,
            })),
          })) || [];

        await updatePolicies(serverAddress, serverPort, rbacPolicies);
        handleConfigUpdate(true, "Policy removed successfully");
      }
    } catch (error) {
      console.error("Error removing policy:", error);
      handleConfigUpdate(false, error instanceof Error ? error.message : "Failed to remove policy");
    }
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
        ) : (
          <NotConnectedState onConnect={connectToServer} connectionError={connectionError || ""} />
        )}
      </div>
    </SidebarProvider>
  );
}
