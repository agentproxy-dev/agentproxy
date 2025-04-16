import { useState, useEffect } from "react";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Globe, Terminal, Server } from "lucide-react";
import { SSETargetForm } from "./SSETargetForm";
import { StdioTargetForm } from "./StdioTargetForm";
import { OpenAPITargetForm } from "./OpenAPITargetForm";
import { Target, TargetType } from "@/lib/types";

interface MCPTargetFormProps {
  targetName: string;
  onTargetNameChange: (name: string) => void;
  onSubmit: (target: Target) => Promise<void>;
  isLoading: boolean;
  existingTarget?: Target;
}

export function MCPTargetForm({
  targetName,
  onTargetNameChange,
  onSubmit,
  isLoading,
  existingTarget,
}: MCPTargetFormProps) {
  const [targetType, setTargetType] = useState<TargetType>("sse");

  // Set the target type based on the existing target if provided
  useEffect(() => {
    if (existingTarget) {
      if (existingTarget.sse) setTargetType("sse");
      else if (existingTarget.stdio) setTargetType("stdio");
      else if (existingTarget.openapi) setTargetType("openapi");
    }
  }, [existingTarget]);

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <Label htmlFor="targetName">Target Name</Label>
        <Input
          id="targetName"
          value={targetName}
          onChange={(e) => onTargetNameChange(e.target.value)}
          placeholder="e.g., local-model"
        />
      </div>

      <div className="space-y-2">
        <Label>Target Type</Label>
        <Tabs value={targetType} onValueChange={(value) => setTargetType(value as TargetType)}>
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="sse" className="flex items-center">
              <Globe className="h-4 w-4 mr-2" />
              SSE
            </TabsTrigger>
            <TabsTrigger value="stdio" className="flex items-center">
              <Terminal className="h-4 w-4 mr-2" />
              stdio
            </TabsTrigger>
            <TabsTrigger value="openapi" className="flex items-center">
              <Server className="h-4 w-4 mr-2" />
              OpenAPI
            </TabsTrigger>
          </TabsList>

          <TabsContent value="sse">
            <SSETargetForm
              targetName={targetName}
              onSubmit={onSubmit}
              isLoading={isLoading}
              existingTarget={existingTarget}
            />
          </TabsContent>

          <TabsContent value="stdio">
            <StdioTargetForm
              targetName={targetName}
              onSubmit={onSubmit}
              isLoading={isLoading}
              existingTarget={existingTarget}
            />
          </TabsContent>

          <TabsContent value="openapi">
            <OpenAPITargetForm
              targetName={targetName}
              onSubmit={onSubmit}
              isLoading={isLoading}
              existingTarget={existingTarget}
            />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
