"use client";

import { useState, forwardRef, useImperativeHandle, useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Loader2, Info, Edit2, AlertCircle } from "lucide-react";
import { Target, Config, TargetWithType, Listener, ListenerProtocol } from "@/lib/types";
import {
  updateTarget,
  createMcpTarget,
  createA2aTarget,
  deleteA2aTarget,
  deleteMcpTarget,
  fetchListeners,
} from "@/lib/api";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from "@/components/ui/dialog";
import TargetItem from "./target-item";
import { MCPTargetForm } from "./setup-wizard/targets/MCPTargetForm";
import { A2ATargetForm } from "./setup-wizard/targets/A2ATargetForm";
import { useServer } from "@/lib/server-context";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { ListenerSelect } from "./setup-wizard/targets/ListenerSelect";
import { toast } from "sonner";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { cn } from "@/lib/utils";

type SelectableTargetType = "a2a" | "mcp";

interface TargetsConfigProps {
  config: Config;
}

export const TargetsConfig = forwardRef<{ openAddTargetDialog: () => void }, TargetsConfigProps>(
  ({ config }, ref) => {
    const { refreshTargets } = useServer();
    const [isDialogOpen, setIsDialogOpen] = useState(false);
    const [editingTarget, setEditingTarget] = useState<TargetWithType | undefined>(undefined);

    const [selectedTargetType, setSelectedTargetType] = useState<SelectableTargetType | null>(null);
    const [targetName, setTargetName] = useState("");
    const [selectedListeners, setSelectedListeners] = useState<string[] | undefined>([]);
    const [allListeners, setAllListeners] = useState<Listener[]>([]);
    const [compatibleListeners, setCompatibleListeners] = useState<Listener[]>([]);

    const [targetNameError, setTargetNameError] = useState<string | null>(null);
    const [listenerError, setListenerError] = useState<string | null>(null);
    const [formError, setFormError] = useState<string | null>(null);
    const [isLoadingListeners, setIsLoadingListeners] = useState(false);
    const [isSubmitting, setIsSubmitting] = useState(false);

    const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
    const [targetToDelete, setTargetToDelete] = useState<{ index: number; name: string } | null>(
      null
    );

    const mcpFormRef = useRef<{ submitForm: () => Promise<void> } | null>(null);
    const a2aFormRef = useRef<{ submitForm: () => Promise<void> } | null>(null);

    useEffect(() => {
      if (isDialogOpen) {
        const loadListeners = async () => {
          setIsLoadingListeners(true);
          setListenerError(null);
          try {
            const fetchedListeners = await fetchListeners();
            setAllListeners(fetchedListeners);
          } catch (err) {
            console.error("Error fetching listeners:", err);
            setListenerError(err instanceof Error ? err.message : "Failed to load listeners.");
            setAllListeners([]);
          } finally {
            setIsLoadingListeners(false);
          }
        };
        loadListeners();
      } else {
        setAllListeners([]);
        setCompatibleListeners([]);
      }
    }, [isDialogOpen]);

    useEffect(() => {
      if (!selectedTargetType) {
        setCompatibleListeners([]);
        setListenerError(null);
        return;
      }

      const requiredProtocol =
        selectedTargetType.toLocaleUpperCase() === "A2A" ? ListenerProtocol.A2A : ListenerProtocol.MCP;

      const filtered = allListeners.filter((listener) => listener.protocol === requiredProtocol);
      setCompatibleListeners(filtered);

      console.log("filtered", filtered);

      if (filtered.length === 0 && !isLoadingListeners) {
        setListenerError(`No ${requiredProtocol} listeners found. Please create one first.`);
      } else {
        setListenerError(null);
      }

      setSelectedListeners(
        editingTarget?.listeners?.filter((lName) => filtered.some((cl) => cl.name === lName)) ?? []
      );
    }, [selectedTargetType, allListeners, isLoadingListeners, editingTarget]);

    const resetFormState = () => {
      setEditingTarget(undefined);
      setSelectedTargetType(null);
      setTargetName("");
      setSelectedListeners([]);
      setCompatibleListeners([]);
      setTargetNameError(null);
      setListenerError(null);
      setFormError(null);
      setIsSubmitting(false);
    };

    const handleRemoveTarget = async (index: number) => {
      const target = config.targets[index];
      setTargetToDelete({ index, name: target.name });
      setDeleteConfirmOpen(true);
    };

    const confirmDelete = async () => {
      console.log("confirming delete", targetToDelete);
      if (!targetToDelete) return;
      setIsSubmitting(true);
      try {
        const target = config.targets[targetToDelete.index];
        if (target.type === "a2a") {
          await deleteA2aTarget(target.name);
        } else {
          await deleteMcpTarget(target.name);
        }
        await refreshTargets();
        toast.success("Target removed", { description: `Target '${target.name}' removed.` });
        setDeleteConfirmOpen(false);
        setTargetToDelete(null);
      } catch (err) {
        console.error("Error deleting target:", err);
        toast.error("Error deleting target", {
          description: err instanceof Error ? err.message : "Failed to delete target.",
        });
      } finally {
        setIsSubmitting(false);
        if (deleteConfirmOpen) setDeleteConfirmOpen(false);
        if (targetToDelete) setTargetToDelete(null);
      }
    };

    const handleFormSubmit = async (formData: Omit<Target, "name" | "listeners">) => {
      if (!selectedTargetType) {
        setFormError("Please select a target type (MCP or A2A).");
        return;
      }
      if (!targetName.trim()) {
        setTargetNameError("Target name is required.");
        return;
      }
      if (!selectedListeners || selectedListeners.length === 0) {
        setListenerError("At least one compatible listener must be selected.");
        return;
      }

      setIsSubmitting(true);
      setFormError(null);
      setTargetNameError(null);

      const targetPayload: Target = {
        ...(formData as Target),
        name: targetName,
        listeners: selectedListeners,
      };

      try {
        let successMessage = "";
        if (editingTarget) {
          await updateTarget(targetPayload);
          successMessage = `Target '${targetPayload.name}' updated.`;
        } else {
          if (selectedTargetType === "a2a") {
            await createA2aTarget(targetPayload);
          } else {
            await createMcpTarget(targetPayload);
          }
          successMessage = `Target '${targetPayload.name}' created.`;
        }

        await refreshTargets();
        handleDialogClose();
        toast.success(successMessage);
      } catch (err) {
        console.error(`Error ${editingTarget ? "updating" : "creating"} target:`, err);
        const message =
          err instanceof Error
            ? err.message
            : `Failed to ${editingTarget ? "update" : "create"} target.`;
        setFormError(message);
        toast.error(`Error ${editingTarget ? "updating" : "creating"} target`, {
          description: message,
        });
      } finally {
        setIsSubmitting(false);
      }
    };

    const openAddTargetDialog = () => {
      resetFormState();
      setIsDialogOpen(true);
    };

    const openEditTargetDialog = (target: TargetWithType) => {
      resetFormState();
      setEditingTarget(target);
      setSelectedTargetType(target.type === "a2a" ? "a2a" : "mcp");

      setTargetName(target.name);
      setSelectedListeners(target.listeners);
      setIsDialogOpen(true);
    };

    const handleDialogClose = () => {
      setIsDialogOpen(false);
      resetFormState();
    };

    useImperativeHandle(ref, () => ({
      openAddTargetDialog,
    }));

    const renderFormFields = () => {
      if (!selectedTargetType) return null;

      switch (selectedTargetType) {
        case "a2a":
          return (
            <A2ATargetForm
              targetName={targetName}
              onSubmit={handleFormSubmit}
              isLoading={isSubmitting}
              existingTarget={editingTarget?.type === "a2a" ? editingTarget : undefined}
              hideSubmitButton={true}
              ref={a2aFormRef}
            />
          );
        case "mcp":
          return (
            <MCPTargetForm
              targetName={targetName}
              onTargetNameChange={setTargetName}
              onSubmit={handleFormSubmit}
              isLoading={isSubmitting}
              existingTarget={editingTarget?.type !== "a2a" ? editingTarget : undefined}
              ref={mcpFormRef}
            />
          );
        default:
          return <p>Invalid target type selected.</p>;
      }
    };

    const showListenerSelection = selectedTargetType && !isLoadingListeners;
    const noListenersFound = showListenerSelection && compatibleListeners.length === 0;
    const canProceed = selectedTargetType && !isLoadingListeners && !listenerError;

    return (
      <div>
        {formError && (
          <Alert variant="destructive">
            <AlertDescription>{formError}</AlertDescription>
          </Alert>
        )}

        {config?.targets && config.targets.length > 0 ? (
          <div className="mt-6">
            <h3 className="font-medium mb-2">Configured Targets</h3>
            <div className="space-y-2">
              {config.targets.map((target, index) => (
                <div
                  key={target.name || index}
                  className="flex items-center justify-between p-3 border rounded-md bg-background hover:bg-muted/50"
                >
                  <TargetItem
                    target={target}
                    index={index}
                    onDelete={handleRemoveTarget}
                    isUpdating={isSubmitting}
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => openEditTargetDialog(target)}
                    className="h-8 w-8 ml-2 text-muted-foreground hover:text-primary flex-shrink-0"
                    disabled={isSubmitting}
                  >
                    <Edit2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="mt-6 text-center text-muted-foreground">
            <Info className="inline-block h-5 w-5 mr-2" />
            No targets configured yet.
          </div>
        )}

        <Dialog open={isDialogOpen} onOpenChange={(open) => !open && handleDialogClose()}>
          <DialogContent className="sm:max-w-[600px]">
            <DialogHeader>
              <DialogTitle>{editingTarget ? "Edit Target" : "Add New Target"}</DialogTitle>
              <DialogDescription>
                Configure the details for your target server. Choose MCP or A2A.
              </DialogDescription>
            </DialogHeader>

            <div className="grid gap-4 py-4">
              <div className="space-y-2">
                <Label>Target Type *</Label>
                <RadioGroup
                  value={selectedTargetType ?? ""}
                  onValueChange={(value) => setSelectedTargetType(value as SelectableTargetType)}
                  className="grid grid-cols-2 gap-4"
                  disabled={!!editingTarget || isLoadingListeners}
                >
                  <Label
                    htmlFor="mcp-target-type"
                    className={cn(
                      "flex flex-col items-center justify-between rounded-md border-2 border-muted bg-popover p-4 hover:bg-accent/20 hover:text-accent-foreground",
                      selectedTargetType === "mcp" && "border-primary"
                    )}
                  >
                    <RadioGroupItem value="mcp" id="mcp-target-type" className="sr-only" />
                    MCP Target
                    <span className="block text-xs font-normal text-muted-foreground mt-1 text-center">
                      For SSE, Stdio, OpenAPI backends
                    </span>
                  </Label>
                  <Label
                    htmlFor="a2a-target-type"
                    className={cn(
                      "flex flex-col items-center justify-between rounded-md border-2 border-muted bg-popover p-4 hover:bg-accent/20 hover:text-accent-foreground",
                      selectedTargetType === "a2a" && "border-primary"
                    )}
                  >
                    <RadioGroupItem value="a2a" id="a2a-target-type" className="sr-only" />
                    A2A Target
                    <span className="block text-xs font-normal text-muted-foreground mt-1 text-center">
                      For Agent-to-Agent protocol
                    </span>
                  </Label>
                </RadioGroup>
              </div>

              {isLoadingListeners && selectedTargetType && (
                <div className="flex items-center text-sm text-muted-foreground">
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" /> Loading compatible listeners...
                </div>
              )}

              {showListenerSelection && (
                <div className="space-y-2">
                  <Label htmlFor="target-listeners">Associate with Listener(s) *</Label>
                  {noListenersFound && listenerError ? (
                    <Alert variant="default">
                      <AlertCircle className="h-4 w-4" />
                      <AlertTitle>No Compatible Listeners Found</AlertTitle>
                      <AlertDescription>{listenerError}</AlertDescription>
                    </Alert>
                  ) : (
                    <>
                      <ListenerSelect
                        selectedListeners={selectedListeners}
                        onListenersChange={setSelectedListeners}
                      />
                      <p className="text-xs text-muted-foreground">
                        Select the listener(s) that can route requests to this target.
                      </p>
                      {listenerError && !noListenersFound && (
                        <p className="text-xs text-destructive">{listenerError}</p>
                      )}
                    </>
                  )}
                </div>
              )}

              {canProceed && (
                <div className="space-y-2">
                  <Label htmlFor="targetName">Target Name *</Label>
                  <Input
                    id="targetName"
                    placeholder="Enter unique target name"
                    value={targetName}
                    onChange={(e) => {
                      setTargetName(e.target.value);
                      if (targetNameError) setTargetNameError(null);
                    }}
                    disabled={!!editingTarget || isSubmitting}
                    required
                  />
                  {targetNameError && <p className="text-xs text-destructive">{targetNameError}</p>}
                  <p className="text-xs text-muted-foreground">
                    A unique identifier for this target.
                  </p>
                </div>
              )}

              {canProceed && renderFormFields()}

              {formError && (
                <Alert variant="destructive">
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription>{formError}</AlertDescription>
                </Alert>
              )}
            </div>

            <DialogFooter>
              <DialogClose asChild>
                <Button variant="outline">Cancel</Button>
              </DialogClose>
              <Button
                type="button"
                onClick={() => {
                  const formRef = selectedTargetType === "a2a" ? a2aFormRef : mcpFormRef;
                  if (formRef.current?.submitForm) {
                    formRef.current.submitForm();
                  } else {
                    console.error("Form ref not available or submitForm missing");
                    setFormError("Could not submit form. Ref is missing.");
                  }
                }}
                disabled={
                  !canProceed ||
                  noListenersFound ||
                  isSubmitting ||
                  !targetName.trim() ||
                  !selectedListeners?.length
                }
              >
                {isSubmitting ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                {editingTarget ? "Save Changes" : "Add Target"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <Dialog open={deleteConfirmOpen} onOpenChange={setDeleteConfirmOpen}>
          <DialogContent>
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
                onClick={() => setDeleteConfirmOpen(false)}
                disabled={isSubmitting}
              >
                Cancel
              </Button>
              <Button variant="destructive" onClick={confirmDelete} disabled={isSubmitting}>
                {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
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
