"use client";

import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Target } from "@/lib/types";

interface ServerListProps {
  targets: Target[];
  removeTarget: (index: number) => void;
}

export function ServerList({ targets, removeTarget }: ServerListProps) {
  if (targets.length === 0) {
    return (
      <Alert>
        <AlertDescription>
          No target servers configured. Add a server to get started.
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="space-y-4">
      {targets.map((target, index) => (
        <Card key={index} id={`target-${index}`} className="relative">
          <CardContent className="p-4">
            <div className="flex justify-between items-start">
              <div>
                <h3 className="font-medium text-lg">{target.name}</h3>
                <div className="flex items-center mt-1">
                  <Badge variant="outline" className="mr-2">
                    {getTargetType(target)}
                  </Badge>
                  {renderTargetDetails(target)}
                </div>
              </div>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => removeTarget(index)}
                className="text-muted-foreground hover:text-destructive"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function getTargetType(target: Target) {
  if (target.stdio) return "stdio";
  if (target.sse) return "sse";
  if (target.openapi) return "openapi";
  return "unknown";
}

function renderTargetDetails(target: Target) {
  if (target.stdio) {
    return (
      <div className="text-sm text-muted-foreground">
        <p>
          Command: {target.stdio.cmd} {target.stdio.args?.join(" ")}
        </p>
      </div>
    );
  }

  if (target.sse) {
    return (
      <div className="text-sm text-muted-foreground">
        <p>
          Host: {target.sse.host}:{target.sse.port}
        </p>
        <p>Path: {target.sse.path || "/"}</p>
      </div>
    );
  }

  if (target.openapi) {
    return (
      <div className="text-sm text-muted-foreground">
        <p>
          Host: {target.openapi.host}:{target.openapi.port}
        </p>
        <p>Schema: {target.openapi.schema?.file_path || "Inline schema"}</p>
      </div>
    );
  }

  return null;
}
