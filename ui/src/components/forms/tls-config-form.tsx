"use client";

import { useState } from "react";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Listener } from "@/lib/types";

interface TLSConfigFormProps {
  listener: Listener | null;
  onSave: (updatedListener: Listener) => void;
  onCancel: () => void;
}

export function TLSConfigForm({ listener, onSave, onCancel }: TLSConfigFormProps) {
  const [config, setConfig] = useState({
    certFile: listener?.sse?.tls?.cert_pem?.file_path || "",
    keyFile: listener?.sse?.tls?.key_pem?.file_path || "",
  });

  const handleSave = () => {
    if (!listener) return;

    const updatedListener: Listener = {
      ...listener,
      sse: {
        ...listener.sse,
        tls: {
          cert_pem: {
            file_path: config.certFile,
          },
          key_pem: {
            file_path: config.keyFile,
          },
        },
      },
    };

    onSave(updatedListener);
  };

  return (
    <div className="space-y-4 py-4">
      <div className="space-y-2">
        <Label htmlFor="cert-file">Certificate File Path</Label>
        <Input
          id="cert-file"
          value={config.certFile}
          onChange={(e) => setConfig({ ...config, certFile: e.target.value })}
          placeholder="/path/to/cert.pem"
        />
        <p className="text-xs text-muted-foreground">Path to the TLS certificate file.</p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="key-file">Key File Path</Label>
        <Input
          id="key-file"
          value={config.keyFile}
          onChange={(e) => setConfig({ ...config, keyFile: e.target.value })}
          placeholder="/path/to/key.pem"
        />
        <p className="text-xs text-muted-foreground">Path to the TLS private key file.</p>
      </div>

      <div className="flex justify-end space-x-2 pt-4">
        <Button variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button onClick={handleSave}>Save Changes</Button>
      </div>
    </div>
  );
}
