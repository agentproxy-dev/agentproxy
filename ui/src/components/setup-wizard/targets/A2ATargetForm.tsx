import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Target } from "@/lib/types";

interface A2ATargetFormProps {
  targetName: string;
  onSubmit: (target: Target) => Promise<void>;
  isLoading: boolean;
  existingTarget?: Target;
}

export function A2ATargetForm({
  targetName,
  onSubmit,
  isLoading,
  existingTarget,
}: A2ATargetFormProps) {
  const [targetHost, setTargetHost] = useState("");
  const [targetPort, setTargetPort] = useState("");
  const [targetPath, setTargetPath] = useState("/");

  // Initialize form with existing target data if provided
  useEffect(() => {
    if (existingTarget?.a2a) {
      setTargetHost(existingTarget.a2a.host);
      setTargetPort(existingTarget.a2a.port.toString());
      setTargetPath(existingTarget.a2a.path || "/");
    }
  }, [existingTarget]);

  const handleSubmit = async () => {
    try {
      if (!targetHost || !targetPort) {
        throw new Error("Host and port are required for A2A targets");
      }

      const port = parseInt(targetPort, 10);
      if (isNaN(port)) {
        throw new Error("Port must be a valid number");
      }

      const target: Target = {
        name: targetName,
        a2a: {
          host: targetHost,
          port: port,
          path: targetPath || "/",
        },
      };

      await onSubmit(target);
    } catch (err) {
      console.error("Error creating A2A target:", err);
      throw err;
    }
  };

  return (
    <div className="space-y-4 pt-4">
      <div className="space-y-2">
        <Label htmlFor="targetHost">Host</Label>
        <Input
          id="targetHost"
          value={targetHost}
          onChange={(e) => setTargetHost(e.target.value)}
          placeholder="e.g., localhost"
          required
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="targetPort">Port</Label>
        <Input
          id="targetPort"
          value={targetPort}
          onChange={(e) => setTargetPort(e.target.value)}
          placeholder="e.g., 8080"
          required
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="targetPath">Path</Label>
        <Input
          id="targetPath"
          value={targetPath}
          onChange={(e) => setTargetPath(e.target.value)}
          placeholder="/"
        />
        <p className="text-sm text-muted-foreground">
          The path where the A2A service is exposed (defaults to /)
        </p>
      </div>

      <Button
        onClick={handleSubmit}
        className="w-full"
        disabled={isLoading || !targetHost || !targetPort}
      >
        {isLoading
          ? existingTarget
            ? "Updating Target..."
            : "Adding Target..."
          : existingTarget
            ? "Update A2A Target"
            : "Add A2A Target"}
      </Button>
    </div>
  );
}
