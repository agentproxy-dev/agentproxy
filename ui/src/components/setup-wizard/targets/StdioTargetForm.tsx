import { useState, useEffect, forwardRef, useImperativeHandle } from "react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { ChevronUp, ChevronDown } from "lucide-react";
import { TargetWithType } from "@/lib/types";

interface StdioTargetFormProps {
  targetName: string;
  onSubmit: (target: TargetWithType) => Promise<void>;
  isLoading: boolean;
  existingTarget?: TargetWithType;
  hideSubmitButton?: boolean;
}

export const StdioTargetForm = forwardRef<
  { submitForm: () => Promise<void> },
  StdioTargetFormProps
>(({ targetName, onSubmit, isLoading, existingTarget, hideSubmitButton = false }, ref) => {
  const [command, setCommand] = useState("npx");
  const [args, setArgs] = useState("");
  const [showStdioAdvancedSettings, setShowStdioAdvancedSettings] = useState(false);
  const [envVars, setEnvVars] = useState<{ [key: string]: string }>({});
  const [envKey, setEnvKey] = useState("");
  const [envValue, setEnvValue] = useState("");

  // Initialize form with existing target data if provided
  useEffect(() => {
    if (existingTarget?.stdio) {
      const { cmd, args: targetArgs, env } = existingTarget.stdio;
      setCommand(cmd || "npx");
      setArgs(Array.isArray(targetArgs) ? targetArgs.join(" ") : "");
      setEnvVars(env || {});
    }
  }, [existingTarget]);

  const addEnvVar = () => {
    if (envKey && envValue) {
      setEnvVars({ ...envVars, [envKey]: envValue });
      setEnvKey("");
      setEnvValue("");
    }
  };

  const removeEnvVar = (key: string) => {
    const newEnvVars = { ...envVars };
    delete newEnvVars[key];
    setEnvVars(newEnvVars);
  };

  const handleSubmit = async () => {
    try {
      const target: TargetWithType = {
        name: targetName,
        type: "stdio",
        stdio: {
          cmd: command,
          args: args.split(" ").filter((arg) => arg.trim() !== ""),
          env: Object.keys(envVars).length > 0 ? envVars : {},
        },
      };

      await onSubmit(target as TargetWithType);
    } catch (err) {
      console.error("Error creating stdio target:", err);
      throw err;
    }
  };

  useImperativeHandle(ref, () => ({
    submitForm: handleSubmit,
  }));

  return (
    <form
      id="mcp-target-form"
      onSubmit={(e) => {
        e.preventDefault();
        handleSubmit();
      }}
      className="space-y-4 pt-4"
    >
      <div className="space-y-2">
        <Label htmlFor="command">Command</Label>
        <Input
          id="command"
          value={command}
          onChange={(e) => setCommand(e.target.value)}
          placeholder="e.g., npx"
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="args">Arguments</Label>
        <Input
          id="args"
          value={args}
          onChange={(e) => setArgs(e.target.value)}
          placeholder="e.g., @modelcontextprotocol/server-everything"
        />
      </div>

      <Collapsible open={showStdioAdvancedSettings} onOpenChange={setShowStdioAdvancedSettings}>
        <CollapsibleTrigger asChild>
          <Label className="flex items-center p-0 h-auto cursor-pointer hover:text-muted-foreground">
            {showStdioAdvancedSettings ? (
              <ChevronUp className="h-4 w-4 mr-1" />
            ) : (
              <ChevronDown className="h-4 w-4 mr-1" />
            )}
            Advanced Settings
          </Label>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-4 pt-4">
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Environment Variables</Label>
              <div className="space-y-2">
                {Object.entries(envVars).map(([key, value]) => (
                  <div key={key} className="flex items-center gap-2">
                    <div className="flex-1">
                      <Input value={key} disabled placeholder="Variable name" />
                    </div>
                    <div className="flex-1">
                      <Input value={value} disabled placeholder="Variable value" />
                    </div>
                    <Button
                      type="button"
                      variant="outline"
                      size="icon"
                      onClick={() => removeEnvVar(key)}
                    >
                      <span className="sr-only">Remove environment variable</span>
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="24"
                        height="24"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        className="h-4 w-4"
                      >
                        <path d="M18 6 6 18" />
                        <path d="m6 6 12 12" />
                      </svg>
                    </Button>
                  </div>
                ))}
                <div className="flex items-center gap-2">
                  <div className="flex-1">
                    <Input
                      value={envKey}
                      onChange={(e) => setEnvKey(e.target.value)}
                      placeholder="Variable name"
                    />
                  </div>
                  <div className="flex-1">
                    <Input
                      value={envValue}
                      onChange={(e) => setEnvValue(e.target.value)}
                      placeholder="Variable value"
                    />
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={addEnvVar}
                    disabled={!envKey || !envValue}
                  >
                    Add
                  </Button>
                </div>
              </div>
            </div>
          </div>
        </CollapsibleContent>
      </Collapsible>

      {!hideSubmitButton && (
        <Button type="submit" className="w-full" disabled={isLoading || !command}>
          {isLoading
            ? existingTarget
              ? "Updating Target..."
              : "Adding Target..."
            : existingTarget
              ? "Update stdio Target"
              : "Add stdio Target"}
        </Button>
      )}
    </form>
  );
});

StdioTargetForm.displayName = "StdioTargetForm";
