"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { PlusCircle, Server, Globe, Terminal, Loader2, Network } from "lucide-react";
import { Target, TargetType, Header } from "@/lib/types";
import { updateTarget } from "@/lib/api";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import TargetItem from "./target-item";

interface TargetsConfigProps {
  targets: Target[];
  addTarget: (target: Target) => void;
  removeTarget: (index: number) => void;
  serverAddress?: string;
  serverPort?: number;
  onConfigUpdate?: (success: boolean, message: string) => void;
}

export function TargetsConfig({
  targets,
  addTarget,
  removeTarget,
  serverAddress,
  serverPort,
  onConfigUpdate,
}: TargetsConfigProps) {
  const [isAddingTarget, setIsAddingTarget] = useState(false);
  const [serverType, setServerType] = useState<TargetType>("sse");
  const [serverName, setServerName] = useState("");
  const [url, setUrl] = useState("");
  const [command, setCommand] = useState("npx");
  const [args, setArgs] = useState("");
  const [targetToDelete, setTargetToDelete] = useState<number | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    const targetConfig: Target = {
      name: serverName,
    };

    if (serverType === "stdio") {
      targetConfig.stdio = {
        cmd: command,
        args: args.split(" ").filter((arg) => arg.trim() !== ""),
        env: {},
      };
    } else if (serverType === "sse") {
      try {
        const urlObj = new URL(url);
        let port: number;
        if (urlObj.port) {
          port = parseInt(urlObj.port, 10);
        } else {
          port = urlObj.protocol === "https:" ? 443 : 80;
        }
        targetConfig.sse = {
          host: urlObj.hostname,
          port: port,
          path: urlObj.pathname + urlObj.search,
          headers: [],
        };
      } catch (error) {
        console.error("Invalid URL:", error);
        return;
      }
    } else if (serverType === "openapi") {
      targetConfig.openapi = {
        host: url,
        port: 80,
        schema: {
          file_path: "",
        },
      };
    } else if (serverType === "a2a") {
      try {
        const urlObj = new URL(url);
        let port: number;
        if (urlObj.port) {
          port = parseInt(urlObj.port, 10);
        } else {
          port = urlObj.protocol === "https:" ? 443 : 80;
        }
        targetConfig.a2a = {
          host: urlObj.hostname,
          port: port,
          path: urlObj.pathname + urlObj.search,
          headers: [],
        };
      } catch (error) {
        console.error("Invalid URL:", error);
        return;
      }
    }

    // Add target to local state
    addTarget(targetConfig);

    // Update target on server
    if (serverAddress && serverPort) {
      setIsUpdating(true);
      try {
        await updateTarget(serverAddress, serverPort, targetConfig);
        if (onConfigUpdate) {
          onConfigUpdate(true, "Target added successfully");
        }
      } catch (error) {
        console.error("Error adding target:", error);
        if (onConfigUpdate) {
          onConfigUpdate(false, error instanceof Error ? error.message : "Failed to add target");
        }
      } finally {
        setIsUpdating(false);
      }
    }

    resetForm();
    setIsAddingTarget(false);
  };

  const resetForm = () => {
    setServerName("");
    setUrl("");
    setCommand("npx");
    setArgs("");
  };

  const handleDeleteTarget = (index: number) => {
    setTargetToDelete(index);
  };

  const confirmDelete = async () => {
    if (targetToDelete !== null) {
      // Remove target from local state
      removeTarget(targetToDelete);

      // Update targets on server
      if (serverAddress && serverPort) {
        setIsUpdating(true);
        try {
          // For deletion, we need to update the entire targets list
          // This is a limitation of the current API design
          const updatedTargets = targets.filter((_, i) => i !== targetToDelete);
          if (updatedTargets.length > 0) {
            await updateTarget(serverAddress, serverPort, updatedTargets[0]);
          }
          if (onConfigUpdate) {
            onConfigUpdate(true, "Target removed successfully");
          }
        } catch (error) {
          console.error("Error removing target:", error);
          if (onConfigUpdate) {
            onConfigUpdate(
              false,
              error instanceof Error ? error.message : "Failed to remove target"
            );
          }
        } finally {
          setIsUpdating(false);
        }
      }

      setTargetToDelete(null);
    }
  };

  const cancelDelete = () => {
    setTargetToDelete(null);
  };

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-medium mb-2">Target Servers</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Configure servers that the proxy connects to
        </p>
      </div>

      {isUpdating && (
        <Alert>
          <AlertDescription className="flex items-center">
            <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            Updating targets on server...
          </AlertDescription>
        </Alert>
      )}

      {targets.length === 0 && !isAddingTarget ? (
        <Alert>
          <AlertDescription>
            No target servers configured. Add a server to get started.
          </AlertDescription>
        </Alert>
      ) : (
        <div className="space-y-4">
          {targets.map((target, index) => (
            <TargetItem
              key={index}
              target={target}
              index={index}
              onDelete={handleDeleteTarget}
              isUpdating={isUpdating}
            />
          ))}
        </div>
      )}

      <Button
        onClick={() => setIsAddingTarget(true)}
        className="flex items-center"
        disabled={isUpdating}
      >
        <PlusCircle className="h-4 w-4 mr-2" />
        Add Target Server
      </Button>

      <Dialog open={isAddingTarget} onOpenChange={setIsAddingTarget}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Target Server</DialogTitle>
            <DialogDescription>
              Configure a new target server for the proxy to connect to.
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSubmit} className="space-y-4 mt-6">
            <div className="space-y-2">
              <Label htmlFor="name">Server Name</Label>
              <Input
                id="name"
                value={serverName}
                onChange={(e) => setServerName(e.target.value)}
                placeholder="Enter server name"
                required
                disabled={isUpdating}
              />
            </div>

            <div className="space-y-2">
              <Label>Server Type</Label>
              <Tabs
                value={serverType}
                onValueChange={(value) => setServerType(value as TargetType)}
              >
                <TabsList className="grid grid-cols-4">
                  <TabsTrigger value="sse" className="flex items-center">
                    <Globe className="h-4 w-4 mr-2" />
                    SSE
                  </TabsTrigger>
                  <TabsTrigger value="stdio" className="flex items-center">
                    <Terminal className="h-4 w-4 mr-2" />
                    stdio
                  </TabsTrigger>
                  <TabsTrigger value="openapi" className="flex items-center">
                    <Server className="h-4 w-4 mr-2" />
                    OpenAPI
                  </TabsTrigger>
                  <TabsTrigger value="a2a" className="flex items-center">
                    <Network className="h-4 w-4 mr-2" />
                    A2A
                  </TabsTrigger>
                </TabsList>

                <TabsContent value="sse" className="space-y-4 pt-4">
                  <div className="space-y-2">
                    <Label htmlFor="url">Server URL</Label>
                    <Input
                      id="url"
                      type="url"
                      value={url}
                      onChange={(e) => setUrl(e.target.value)}
                      placeholder="http://localhost:3000/events"
                      required
                      disabled={isUpdating}
                    />
                    <p className="text-sm text-muted-foreground">
                      Enter the full URL including protocol, hostname, port, and path
                    </p>
                  </div>
                </TabsContent>

                <TabsContent value="stdio" className="space-y-4 pt-4">
                  <div className="space-y-2">
                    <Label htmlFor="command">Command</Label>
                    <Input
                      id="command"
                      value={command}
                      onChange={(e) => setCommand(e.target.value)}
                      placeholder="npx"
                      required
                      disabled={isUpdating}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="args">Arguments</Label>
                    <Input
                      id="args"
                      value={args}
                      onChange={(e) => setArgs(e.target.value)}
                      placeholder="--port 3000"
                      disabled={isUpdating}
                    />
                  </div>
                </TabsContent>

                <TabsContent value="openapi" className="space-y-4 pt-4">
                  <div className="space-y-2">
                    <Label htmlFor="openapi-url">Server URL</Label>
                    <Input
                      id="openapi-url"
                      type="url"
                      value={url}
                      onChange={(e) => setUrl(e.target.value)}
                      placeholder="http://localhost:3000"
                      required
                      disabled={isUpdating}
                    />
                  </div>
                </TabsContent>

                <TabsContent value="a2a" className="space-y-4 pt-4">
                  <div className="space-y-2">
                    <Label htmlFor="a2a-url">Server URL</Label>
                    <Input
                      id="a2a-url"
                      type="url"
                      value={url}
                      onChange={(e) => setUrl(e.target.value)}
                      placeholder="http://localhost:3000"
                      required
                      disabled={isUpdating}
                    />
                    <p className="text-sm text-muted-foreground">
                      Enter the full URL including protocol, hostname, port, and path
                    </p>
                  </div>
                </TabsContent>
              </Tabs>

              <div className="flex justify-end">
                <Button type="submit" disabled={isUpdating}>
                  {isUpdating ? (
                    <>
                      <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                      Adding...
                    </>
                  ) : (
                    "Add Server"
                  )}
                </Button>
              </div>
            </div>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={targetToDelete !== null} onOpenChange={(open) => !open && cancelDelete()}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Target Server</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this target server? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-2 mt-4">
            <Button variant="outline" onClick={cancelDelete} disabled={isUpdating}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={confirmDelete} disabled={isUpdating}>
              {isUpdating ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Deleting...
                </>
              ) : (
                "Delete"
              )}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
