import { Target, TargetType } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Trash2, Globe, Terminal, Server, Network } from "lucide-react";
import { TooltipProvider, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

interface TargetItemProps {
  target: Target;
  index: number;
  onDelete: (index: number) => void;
  isUpdating: boolean;
}

const getTargetIcon = (type: TargetType) => {
  switch (type) {
    case "sse":
      return <Globe className="h-4 w-4" />;
    case "stdio":
      return <Terminal className="h-4 w-4" />;
    case "openapi":
      return <Server className="h-4 w-4" />;
    case "a2a":
      return <Network className="h-4 w-4" />;
    default:
      return <Server className="h-4 w-4" />;
  }
};

const getTargetType = (target: Target): TargetType => {
  if (target.stdio) return "stdio";
  if (target.sse) return "sse";
  if (target.openapi) return "openapi";
  if (target.a2a) return "a2a";
  return "sse";
};

export default function TargetItem({ target, index, onDelete, isUpdating }: TargetItemProps) {
  const targetType = getTargetType(target);

  return (
    <div className="flex items-center justify-between w-full">
      <div className="flex items-center space-x-2">
        {getTargetIcon(targetType)}
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
      <div className="flex items-center space-x-2">
        <Badge variant="outline">{targetType}</Badge>
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
