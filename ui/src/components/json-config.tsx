"use client";

import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Textarea } from "@/components/ui/textarea";
import { Config } from "@/lib/types";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";

interface JsonConfigProps {
  config: Config;
  onConfigChange: (config: Config) => void;
}

export function JsonConfig({ config, onConfigChange }: JsonConfigProps) {
  const [jsonText, setJsonText] = useState(JSON.stringify(config, null, 2));
  const [error, setError] = useState<string | null>(null);

  const handleJsonChange = (value: string) => {
    setJsonText(value);
    try {
      const parsedConfig = JSON.parse(value) as Config;
      onConfigChange(parsedConfig);
      setError(null);
    } catch {
      setError("Invalid JSON format");
    }
  };

  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h3 className="text-lg font-medium mb-2">JSON Configuration</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Edit your MCP proxy configuration in JSON format
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Configuration</CardTitle>
          <CardDescription>Edit the configuration in JSON format</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <Textarea
              value={jsonText}
              onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
                handleJsonChange(e.target.value)
              }
              className="font-mono h-[500px]"
              placeholder="Enter JSON configuration"
            />
            {error && (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
