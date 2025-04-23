import { Target, TargetType, TargetWithType } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Trash2 } from "lucide-react";
import { TooltipProvider, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

interface TargetItemProps {
  target: Target;
  index: number;
  onDelete: (index: number) => void;
  isUpdating: boolean;
}

export const getTargetType = (target: TargetWithType): TargetType => {
  switch (target.type) {
    case "a2a":
      return "a2a";
    case "mcp":
      return "mcp";
    case "openapi":
      return "openapi";
    case "stdio":
      return "stdio";
    case "sse":
      return "sse";
    default:
      return "unknown";
  }
};

export default function TargetItem({ target, index, onDelete, isUpdating }: TargetItemProps) {
  const targetType = getTargetType(target as TargetWithType);

  return (
    <div className="flex items-center justify-between w-full">
      <div className="flex items-center space-x-2">
        <Badge variant="outline">{targetType}</Badge>
        <div>
          <div className="font-medium">{target.name}</div>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="text-xs text-muted-foreground truncate max-w-[400px]">
                  {target.sse && `${target.sse.host}:${target.sse.port}${target.sse.path}`}
                  {target.stdio && `${target.stdio.cmd} ${target.stdio.args?.join(" ")}`}
                  {target.openapi && `${target.openapi.host}:${target.openapi.port}`}
                  {target.a2a && `${target.a2a.host}:${target.a2a.port}${target.a2a.path}`}
                </div>
              </TooltipTrigger>
              <TooltipContent>
                {target.sse && `${target.sse.host}:${target.sse.port}${target.sse.path}`}
                {target.stdio && `${target.stdio.cmd} ${target.stdio.args?.join(" ")}`}
                {target.openapi && `${target.openapi.host}:${target.openapi.port}`}
                {target.a2a && `${target.a2a.host}:${target.a2a.port}${target.a2a.path}`}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <div className="flex flex-wrap items-center gap-1">
          {target.listeners === undefined ? (
            <Badge variant="secondary" className="text-xs">
              All listeners
            </Badge>
          ) : target.listeners.length === 0 ? (
            <Badge variant="secondary" className="text-xs bg-muted">
              No listeners
            </Badge>
          ) : (
            target.listeners.map((listener) => (
              <Badge key={listener} variant="secondary" className="text-xs">
                {listener}
              </Badge>
            ))
          )}
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => onDelete(index)}
          className="h-8 w-8"
          disabled={isUpdating}
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
