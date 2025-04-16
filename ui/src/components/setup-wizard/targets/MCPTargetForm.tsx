import { useState, useEffect } from "react";
import { Label } from "@/components/ui/label";
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
  // Initialize target type based on existing target if available
  const getInitialTargetType = (): TargetType => {
    console.log('existingTarget', existingTarget)
    if (existingTarget) {
      if (existingTarget.stdio) return "stdio";
      if (existingTarget.openapi) return "openapi";
      if (existingTarget.sse) return "sse";
    }
    return "sse"; // Default to SSE if no existing target
  };

  const [targetType, setTargetType] = useState<TargetType>(getInitialTargetType());

  // Update target type when existingTarget changes
  useEffect(() => {
    const newType = getInitialTargetType();
    setTargetType(newType);
  }, [existingTarget]);

  console.log('target', existingTarget)
  console.log(targetType);
  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <Label>Target Type</Label>
        <Tabs defaultValue={targetType} value={targetType} onValueChange={(value) => setTargetType(value as TargetType)}>
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
              hideSubmitButton={true}
            />
          </TabsContent>

          <TabsContent value="stdio">
            <StdioTargetForm
              targetName={targetName}
              onSubmit={onSubmit}
              isLoading={isLoading}
              existingTarget={existingTarget}
              hideSubmitButton={true}
            />
          </TabsContent>

          <TabsContent value="openapi">
            <OpenAPITargetForm
              targetName={targetName}
              onSubmit={onSubmit}
              isLoading={isLoading}
              existingTarget={existingTarget}
              hideSubmitButton={true}
            />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
