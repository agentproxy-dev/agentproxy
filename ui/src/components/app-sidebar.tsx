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
} from "@/components/ui/sidebar";
import { Button } from "@/components/ui/button";
import { MCPLogo } from "@/components/mcp-logo";
import { ThemeToggle } from "@/components/theme-toggle";
import { Target } from "@/lib/types";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Loader2, Home, Shield, Headphones, Server, Code, Settings } from "lucide-react";
import { fetchListeners, deleteListener } from "@/lib/api";
import { useLoading } from "@/lib/loading-context";
import { useRouter, usePathname } from "next/navigation";
import { useServer } from "@/lib/server-context";

interface AppSidebarProps {
  targets: any[];
  listeners: any[];
  activeView: string;
  setActiveView: (view: string) => void;
  addTarget: (target: Target) => void;
  onRestartWizard: () => void;
}

export function AppSidebar({
  targets,
  listeners,
  setActiveView,
  onRestartWizard,
}: AppSidebarProps) {
  const { setIsLoading } = useLoading();
  const [showRestartDialog, setShowRestartDialog] = useState(false);
  const [isDeletingListeners, setIsDeletingListeners] = useState(false);
  const router = useRouter();
  const pathname = usePathname();
  const {} = useServer();

  const handleRestartWizard = () => {
    setShowRestartDialog(true);
  };

  const confirmRestartWizard = async () => {
    try {
      setIsDeletingListeners(true);
      setIsLoading(true);

      // Fetch all listeners
      const listeners = await fetchListeners("0.0.0.0", 19000);

      // Delete each listener
      for (const listener of listeners) {
        // Create a unique identifier for each listener based on its properties
        await deleteListener("0.0.0.0", 19000, listener);
      }

      // Call the parent component's onRestartWizard function
      onRestartWizard();
    } catch (error) {
      console.error("Error restarting wizard:", error);
    } finally {
      setIsDeletingListeners(false);
      setIsLoading(false);
      setShowRestartDialog(false);
    }
  };

  const navigateTo = (path: string) => {
    router.push(path);
    setActiveView(path.split("/").pop() || "home");
  };

  return (
    <Sidebar>
      <SidebarHeader className="border-b">
        <div className="p-2 flex items-center justify-center mb-2">
          <MCPLogo className="h-10 w-auto" />
        </div>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Navigation</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton
                  tooltip="Home"
                  isActive={pathname === "/"}
                  onClick={() => navigateTo("/")}
                  aria-label="Home"
                >
                  <Home className="h-4 w-4" />
                  <span>Home</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton
                  tooltip="Listener Settings"
                  isActive={pathname === "/listeners"}
                  onClick={() => navigateTo("/listeners")}
                  aria-label="Listener Settings"
                >
                  <Headphones className="h-4 w-4" />
                  <span>Listener</span>
                  <SidebarMenuBadge>{listeners.length}</SidebarMenuBadge>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton
                  tooltip="Target Servers"
                  isActive={pathname === "/targets"}
                  onClick={() => navigateTo("/targets")}
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
                  isActive={pathname === "/policies"}
                  onClick={() => navigateTo("/policies")}
                  aria-label="Security Policies"
                >
                  <Shield className="h-4 w-4" />
                  <span>Policies</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton
                  tooltip="JSON Configuration"
                  isActive={pathname === "/json"}
                  onClick={() => navigateTo("/json")}
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
      </SidebarContent>

      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              tooltip="Restart Setup Wizard"
              onClick={handleRestartWizard}
              aria-label="Restart Setup Wizard"
            >
              <Settings className="h-4 w-4" />
              <span>Restart Setup</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <SidebarMenuButton tooltip="Theme">
              <ThemeToggle asChild className="flex items-center gap-2" />
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>

      <Dialog open={showRestartDialog} onOpenChange={setShowRestartDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Restart Setup Wizard</DialogTitle>
            <DialogDescription>
              Are you sure you want to restart the setup wizard? This will reset all your current
              configuration settings.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowRestartDialog(false)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={confirmRestartWizard}
              disabled={isDeletingListeners}
            >
              {isDeletingListeners ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Restarting...
                </>
              ) : (
                "Restart Wizard"
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Sidebar>
  );
}
