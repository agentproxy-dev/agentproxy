import { useState, useEffect, forwardRef, useImperativeHandle, useRef } from "react";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Globe2, Terminal, ServerIcon, Network } from "lucide-react";
import { SSETargetForm } from "./SSETargetForm";
import { StdioTargetForm } from "./StdioTargetForm";
import { OpenAPITargetForm } from "./OpenAPITargetForm";
import { TargetType, TargetWithType } from "@/lib/types";

interface MCPTargetFormProps {
  targetName: string;
  onTargetNameChange: (name: string) => void;
  onSubmit: (target: TargetWithType) => Promise<void>;
  isLoading: boolean;
  existingTarget?: TargetWithType;
}

export const MCPTargetForm = forwardRef<{ submitForm: () => Promise<void> }, MCPTargetFormProps>(
  ({ targetName, onSubmit, isLoading, existingTarget }, ref) => {
    const [targetType, setTargetType] = useState<TargetType>(() =>
      getInitialTargetType(existingTarget)
    );
    const sseFormRef = useRef<{ submitForm: () => Promise<void> } | null>(null);
    const mcpFormRef = useRef<{ submitForm: () => Promise<void> } | null>(null);
    const stdioFormRef = useRef<{ submitForm: () => Promise<void> } | null>(null);
    const openApiFormRef = useRef<{ submitForm: () => Promise<void> } | null>(null);

    // Initialize target type based on existing target if available
    function getInitialTargetType(target?: TargetWithType): TargetType {
      if (target) {
        if (target.stdio) return "stdio";
        if (target.openapi) return "openapi";
        if (target.sse) return "sse";
        if (target.mcp) return "mcp";
      }
      return "sse"; // Default to SSE if no existing target
    }

    useEffect(() => {
      if (existingTarget) {
        setTargetType(getInitialTargetType(existingTarget));
      }
    }, [existingTarget]);

    useImperativeHandle(
      ref,
      () => ({
        submitForm: async () => {
          switch (targetType) {
            case "sse":
              if (sseFormRef.current) await sseFormRef.current.submitForm();
              break;
            case "mcp":
              if (mcpFormRef.current) await mcpFormRef.current.submitForm();
              break;
            case "stdio":
              if (stdioFormRef.current) await stdioFormRef.current.submitForm();
              break;
            case "openapi":
              if (openApiFormRef.current) await openApiFormRef.current.submitForm();
              break;
          }
        },
      }),
      [targetType]
    );

    return (
      <div className="space-y-4">
        <div className="space-y-2">
          <Label>Target Type</Label>
          <Tabs
            defaultValue={targetType}
            value={targetType}
            onValueChange={(value) => setTargetType(value as TargetType)}
          >
            <TabsList className="grid w-full grid-cols-4">
              <TabsTrigger value="sse" className="flex items-center">
                <Globe2 className="h-4 w-4 mr-2" />
                SSE
              </TabsTrigger>
              <TabsTrigger value="mcp" className="flex items-center">
                <Network className="h-4 w-4 mr-2" />
                MCP
              </TabsTrigger>
              <TabsTrigger value="stdio" className="flex items-center">
                <Terminal className="h-4 w-4 mr-2" />
                stdio
              </TabsTrigger>
              <TabsTrigger value="openapi" className="flex items-center">
                <ServerIcon className="h-4 w-4 mr-2" />
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
                ref={sseFormRef}
              />
            </TabsContent>

            <TabsContent value="mcp">
              <SSETargetForm
                targetName={targetName}
                onSubmit={(target) => {
                  // Convert SSE format to MCP format
                  const mcpTarget = {
                    ...target,
                    mcp: target.sse,
                    sse: undefined,
                  };
                  return onSubmit(mcpTarget);
                }}
                isLoading={isLoading}
                existingTarget={
                  existingTarget?.mcp
                    ? { ...existingTarget, sse: existingTarget.mcp }
                    : existingTarget
                }
                hideSubmitButton={true}
                ref={mcpFormRef}
              />
            </TabsContent>

            <TabsContent value="stdio">
              <StdioTargetForm
                targetName={targetName}
                onSubmit={onSubmit}
                isLoading={isLoading}
                existingTarget={existingTarget}
                hideSubmitButton={true}
                ref={stdioFormRef}
              />
            </TabsContent>

            <TabsContent value="openapi">
              <OpenAPITargetForm
                targetName={targetName}
                onSubmit={onSubmit}
                isLoading={isLoading}
                existingTarget={existingTarget}
                hideSubmitButton={true}
                ref={openApiFormRef}
              />
            </TabsContent>
          </Tabs>
        </div>
      </div>
    );
  }
);

MCPTargetForm.displayName = "MCPTargetForm";
