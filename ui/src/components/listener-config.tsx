"use client";

import { useState, useEffect } from "react";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Listener } from "@/lib/types";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, Trash2, Shield, Lock, Key, Settings2, MoreVertical } from "lucide-react";
import { fetchListeners, addListener, deleteListener, fetchListenerTargets } from "@/lib/api";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { JWTConfigForm, TLSConfigForm, RBACConfigForm } from "@/components/forms";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

interface ListenerConfigProps {
  serverAddress?: string;
  serverPort?: number;
  isAddingListener?: boolean;
  setIsAddingListener?: (isAdding: boolean) => void;
}

interface NewListenerState {
  name: string;
  address: string;
  port: string;
  type: "sse";
}

interface ConfigDialogState {
  type: "jwt" | "tls" | "rbac" | null;
  isOpen: boolean;
  listener: Listener | null;
  listenerIndex: number;
}

interface DeleteDialogState {
  isOpen: boolean;
  listenerIndex: number;
}

interface ListenerWithTargets extends Listener {
  targetCount?: number;
}

export function ListenerConfig({
  serverAddress,
  serverPort,
  isAddingListener = false,
  setIsAddingListener = () => {},
}: ListenerConfigProps) {
  const [listeners, setListeners] = useState<ListenerWithTargets[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [configDialog, setConfigDialog] = useState<ConfigDialogState>({
    type: null,
    isOpen: false,
    listener: null,
    listenerIndex: -1,
  });
  const [newListener, setNewListener] = useState<NewListenerState>({
    name: "",
    address: "0.0.0.0",
    port: "5555",
    type: "sse",
  });
  const [deleteDialog, setDeleteDialog] = useState<DeleteDialogState>({
    isOpen: false,
    listenerIndex: -1,
  });

  // Fetch listener configuration and target counts
  useEffect(() => {
    const fetchListenerConfig = async () => {
      if (!serverAddress || !serverPort) {
        setIsLoading(false);
        return;
      }

      setIsLoading(true);
      setError(null);

      try {
        const fetchedListeners = await fetchListeners();
        const listenersArray = Array.isArray(fetchedListeners)
          ? fetchedListeners
          : [fetchedListeners];

        // Fetch target counts for each listener
        const listenersWithTargets = await Promise.all(
          listenersArray.map(async (listener) => {
            try {
              const targets = await fetchListenerTargets(listener.name);
              return {
                ...listener,
                targetCount: targets.length,
              };
            } catch (err) {
              console.error(`Error fetching targets for listener ${listener.name}:`, err);
              return {
                ...listener,
                targetCount: 0,
              };
            }
          })
        );

        setListeners(listenersWithTargets);
      } catch (err) {
        console.error("Error fetching listener configuration:", err);
        setError(err instanceof Error ? err.message : "Failed to fetch listener configuration");
      } finally {
        setIsLoading(false);
      }
    };

    fetchListenerConfig();
  }, [serverAddress, serverPort]);

  const handleAddListener = async () => {
    if (!serverAddress || !serverPort) return;

    setIsLoading(true);
    setError(null);

    try {
      const listenerToAdd: Listener = {
        name: newListener.name || `listener-${listeners.length + 1}`,
        sse: {
          address: newListener.address,
          port: parseInt(newListener.port),
        },
      };

      await addListener(listenerToAdd);

      // Refresh the listeners list
      const updatedListeners = await fetchListeners();
      const listenersArray = Array.isArray(updatedListeners)
        ? updatedListeners
        : [updatedListeners];
      setListeners(listenersArray);

      // Reset the form
      setNewListener({
        name: "",
        address: "0.0.0.0",
        port: "5555",
        type: "sse",
      });

      setIsAddingListener(false);
    } catch (err) {
      console.error("Error adding listener:", err);
      setError(err instanceof Error ? err.message : "Failed to add listener");
    } finally {
      setIsLoading(false);
    }
  };

  const handleUpdateListener = async (updatedListener: Listener) => {
    if (!serverAddress || !serverPort) return;

    setIsLoading(true);
    setError(null);

    try {
      await addListener(updatedListener);

      // Refresh the listeners list
      const updatedListeners = await fetchListeners();
      const listenersArray = Array.isArray(updatedListeners)
        ? updatedListeners
        : [updatedListeners];
      setListeners(listenersArray);

      setConfigDialog({
        type: null,
        isOpen: false,
        listener: null,
        listenerIndex: -1,
      });
    } catch (err) {
      console.error("Error updating listener:", err);
      setError(err instanceof Error ? err.message : "Failed to update listener");
    } finally {
      setIsLoading(false);
    }
  };

  const handleDeleteListener = async (index: number) => {
    if (!serverAddress || !serverPort) return;

    setIsLoading(true);
    setError(null);

    try {
      const listenerToDelete = listeners[index];
      // Extract the listener name or use a default if not available
      const listenerName = listenerToDelete.name || `listener-${index}`;

      // Create a copy of the listener with the name property
      const listenerWithName = {
        ...listenerToDelete,
        name: listenerName,
      };

      await deleteListener(listenerWithName);

      // Refresh the listeners list
      const updatedListeners = await fetchListeners();
      const listenersArray = Array.isArray(updatedListeners)
        ? updatedListeners
        : [updatedListeners];
      setListeners(listenersArray);

      // Close the delete dialog
      setDeleteDialog({ isOpen: false, listenerIndex: -1 });
    } catch (err) {
      console.error("Error deleting listener:", err);
      setError(err instanceof Error ? err.message : "Failed to delete listener");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div>
      {error && (
        <Alert variant="destructive" className="mb-6">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {isLoading ? (
        <div className="flex items-center justify-center py-8">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
          <span className="ml-2">Loading listener configuration...</span>
        </div>
      ) : listeners.length === 0 ? (
        <div className="text-center py-12 border rounded-md bg-muted/20">
          <p className="text-muted-foreground">
            No listeners configured. Add a listener to get started.
          </p>
        </div>
      ) : (
        <div className="border rounded-md overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow className="bg-muted/50">
                <TableHead>Name</TableHead>
                <TableHead>Address</TableHead>
                <TableHead>Targets</TableHead>
                <TableHead>Authentication</TableHead>
                <TableHead>TLS</TableHead>
                <TableHead>Policies</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {listeners.map((listener, index) => (
                <TableRow key={index} className="hover:bg-muted/30">
                  <TableCell className="font-medium">
                    {listener.name || `listener-${index + 1}`}
                  </TableCell>
                  <TableCell>
                    {listener.sse?.address}:{listener.sse?.port}
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline">
                      {listener.targetCount ?? 0} target
                      {(listener.targetCount ?? 0) !== 1 ? "s" : ""}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center space-x-2">
                      <Badge
                        variant={listener.sse?.authn ? "default" : "outline"}
                        className="h-7 space-x-2"
                      >
                        <Shield className="h-4 w-4" />
                        <span>JWT</span>
                      </Badge>
                      <DropdownMenu>
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <DropdownMenuTrigger asChild>
                                <Button variant="ghost" size="icon">
                                  <MoreVertical className="h-4 w-4" />
                                </Button>
                              </DropdownMenuTrigger>
                            </TooltipTrigger>
                            <TooltipContent side="top">
                              <p className="text-xs">Manage JWT Authentication</p>
                            </TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem
                            onClick={() =>
                              setConfigDialog({
                                type: "jwt",
                                isOpen: true,
                                listener,
                                listenerIndex: index,
                              })
                            }
                          >
                            <Settings2 className="h-4 w-4 mr-2" />
                            Configure
                          </DropdownMenuItem>
                          {listener.sse?.authn && (
                            <DropdownMenuItem
                              className="text-destructive"
                              onClick={() => {
                                const updatedListener = {
                                  ...listener,
                                  sse: {
                                    ...listener.sse,
                                    authn: undefined,
                                    rbac: undefined, // Remove RBAC when removing auth
                                  },
                                };
                                handleUpdateListener(updatedListener);
                              }}
                            >
                              <Trash2 className="h-4 w-4 mr-2" />
                              Delete
                            </DropdownMenuItem>
                          )}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center space-x-2">
                      <Badge
                        variant={listener.sse?.tls ? "default" : "outline"}
                        className="h-7 space-x-2"
                      >
                        <Lock className="h-4 w-4" />
                        <span>TLS</span>
                      </Badge>
                      <DropdownMenu>
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <DropdownMenuTrigger asChild>
                                <Button variant="ghost" size="icon">
                                  <MoreVertical className="h-4 w-4" />
                                </Button>
                              </DropdownMenuTrigger>
                            </TooltipTrigger>
                            <TooltipContent side="top">
                              <p className="text-xs">Manage TLS Encryption</p>
                            </TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem
                            onClick={() =>
                              setConfigDialog({
                                type: "tls",
                                isOpen: true,
                                listener,
                                listenerIndex: index,
                              })
                            }
                          >
                            <Settings2 className="h-4 w-4 mr-2" />
                            Configure
                          </DropdownMenuItem>
                          {listener.sse?.tls && (
                            <DropdownMenuItem
                              className="text-destructive"
                              onClick={() => {
                                const updatedListener = {
                                  ...listener,
                                  sse: {
                                    ...listener.sse,
                                    tls: undefined,
                                  },
                                };
                                handleUpdateListener(updatedListener);
                              }}
                            >
                              <Trash2 className="h-4 w-4 mr-2" />
                              Delete
                            </DropdownMenuItem>
                          )}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center space-x-2">
                      <Badge
                        variant={
                          listener.sse?.rbac && listener.sse.rbac.length > 0 ? "default" : "outline"
                        }
                        className="h-7 space-x-2"
                      >
                        <Key className="h-4 w-4" />
                        <span>Policy</span>
                      </Badge>
                      <DropdownMenu>
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <div>
                                <DropdownMenuTrigger asChild>
                                  <Button
                                    variant="ghost"
                                    size="icon"
                                    disabled={!listener.sse?.authn}
                                  >
                                    <MoreVertical className="h-4 w-4" />
                                  </Button>
                                </DropdownMenuTrigger>
                              </div>
                            </TooltipTrigger>
                            <TooltipContent side="top">
                              <p className="text-xs">
                                {!listener.sse?.authn
                                  ? "Enable JWT Authentication first to configure RBAC"
                                  : "Manage RBAC Policies"}
                              </p>
                            </TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem
                            onClick={() =>
                              setConfigDialog({
                                type: "rbac",
                                isOpen: true,
                                listener,
                                listenerIndex: index,
                              })
                            }
                          >
                            <Settings2 className="h-4 w-4 mr-2" />
                            Configure
                          </DropdownMenuItem>
                          {listener.sse?.rbac && listener.sse.rbac.length > 0 && (
                            <DropdownMenuItem
                              className="text-destructive"
                              onClick={() => {
                                const updatedListener = {
                                  ...listener,
                                  sse: {
                                    ...listener.sse,
                                    rbac: undefined,
                                  },
                                };
                                handleUpdateListener(updatedListener);
                              }}
                            >
                              <Trash2 className="h-4 w-4 mr-2" />
                              Delete
                            </DropdownMenuItem>
                          )}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => setDeleteDialog({ isOpen: true, listenerIndex: index })}
                      className="text-destructive hover:text-destructive"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      {/* Add New Listener Dialog */}
      <Dialog open={isAddingListener} onOpenChange={setIsAddingListener}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add New Listener</DialogTitle>
            <DialogDescription>
              Configure a new SSE listener for the proxy server. Additional features can be
              configured after creation.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                value={newListener.name}
                onChange={(e) => setNewListener({ ...newListener, name: e.target.value })}
                placeholder="e.g., default"
              />
              <p className="text-xs text-muted-foreground">
                A unique name for this listener. If left empty, a default name will be generated.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="address">Address</Label>
              <Input
                id="address"
                value={newListener.address}
                onChange={(e) => setNewListener({ ...newListener, address: e.target.value })}
                placeholder="0.0.0.0"
              />
              <p className="text-xs text-muted-foreground">
                The IP address the listener will bind to. 0.0.0.0 means it will listen on all
                interfaces.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="port">Port</Label>
              <Input
                id="port"
                value={newListener.port}
                onChange={(e) => setNewListener({ ...newListener, port: e.target.value })}
                placeholder="5555"
              />
              <p className="text-xs text-muted-foreground">The port number for the listener.</p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setIsAddingListener(false)}>
              Cancel
            </Button>
            <Button onClick={handleAddListener}>Add Listener</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* JWT Configuration Dialog */}
      <Dialog
        open={configDialog.type === "jwt" && configDialog.isOpen}
        onOpenChange={(open) => !open && setConfigDialog({ ...configDialog, isOpen: false })}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Configure JWT Authentication</DialogTitle>
            <DialogDescription>Set up JWT authentication for the listener.</DialogDescription>
          </DialogHeader>
          <JWTConfigForm
            listener={configDialog.listener}
            onSave={handleUpdateListener}
            onCancel={() => setConfigDialog({ ...configDialog, isOpen: false })}
          />
        </DialogContent>
      </Dialog>

      {/* TLS Configuration Dialog */}
      <Dialog
        open={configDialog.type === "tls" && configDialog.isOpen}
        onOpenChange={(open) => !open && setConfigDialog({ ...configDialog, isOpen: false })}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Configure TLS</DialogTitle>
            <DialogDescription>Set up TLS encryption for the listener.</DialogDescription>
          </DialogHeader>
          <TLSConfigForm
            listener={configDialog.listener}
            onSave={handleUpdateListener}
            onCancel={() => setConfigDialog({ ...configDialog, isOpen: false })}
          />
        </DialogContent>
      </Dialog>

      {/* RBAC Configuration Dialog */}
      <Dialog
        open={configDialog.type === "rbac" && configDialog.isOpen}
        onOpenChange={(open) => !open && setConfigDialog({ ...configDialog, isOpen: false })}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Configure RBAC</DialogTitle>
            <DialogDescription>Set up role-based access control policies.</DialogDescription>
          </DialogHeader>
          <RBACConfigForm
            listener={configDialog.listener}
            onSave={handleUpdateListener}
            onCancel={() => setConfigDialog({ ...configDialog, isOpen: false })}
          />
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog
        open={deleteDialog.isOpen}
        onOpenChange={(open) => !open && setDeleteDialog({ ...deleteDialog, isOpen: false })}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Listener</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this listener? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDeleteDialog({ isOpen: false, listenerIndex: -1 })}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => handleDeleteListener(deleteDialog.listenerIndex)}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
