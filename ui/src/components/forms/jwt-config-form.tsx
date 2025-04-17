"use client";

import { useState } from "react";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Listener } from "@/lib/types";

interface JWTConfigFormProps {
  listener: Listener | null;
  onSave: (updatedListener: Listener) => void;
  onCancel: () => void;
}

export function JWTConfigForm({ listener, onSave, onCancel }: JWTConfigFormProps) {
  const [config, setConfig] = useState({
    issuer: listener?.sse?.authn?.jwt?.issuer?.join(",") || "",
    audience: listener?.sse?.authn?.jwt?.audience?.join(",") || "",
    localJwksPath: listener?.sse?.authn?.jwt?.localJwks?.file_path || "",
    remoteJwksUrl: listener?.sse?.authn?.jwt?.remoteJwks?.url || "",
    jwksSource: listener?.sse?.authn?.jwt?.localJwks ? "local" : "remote",
  });

  const handleSave = () => {
    if (!listener) return;

    const updatedListener: Listener = {
      ...listener,
      sse: {
        ...listener.sse,
        authn: {
          jwt: {
            issuer: config.issuer
              .split(",")
              .map((s) => s.trim())
              .filter(Boolean),
            audience: config.audience
              .split(",")
              .map((s) => s.trim())
              .filter(Boolean),
            ...(config.jwksSource === "local" && config.localJwksPath
              ? {
                  local_jwks: {
                    file_path: config.localJwksPath,
                  },
                }
              : {}),
            ...(config.jwksSource === "remote" && config.remoteJwksUrl
              ? {
                  remote_jwks: {
                    url: config.remoteJwksUrl,
                  },
                }
              : {}),
          },
        },
      },
    };

    onSave(updatedListener);
  };

  return (
    <div className="space-y-4 py-4">
      <div className="space-y-2">
        <Label htmlFor="jwt-issuer">JWT Issuer</Label>
        <Input
          id="jwt-issuer"
          value={config.issuer}
          onChange={(e) => setConfig({ ...config, issuer: e.target.value })}
          placeholder="Enter comma-separated issuers"
        />
        <p className="text-xs text-muted-foreground">
          Comma-separated list of allowed JWT issuers.
        </p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="jwt-audience">JWT Audience</Label>
        <Input
          id="jwt-audience"
          value={config.audience}
          onChange={(e) => setConfig({ ...config, audience: e.target.value })}
          placeholder="Enter comma-separated audiences"
        />
        <p className="text-xs text-muted-foreground">
          Comma-separated list of allowed JWT audiences.
        </p>
      </div>

      <div className="space-y-4">
        <Label>JWKS Source</Label>
        <RadioGroup
          value={config.jwksSource}
          onValueChange={(value) =>
            setConfig({
              ...config,
              jwksSource: value as "local" | "remote",
              ...(value === "local" ? { remoteJwksUrl: "" } : { localJwksPath: "" }),
            })
          }
        >
          <div className="space-y-6">
            <div className="flex items-start space-x-4">
              <RadioGroupItem value="local" id="jwks-local" />
              <div className="space-y-2 flex-1">
                <Label htmlFor="jwks-file">Local JWKS File Path</Label>
                <Input
                  id="jwks-file"
                  value={config.localJwksPath}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      localJwksPath: e.target.value,
                    })
                  }
                  placeholder="/path/to/jwks.json"
                  disabled={config.jwksSource !== "local"}
                />
                <p className="text-xs text-muted-foreground mt-1">Path to a local JWKS file.</p>
              </div>
            </div>

            <div className="flex items-start space-x-4">
              <RadioGroupItem value="remote" id="jwks-remote" />
              <div className="space-y-2 flex-1">
                <Label htmlFor="jwks-url">Remote JWKS URL</Label>
                <Input
                  id="jwks-url"
                  value={config.remoteJwksUrl}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      remoteJwksUrl: e.target.value,
                    })
                  }
                  placeholder="https://example.com/.well-known/jwks.json"
                  disabled={config.jwksSource !== "remote"}
                />
                <p className="text-xs text-muted-foreground mt-1">URL to a remote JWKS endpoint.</p>
              </div>
            </div>
          </div>
        </RadioGroup>
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
