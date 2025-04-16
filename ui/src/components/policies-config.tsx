"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Trash2, PlusCircle, Loader2 } from "lucide-react";
import { RBACConfig } from "@/lib/types";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";

interface PoliciesConfigProps {
  policies: RBACConfig[];
  addPolicy: (policy: RBACConfig) => void;
  removePolicy: (index: number) => void;
  serverAddress?: string;
  serverPort?: number;
  onConfigUpdate?: (success: boolean, message: string) => void;
}

export function PoliciesConfig({
  policies,
  addPolicy,
  removePolicy,
  serverAddress,
  serverPort,
  onConfigUpdate,
}: PoliciesConfigProps) {
  const [isAddingPolicy, setIsAddingPolicy] = useState(false);
  const [policyName, setPolicyName] = useState("");
  const [policyToDelete, setPolicyToDelete] = useState<number | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);

  const updatePoliciesOnServer = async (updatedPolicies: RBACConfig[]) => {
    if (!serverAddress || !serverPort) return false;

    setIsUpdating(true);
    try {
      const response = await fetch(`http://${serverAddress}:${serverPort}/rbac`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(updatedPolicies),
      });

      if (!response.ok) {
        throw new Error(`Failed to update policies: ${response.status} ${response.statusText}`);
      }

      if (onConfigUpdate) {
        onConfigUpdate(true, "Policies updated successfully");
      }
      return true;
    } catch (error) {
      console.error("Error updating policies:", error);
      if (onConfigUpdate) {
        onConfigUpdate(false, error instanceof Error ? error.message : "Failed to update policies");
      }
      return false;
    } finally {
      setIsUpdating(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    const policy: RBACConfig = {
      name: policyName,
      namespace: "default",
      rules: [],
    };

    // Add policy to local state
    addPolicy(policy);

    // Update policies on server
    if (serverAddress && serverPort) {
      const updatedPolicies = [...policies, policy];
      await updatePoliciesOnServer(updatedPolicies);
    }

    resetForm();
    setIsAddingPolicy(false);
  };

  const resetForm = () => {
    setPolicyName("");
  };

  const handleDeletePolicy = (index: number) => {
    setPolicyToDelete(index);
  };

  const confirmDelete = async () => {
    if (policyToDelete !== null) {
      // Remove policy from local state
      removePolicy(policyToDelete);

      // Update policies on server
      if (serverAddress && serverPort) {
        const updatedPolicies = policies.filter((_, i) => i !== policyToDelete);
        await updatePoliciesOnServer(updatedPolicies);
      }

      setPolicyToDelete(null);
    }
  };

  const cancelDelete = () => {
    setPolicyToDelete(null);
  };

  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h3 className="text-lg font-medium mb-2">Security Policies</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Configure security policies for the proxy
        </p>
      </div>

      {isUpdating && (
        <Alert>
          <AlertDescription className="flex items-center">
            <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            Updating policies on server...
          </AlertDescription>
        </Alert>
      )}

      {policies.length === 0 && !isAddingPolicy ? (
        <Alert>
          <AlertDescription>
            No security policies configured. Add a policy to get started.
          </AlertDescription>
        </Alert>
      ) : (
        <div className="space-y-4">
          {policies.map((policy, index) => (
            <div
              key={index}
              id={`policy-${index}`}
              className="border rounded-lg p-4 flex justify-between items-start"
            >
              <div>
                <h4 className="font-medium">{policy.name}</h4>
                <div className="flex items-center mt-1">
                  <Badge variant="outline" className="mr-2">
                    {policy.rules.length} rules
                  </Badge>
                </div>
              </div>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => handleDeletePolicy(index)}
                className="text-muted-foreground hover:text-destructive"
                disabled={isUpdating}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ))}
        </div>
      )}

      <Button
        onClick={() => setIsAddingPolicy(true)}
        className="flex items-center"
        disabled={isUpdating}
      >
        <PlusCircle className="h-4 w-4 mr-2" />
        Add Policy
      </Button>

      <Dialog open={isAddingPolicy} onOpenChange={setIsAddingPolicy}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Security Policy</DialogTitle>
            <DialogDescription>Configure a new security policy for the proxy.</DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSubmit} className="space-y-4 mt-6">
            <div className="space-y-2">
              <Label htmlFor="name">Policy Name</Label>
              <Input
                id="name"
                value={policyName}
                onChange={e => setPolicyName(e.target.value)}
                placeholder="Enter policy name"
                required
                disabled={isUpdating}
              />
            </div>

            <div className="flex justify-end">
              <Button type="submit" disabled={isUpdating}>
                {isUpdating ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    Adding...
                  </>
                ) : (
                  "Add Policy"
                )}
              </Button>
            </div>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={policyToDelete !== null} onOpenChange={open => !open && cancelDelete()}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Security Policy</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this security policy? This action cannot be undone.
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
