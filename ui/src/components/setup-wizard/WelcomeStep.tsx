import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { MCPLogo } from "@/components/mcp-logo";
import { ArrowRight } from "lucide-react";

interface WelcomeStepProps {
  onNext: () => void;
  onSkip: () => void;
}

export function WelcomeStep({ onNext, onSkip }: WelcomeStepProps) {
  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <div className="flex justify-center mb-6">
          <MCPLogo className="h-12" />
        </div>
        <CardTitle className="text-center">Welcome to agent-proxy</CardTitle>
        <CardDescription className="text-center">
          Let&apos;s get your proxy server up and running in just a few steps
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <h3 className="font-medium">What is agent-proxy?</h3>
          <p className="text-sm text-muted-foreground">
            Agentproxy is a powerful tool that helps you manage and secure your server connections.
            It allows you to configure listeners, set up target servers, and implement security
            policies.
          </p>
        </div>
        <div className="space-y-2">
          <h3 className="font-medium">What you&apos;ll configure:</h3>
          <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
            <li>Listener settings for your proxy server</li>
            <li>Target servers that your proxy will forward requests to</li>
            <li>Security policies to protect your infrastructure</li>
          </ul>
        </div>
      </CardContent>
      <CardFooter className="flex justify-between">
        <Button variant="outline" onClick={onSkip}>
          Skip Wizard
        </Button>
        <Button onClick={onNext}>
          Start Setup
          <ArrowRight className="ml-2 h-4 w-4" />
        </Button>
      </CardFooter>
    </Card>
  );
}
