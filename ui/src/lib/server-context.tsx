"use client";

import React, { createContext, useContext, useState, useEffect } from "react";
import { Config, Target, RBACConfig, Listener, TargetWithType, Bind } from "@/lib/types";
import { fetchBinds, fetchMcpTargets, fetchA2aTargets } from "@/lib/api";

interface ServerContextType {
  config: Config;
  setConfig: (config: Config) => void;
  isConnected: boolean;
  connectionError: string | null;
  binds: Bind[];
  listeners: Listener[];
  targets: Target[];
  policies: RBACConfig[];
  refreshBinds: () => Promise<void>;
  refreshListeners: () => Promise<void>;
  refreshTargets: () => Promise<void>;
  isConfigurationEmpty: () => Promise<boolean>;
}

const ServerContext = createContext<ServerContextType | undefined>(undefined);

export function ServerProvider({ children }: { children: React.ReactNode }) {
  const [config, setConfig] = useState<Config>({
    type: "static",
    listeners: [],
    targets: [],
    policies: [],
  });

  const [isConnected, setIsConnected] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [binds, setBinds] = useState<Bind[]>([]);
  const [listeners, setListeners] = useState<Listener[]>([]);
  const [targets, setTargets] = useState<TargetWithType[]>([]);
  const [policies, setPolicies] = useState<RBACConfig[]>([]);

  // Function to refresh binds
  const refreshBinds = async () => {
    try {
      const bindsData = await fetchBinds();
      setBinds(bindsData);

      // Extract all listeners from binds
      const allListeners: Listener[] = [];
      bindsData.forEach((bind) => {
        allListeners.push(...bind.listeners);
      });
      setListeners(allListeners);

      // Extract policies from listeners - in the new schema, policies are handled at the route level
      // For now, we'll set an empty array since policies are handled at the route level
      const allPolicies: any[] = [];
      setPolicies(allPolicies);
    } catch (err) {
      console.error("Error refreshing binds:", err);
      setConnectionError(err instanceof Error ? err.message : "Failed to refresh binds");
    }
  };

  // Function to refresh listeners (now delegates to refreshBinds)
  const refreshListeners = async () => {
    await refreshBinds();
  };

  // Function to refresh targets
  const refreshTargets = async () => {
    try {
      // Fetch MCP targets (A2A targets are no longer supported in the new schema)
      const mcpTargetsData = await fetchMcpTargets();

      // Set targets directly as they're already properly typed
      const targetsArray: TargetWithType[] = mcpTargetsData;
      setTargets(targetsArray);

      // Update the config with the new targets
      setConfig((prevConfig) => ({
        ...prevConfig,
        targets: targetsArray,
      }));
    } catch (err) {
      console.error("Error refreshing targets:", err);
      setConnectionError(err instanceof Error ? err.message : "Failed to refresh targets");
    }
  };

  // Load configuration from server on mount
  useEffect(() => {
    const loadConfiguration = async () => {
      try {
        // Fetch binds configuration
        const bindsData = await fetchBinds();
        setBinds(bindsData);

        // Extract all listeners from binds
        const allListeners: Listener[] = [];
        bindsData.forEach((bind) => {
          allListeners.push(...bind.listeners);
        });
        setListeners(allListeners);

        // Fetch MCP targets (A2A targets are no longer supported in the new schema)
        const mcpTargetsData = await fetchMcpTargets();

        // Set targets directly as they're already properly typed
        const targetsArray: TargetWithType[] = mcpTargetsData;
        setTargets(targetsArray);

        // Extract policies from listeners - in the new schema, policies are handled differently
        // For now, we'll set an empty array since policies are handled at the route level
        const allPolicies: any[] = [];
        setPolicies(allPolicies);

        // Update the config state with the loaded data
        setConfig({
          type: "static",
          listeners: allListeners,
          targets: targetsArray,
          policies: allPolicies,
        });

        setIsConnected(true);
        setConnectionError(null);
      } catch (err) {
        console.error("Error loading configuration:", err);
        setConnectionError(err instanceof Error ? err.message : "Failed to load configuration");
        setIsConnected(false);
      }
    };

    loadConfiguration();
  }, []);

  const isConfigurationEmpty = async () => {
    const bindsData = await fetchBinds();
    const mcpTargets = await fetchMcpTargets();
    const a2aTargets = await fetchA2aTargets();

    // Check if there are any listeners in any bind
    const hasListeners = bindsData.some((bind) => bind.listeners.length > 0);

    return !hasListeners && mcpTargets.length === 0 && a2aTargets.length === 0;
  };

  return (
    <ServerContext.Provider
      value={{
        config,
        setConfig,
        isConnected,
        connectionError,
        binds,
        listeners,
        targets,
        policies,
        refreshBinds,
        refreshListeners,
        refreshTargets,
        isConfigurationEmpty,
      }}
    >
      {children}
    </ServerContext.Provider>
  );
}

export function useServer() {
  const context = useContext(ServerContext);
  if (context === undefined) {
    throw new Error("useServer must be used within a ServerProvider");
  }
  return context;
}
