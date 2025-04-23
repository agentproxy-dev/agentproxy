"use client";

import { Tool as McpTool } from "@modelcontextprotocol/sdk/types.js";
import { AgentSkill } from "@/lib/a2a-schema";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Checkbox } from "@/components/ui/checkbox";
import { Loader2, Send, BotMessageSquare } from "lucide-react";

interface ActionPanelProps {
  connectionType: "mcp" | "a2a" | null;
  mcpSelectedTool: McpTool | null;
  a2aSelectedSkill: AgentSkill | null;
  mcpParamValues: Record<string, any>;
  a2aMessage: string;
  isRequestRunning: boolean;
  onMcpParamChange: (key: string, value: any) => void;
  onA2aMessageChange: (message: string) => void;
  onRunMcpTool: () => void;
  onRunA2aSkill: () => void;
}

export function ActionPanel({
  connectionType,
  mcpSelectedTool,
  a2aSelectedSkill,
  mcpParamValues,
  a2aMessage,
  isRequestRunning,
  onMcpParamChange,
  onA2aMessageChange,
  onRunMcpTool,
  onRunA2aSkill,
}: ActionPanelProps) {
  const isMcp = connectionType === "mcp" && mcpSelectedTool;
  const isA2a = connectionType === "a2a" && a2aSelectedSkill;

  if (!isMcp && !isA2a) {
    return (
      <Card className="flex flex-col">
        <div className="flex-1 flex items-center justify-center p-4 text-muted-foreground bg-card h-full">
          Select one of the available {connectionType === "a2a" ? "skills" : "tools"}
        </div>
      </Card>
    );
  }

  const title = isA2a ? a2aSelectedSkill?.name : mcpSelectedTool?.name;
  const description = isA2a ? a2aSelectedSkill?.description : mcpSelectedTool?.description;

  return (
    <Card className="flex flex-col">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4 flex-grow flex flex-col">
        <div className="flex-grow space-y-4">
          {isMcp &&
            mcpSelectedTool &&
            Object.entries(mcpSelectedTool.inputSchema.properties || {}).map(
              ([key, prop]: [string, any]) => (
                <div key={key} className="space-y-2">
                  <Label htmlFor={key}>
                    {key}
                    {Array.isArray(mcpSelectedTool.inputSchema.required) &&
                      mcpSelectedTool.inputSchema.required.includes(key) && (
                        <span className="text-red-500 ml-1">*</span>
                      )}
                  </Label>
                  {prop.type === "boolean" ? (
                    <div className="flex items-center space-x-2">
                      <Checkbox
                        id={key}
                        checked={!!mcpParamValues[key]}
                        onCheckedChange={(checked) => onMcpParamChange(key, Boolean(checked))}
                      />
                      <label htmlFor={key} className="text-sm text-muted-foreground">
                        {prop.description || "Toggle option"}
                      </label>
                    </div>
                  ) : prop.type === "string" &&
                    (prop.format === "textarea" ||
                      (prop.description && prop.description.length > 80)) ? (
                    <Textarea
                      id={key}
                      placeholder={prop.description}
                      value={mcpParamValues[key] || ""}
                      onChange={(e) => onMcpParamChange(key, e.target.value)}
                      rows={3}
                    />
                  ) : prop.type === "number" || prop.type === "integer" ? (
                    <Input
                      type="number"
                      id={key}
                      placeholder={prop.description}
                      value={mcpParamValues[key] ?? ""}
                      onChange={(e) =>
                        onMcpParamChange(key, e.target.value === "" ? null : Number(e.target.value))
                      }
                    />
                  ) : (
                    <Input
                      id={key}
                      placeholder={prop.description}
                      value={mcpParamValues[key] || ""}
                      onChange={(e) => onMcpParamChange(key, e.target.value)}
                    />
                  )}
                </div>
              )
            )}

          {isA2a && a2aSelectedSkill && (
            <div className="space-y-2 flex flex-col flex-grow">
              <Label htmlFor="a2aMessage">Message</Label>
              <Textarea
                id="a2aMessage"
                placeholder={`Enter message for ${a2aSelectedSkill.name}...`}
                value={a2aMessage}
                onChange={(e) => onA2aMessageChange(e.target.value)}
                className="flex-grow"
                rows={5}
              />
            </div>
          )}
        </div>

        <Button
          onClick={isA2a ? onRunA2aSkill : onRunMcpTool}
          disabled={isRequestRunning || (isA2a && !a2aMessage.trim()) ? true : undefined}
          className="w-full mt-auto"
        >
          {isRequestRunning ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Running...
            </>
          ) : isA2a ? (
            <>
              <BotMessageSquare className="mr-2 h-4 w-4" />
              Send Task
            </>
          ) : (
            <>
              <Send className="mr-2 h-4 w-4" />
              Run Tool
            </>
          )}
        </Button>
      </CardContent>
    </Card>
  );
}
