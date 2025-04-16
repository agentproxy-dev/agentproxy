"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { PlusCircle, Loader2, Info, Edit2 } from "lucide-react";
import { Target, Config } from "@/lib/types";
import { updateTarget, createMcpTarget, createA2aTarget } from "@/lib/api";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import TargetItem from "./target-item";
import { MCPTargetForm } from "./setup-wizard/targets/MCPTargetForm";
import { A2ATargetForm } from "./setup-wizard/targets/A2ATargetForm";
import { toast } from "@/lib/toast";

interface TargetsConfigProps {
  config: Config;
  onConfigChange: (config: Config) => void;
  serverAddress?: string;
  serverPort?: number;
}

export function TargetsConfig({
  config,
  onConfigChange,
  serverAddress = "0.0.0.0",
  serverPort = 19000,
}: TargetsConfigProps) {
  const [targetCategory, setTargetCategory] = useState<"mcp" | "a2a">("mcp");
  const [targetName, setTargetName] = useState("");
  const [isAddingTarget, setIsAddingTarget] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [editingTarget, setEditingTarget] = useState<Target | undefined>(undefined);

  const handleAddTarget = (target: Target) => {
    const newConfig = {
      ...config,
      targets: [...config.targets, target],
    };
    onConfigChange(newConfig);
  };

  const handleRemoveTarget = (index: number) => {
    const newConfig = {
      ...config,
      targets: config.targets.filter((_, i) => i !== index),
    };
    onConfigChange(newConfig);
    toast.success("Target removed", {
      description: "The target has been removed from your configuration.",
    });
  };

  const handleCreateTarget = async (target: Target) => {
    setIsAddingTarget(true);
    setError(null);

    try {
      if (targetCategory === "a2a") {
        await createA2aTarget(serverAddress, serverPort, target);
      } else {
        await createMcpTarget(serverAddress, serverPort, target);
      }

      handleAddTarget(target);
      setTargetName("");
      setIsDialogOpen(false);
      toast.success("Target created", {
        description: `Successfully created ${target.name} target.`,
      });
    } catch (err) {
      console.error("Error creating target:", err);
      setError(err instanceof Error ? err.message : "Failed to create target");
      toast.error("Error creating target", {
        description: err instanceof Error ? err.message : "Failed to create target",
      });
      throw err;
    } finally {
      setIsAddingTarget(false);
    }
  };

  const handleUpdateTarget = async (target: Target) => {
    setIsUpdating(true);
    try {
      await updateTarget(serverAddress, serverPort, target);

      // Update the target in the config
      const newConfig = {
        ...config,
        targets: config.targets.map((t) => (t.name === target.name ? target : t)),
      };
      onConfigChange(newConfig);

      setIsDialogOpen(false);
      setEditingTarget(undefined);
      toast.success("Target updated", {
        description: `Successfully updated ${target.name} target.`,
      });
    } catch (err) {
      console.error("Error updating target:", err);
      setError(err instanceof Error ? err.message : "Failed to update target");
      toast.error("Error updating target", {
        description: err instanceof Error ? err.message : "Failed to update target",
      });
    } finally {
      setIsUpdating(false);
    }
  };

  const openAddTargetDialog = () => {
    setTargetName("");
    setTargetCategory("mcp");
    setEditingTarget(undefined);
    setIsDialogOpen(true);
  };

  const openEditTargetDialog = (target: Target) => {
    setTargetName(target.name);
    setTargetCategory(target.a2a ? "a2a" : "mcp");
    setEditingTarget(target);
    setIsDialogOpen(true);
  };

  const handleDialogClose = () => {
    setIsDialogOpen(false);
    setEditingTarget(undefined);
    setTargetName("");
  };

  return (
    <Card className="w-full">
      <CardHeader>
        <div className="flex justify-between items-center">
          <div>
            <CardTitle>Target Servers</CardTitle>
            <CardDescription>
              Configure the servers that your proxy will forward requests to
            </CardDescription>
          </div>
          <Button onClick={openAddTargetDialog}>
            <PlusCircle className="h-4 w-4 mr-2" />
            Add Target
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          <div className="space-y-2">
            <h3 className="font-medium">What are Targets?</h3>
            <p className="text-sm text-muted-foreground">
              Targets are the destination servers that your proxy will forward requests to. You can
              add multiple targets and configure their connection settings.
            </p>
          </div>

          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {isUpdating && (
            <Alert>
              <AlertDescription className="flex items-center">
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Updating targets...
              </AlertDescription>
            </Alert>
          )}

          {config?.targets && config.targets.length > 0 ? (
            <div className="mt-6">
              <h3 className="font-medium mb-2">Configured Targets</h3>
              <div className="space-y-2">
                {config.targets.map((target, index) => (
                  <div
                    key={index}
                    className="flex items-center justify-between p-3 border rounded-md"
                  >
                    <TargetItem
                      target={target}
                      index={index}
                      onDelete={handleRemoveTarget}
                      isUpdating={isUpdating}
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => openEditTargetDialog(target)}
                      className="h-8 w-8 ml-2"
                      disabled={isUpdating}
                    >
                      <Edit2 className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <Alert>
              <Info className="h-4 w-4" />
              <AlertDescription>
                No targets configured yet. Click <strong>Add Target</strong> to create your first
                target.
              </AlertDescription>
            </Alert>
          )}
        </div>
      </CardContent>

      <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
        <DialogContent className="sm:max-w-[600px]">
          <DialogHeader>
            <DialogTitle>{editingTarget ? "Edit Target" : "Add New Target"}</DialogTitle>
            <DialogDescription>
              {editingTarget
                ? "Update the configuration for your target server."
                : "Configure a new target server for your proxy."}
            </DialogDescription>
          </DialogHeader>

          <Tabs
            value={targetCategory}
            onValueChange={(value) => setTargetCategory(value as "mcp" | "a2a")}
          >
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="mcp">MCP Target</TabsTrigger>
              <TabsTrigger value="a2a">A2A Target</TabsTrigger>
            </TabsList>

            <TabsContent value="mcp" className="space-y-4 pt-4">
              <Alert>
                <Info className="h-4 w-4" />
                <AlertDescription>
                  MCP (Model Control Protocol) targets are used to connect to AI model servers that
                  support the MCP protocol. These are typically used for AI model inference and
                  control.
                </AlertDescription>
              </Alert>

              <MCPTargetForm
                targetName={targetName}
                onTargetNameChange={setTargetName}
                onSubmit={editingTarget ? handleUpdateTarget : handleCreateTarget}
                isLoading={isAddingTarget || isUpdating}
                existingTarget={editingTarget}
              />
            </TabsContent>

            <TabsContent value="a2a" className="space-y-4 pt-4">
              <Alert>
                <Info className="h-4 w-4" />
                <AlertDescription>
                  A2A (Agent-to-Agent) targets are used to connect to other agent systems that
                  support the A2A protocol. These are typically used for agent-to-agent
                  communication and collaboration.
                </AlertDescription>
              </Alert>

              <A2ATargetForm
                targetName={targetName}
                onSubmit={editingTarget ? handleUpdateTarget : handleCreateTarget}
                isLoading={isAddingTarget || isUpdating}
                existingTarget={editingTarget}
              />
            </TabsContent>
          </Tabs>

          <DialogFooter>
            <Button variant="outline" onClick={handleDialogClose}>
              Cancel
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
