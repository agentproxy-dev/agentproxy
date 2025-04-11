import { AlertCircle } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";

interface ConnectionErrorProps {
  error: string;
  onRetry: () => void;
}

export function ConnectionError({ error, onRetry }: ConnectionErrorProps) {
  return (
    <div className="flex flex-col items-center justify-center min-h-[400px] p-6 space-y-6">
      <Alert variant="destructive" className="max-w-md">
        <AlertCircle className="h-4 w-4" />
        <AlertTitle>Connection Failed</AlertTitle>
        <AlertDescription className="mt-2">{error}</AlertDescription>
      </Alert>
      <Button onClick={onRetry} variant="outline">
        Try Again
      </Button>
    </div>
  );
}
