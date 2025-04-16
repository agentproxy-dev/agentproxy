"use client";

import { useState, useEffect } from "react";
import { AppSidebar } from "@/components/app-sidebar";
import { useServer } from "@/lib/server-context";
import { useLoading } from "@/lib/loading-context";

export function SidebarWrapper() {
  const { targets, listeners, setConfig, isConnected } = useServer();
  const { setIsLoading } = useLoading();
  const [activeView, setActiveView] = useState("home");

  // Update loading state based on connection status
  useEffect(() => {
    if (isConnected) {
      setIsLoading(false);
    }
  }, [isConnected, setIsLoading]);

  const handleRestartWizard = () => {
    // Reset the configuration
    setConfig({
      type: "static",
      listeners: [],
      targets: [],
      policies: [],
    });
  };

  return (
    <AppSidebar
      targets={targets}
      listeners={listeners}
      activeView={activeView}
      setActiveView={setActiveView}
      addTarget={() => {}}
      onRestartWizard={handleRestartWizard}
    />
  );
}
