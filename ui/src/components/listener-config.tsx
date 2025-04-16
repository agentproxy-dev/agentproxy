"use client";

import { useState, useEffect } from "react";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Listener } from "@/lib/types";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, Trash2 } from "lucide-react";
import { fetchListeners, addListener, deleteListener } from "@/lib/api";
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

interface ListenerConfigProps {
  serverAddress?: string;
  serverPort?: number;
  isAddingListener?: boolean;
  setIsAddingListener?: (isAdding: boolean) => void;
}

export function ListenerConfig({
  serverAddress,
  serverPort,
  isAddingListener = false,
  setIsAddingListener = () => {},
}: ListenerConfigProps) {
  const [listeners, setListeners] = useState<Listener[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [newListener, setNewListener] = useState({
    name: "",
    address: "0.0.0.0",
    port: "5555",
    type: "sse",
  });

  // Fetch listener configuration from the proxy API
  useEffect(() => {
    const fetchListenerConfig = async () => {
      if (!serverAddress || !serverPort) {
        setIsLoading(false);
        return;
      }

      setIsLoading(true);
      setError(null);

      try {
        const fetchedListeners = await fetchListeners(serverAddress, serverPort);
        console.log("ListenerConfig received data:", fetchedListeners);

        // Ensure we have an array of listeners
        const listenersArray = Array.isArray(fetchedListeners)
          ? fetchedListeners
          : [fetchedListeners];
        setListeners(listenersArray);
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

      await addListener(serverAddress, serverPort, listenerToAdd);

      // Refresh the listeners list
      const updatedListeners = await fetchListeners(serverAddress, serverPort);
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

      await deleteListener(serverAddress, serverPort, listenerWithName);

      // Refresh the listeners list
      const updatedListeners = await fetchListeners(serverAddress, serverPort);
      const listenersArray = Array.isArray(updatedListeners)
        ? updatedListeners
        : [updatedListeners];
      setListeners(listenersArray);
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
                <TableHead>Type</TableHead>
                <TableHead>Address</TableHead>
                <TableHead>Port</TableHead>
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
                    <Badge variant="outline">SSE</Badge>
                  </TableCell>
                  <TableCell>{listener.sse?.address || listener.sse?.host || "0.0.0.0"}</TableCell>
                  <TableCell>{listener.sse?.port || "5555"}</TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleDeleteListener(index)}
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

      <Dialog open={isAddingListener} onOpenChange={setIsAddingListener}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add New Listener</DialogTitle>
            <DialogDescription>
              Configure a new SSE listener for the proxy server.
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
    </div>
  );
}
