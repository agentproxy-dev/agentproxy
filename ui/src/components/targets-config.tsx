"use client";

import { useState, forwardRef, useImperativeHandle } from "react";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Loader2, Info, Edit2, HelpCircle } from "lucide-react";
import { Target, Config } from "@/lib/types";
import {
  updateTarget,
  createMcpTarget,
  createA2aTarget,
  deleteA2aTarget,
  deleteMcpTarget,
} from "@/lib/api";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import TargetItem from "./target-item";
import { MCPTargetForm } from "./setup-wizard/targets/MCPTargetForm";
import { A2ATargetForm } from "./setup-wizard/targets/A2ATargetForm";
import { useServer } from "@/lib/server-context";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { ListenerSelect } from "./setup-wizard/targets/ListenerSelect";
import { toast } from "sonner";

interface TargetsConfigProps {
  config: Config;
  onConfigChange: (config: Config) => void;
  isAddingTarget?: boolean;
  setIsAddingTarget?: (isAdding: boolean) => void;
}

export const TargetsConfig = forwardRef<{ openAddTargetDialog: () => void }, TargetsConfigProps>(
  (
    {
      config,
      onConfigChange,
      isAddingTarget: externalIsAddingTarget,
      setIsAddingTarget: externalSetIsAddingTarget,
    },
    ref
  ) => {
    const { refreshTargets } = useServer();
    const [targetCategory, setTargetCategory] = useState<"mcp" | "a2a">("mcp");
    const [targetName, setTargetName] = useState("");
    const [selectedListeners, setSelectedListeners] = useState<string[]>([]);
    const [targetNameError, setTargetNameError] = useState<string | null>(null);
    const [internalIsAddingTarget, setInternalIsAddingTarget] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [isUpdating, setIsUpdating] = useState(false);
    const [isDialogOpen, setIsDialogOpen] = useState(false);
    const [editingTarget, setEditingTarget] = useState<Target | undefined>(undefined);
    const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
    const [targetToDelete, setTargetToDelete] = useState<{ index: number; name: string } | null>(
      null
    );

    // Use external state if provided, otherwise use internal state
    const isAddingTarget = externalIsAddingTarget ?? internalIsAddingTarget;
    const setIsAddingTarget = externalSetIsAddingTarget ?? setInternalIsAddingTarget;

    const handleRemoveTarget = async (index: number) => {
      const target = config.targets[index];
      setTargetToDelete({ index, name: target.name });
      setDeleteConfirmOpen(true);
    };

    const confirmDelete = async () => {
      if (!targetToDelete) return;

      try {
        const target = config.targets[targetToDelete.index];

        // Call the appropriate delete API based on target type
        if (target.a2a) {
          await deleteA2aTarget(target.name);
        } else {
          await deleteMcpTarget(target.name);
        }

        // Only update local state after successful API call
        const newConfig = {
          ...config,
          targets: config.targets.filter((_, i) => i !== targetToDelete.index),
        };
        onConfigChange(newConfig);

        // Refresh targets from the server
        await refreshTargets();

        toast.success("Target removed", {
          description: "The target has been removed from your configuration.",
        });
      } catch (err) {
        console.error("Error deleting target:", err);
        toast.error("Error deleting target", {
          description:
            err instanceof Error
              ? err.message
              : "Failed to delete target. The target may be in use or the server encountered an error.",
        });
      } finally {
        setDeleteConfirmOpen(false);
        setTargetToDelete(null);
      }
    };

    const handleCreateTarget = async (target: Target) => {
      // Validate target name
      if (!targetName.trim()) {
        setTargetNameError("Target name is required");
        return;
      }

      if (setIsAddingTarget) {
        setIsAddingTarget(true);
      }
      setError(null);
      setTargetNameError(null);

      try {
        const targetWithListeners = {
          ...target,
          listeners: selectedListeners,
        };

        if (targetCategory === "a2a") {
          await createA2aTarget(targetWithListeners);
        } else {
          await createMcpTarget(targetWithListeners);
        }

        // Refresh targets from the server
        await refreshTargets();

        setTargetName("");
        setSelectedListeners([]);
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
      } finally {
        if (setIsAddingTarget) {
          setIsAddingTarget(false);
        }
      }
    };

    const handleUpdateTarget = async (target: Target) => {
      setIsUpdating(true);
      try {
        const targetWithListeners = {
          ...target,
          listeners: selectedListeners,
        };
        await updateTarget(targetWithListeners);

        // Refresh targets from the server
        await refreshTargets();

        setIsDialogOpen(false);
        setEditingTarget(undefined);
        setSelectedListeners([]);
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
      setSelectedListeners([]);
      setTargetNameError(null);
      setTargetCategory("mcp");
      setEditingTarget(undefined);
      setIsDialogOpen(true);
    };

    const openEditTargetDialog = (target: Target) => {
      setTargetName(target.name);
      setSelectedListeners(target.listeners || []);
      if (target.a2a) {
        setTargetCategory("a2a");
      } else {
        setTargetCategory("mcp");
      }
      setEditingTarget(target);
      setIsDialogOpen(true);
    };

    const handleDialogClose = () => {
      setIsDialogOpen(false);
      setEditingTarget(undefined);
      setTargetName("");
      setSelectedListeners([]);
      setTargetNameError(null);
      if (setIsAddingTarget) {
        setIsAddingTarget(false);
      }
    };

    useImperativeHandle(ref, () => ({
      openAddTargetDialog,
    }));

    return (
      <div>
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
              No targets configured yet. Click &apos;Add Target&apos; to create your first target.
            </AlertDescription>
          </Alert>
        )}

        <Dialog open={isDialogOpen} onOpenChange={handleDialogClose}>
          <DialogContent className="sm:max-w-[600px]">
            <DialogHeader>
              <DialogTitle>{editingTarget ? "Edit Target" : "Add New Target"}</DialogTitle>
              <DialogDescription>
                {editingTarget
                  ? "Update the configuration for your target server."
                  : "Configure a new target server for your proxy."}
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="targetName">
                  Target Name <span className="text-red-500">*</span>
                </Label>
                <Input
                  id="targetName"
                  value={targetName}
                  onChange={(e) => {
                    setTargetName(e.target.value);
                    if (e.target.value.trim()) {
                      setTargetNameError(null);
                    }
                  }}
                  placeholder="e.g., local-model"
                  className={targetNameError ? "border-red-500" : ""}
                />
                {targetNameError && <p className="text-sm text-red-500">{targetNameError}</p>}
              </div>

              <div className="space-y-2">
                <ListenerSelect
                  selectedListeners={selectedListeners}
                  onListenersChange={setSelectedListeners}
                />
              </div>

              <div className="space-y-2">
                <div className="flex items-center space-x-2">
                  <Label>Target Type</Label>
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger>
                        <HelpCircle className="h-4 w-4 text-muted-foreground" />
                      </TooltipTrigger>
                      <TooltipContent>
                        <p className="max-w-xs">
                          Choose between MCP for AI model servers or A2A for agent-to-agent
                          communication
                        </p>
                      </TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                </div>

                <RadioGroup
                  value={targetCategory}
                  onValueChange={(value) => {
                    setTargetCategory(value as "mcp" | "a2a");
                    // Clear editing target when switching categories to prevent form field confusion
                    if (
                      editingTarget &&
                      ((value === "a2a" && !editingTarget.a2a) ||
                        (value === "mcp" && editingTarget.a2a))
                    ) {
                      setEditingTarget(undefined);
                    }
                  }}
                  className="grid grid-cols-2 gap-4"
                >
                  <div className="flex items-center space-x-2">
                    <TooltipProvider>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <div className="flex items-center space-x-2">
                            <RadioGroupItem value="mcp" id="mcp" />
                            <Label htmlFor="mcp" className="cursor-pointer">
                              MCP Target
                            </Label>
                          </div>
                        </TooltipTrigger>
                        <TooltipContent>
                          <p className="max-w-xs">
                            MCP (Model Control Protocol) targets are used to connect to AI model
                            servers that support the MCP protocol. These are typically used for AI
                            model inference and control.
                          </p>
                        </TooltipContent>
                      </Tooltip>
                    </TooltipProvider>
                  </div>

                  <div className="flex items-center space-x-2">
                    <TooltipProvider>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <div className="flex items-center space-x-2">
                            <RadioGroupItem value="a2a" id="a2a" />
                            <Label htmlFor="a2a" className="cursor-pointer">
                              A2A Target
                            </Label>
                          </div>
                        </TooltipTrigger>
                        <TooltipContent>
                          <p className="max-w-xs">
                            A2A (Agent-to-Agent) targets are used to connect to other agent systems
                            that support the A2A protocol. These are typically used for
                            agent-to-agent communication and collaboration.
                          </p>
                        </TooltipContent>
                      </Tooltip>
                    </TooltipProvider>
                  </div>
                </RadioGroup>
              </div>

              <div className="pt-4">
                {targetCategory === "mcp" ? (
                  <MCPTargetForm
                    targetName={targetName}
                    onTargetNameChange={setTargetName}
                    onSubmit={editingTarget ? handleUpdateTarget : handleCreateTarget}
                    isLoading={isAddingTarget || isUpdating}
                    existingTarget={editingTarget?.a2a ? undefined : editingTarget}
                  />
                ) : (
                  <A2ATargetForm
                    targetName={targetName}
                    onSubmit={editingTarget ? handleUpdateTarget : handleCreateTarget}
                    isLoading={isAddingTarget || isUpdating}
                    existingTarget={editingTarget?.a2a ? editingTarget : undefined}
                  />
                )}
              </div>
            </div>

            <DialogFooter className="gap-2 mt-6">
              <Button
                variant="outline"
                onClick={handleDialogClose}
                disabled={isAddingTarget || isUpdating}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                form={targetCategory === "mcp" ? "mcp-target-form" : "a2a-target-form"}
                disabled={isAddingTarget || isUpdating}
              >
                {isAddingTarget || isUpdating ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    {editingTarget ? "Updating..." : "Adding..."}
                  </>
                ) : editingTarget ? (
                  "Update Target"
                ) : (
                  "Add Target"
                )}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <Dialog open={deleteConfirmOpen} onOpenChange={setDeleteConfirmOpen}>
          <DialogContent className="sm:max-w-[425px]">
            <DialogHeader>
              <DialogTitle>Confirm Deletion</DialogTitle>
              <DialogDescription>
                Are you sure you want to delete the target &quot;{targetToDelete?.name}&quot;? This
                action cannot be undone.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => {
                  setDeleteConfirmOpen(false);
                  setTargetToDelete(null);
                }}
              >
                Cancel
              </Button>
              <Button variant="destructive" onClick={confirmDelete}>
                Delete
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    );
  }
);

TargetsConfig.displayName = "TargetsConfig";
