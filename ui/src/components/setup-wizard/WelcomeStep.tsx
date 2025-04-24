import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { AgentgatewayLogo } from "@/components/agentgateway-logo";
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
          <AgentgatewayLogo className="h-12" />
        </div>
        <CardTitle className="text-center">Welcome to agentgateway</CardTitle>
        <CardDescription className="text-center">
          Let&apos;s get your gateway up and running in just a few steps
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <h3 className="font-medium">What is Agent Gateway?</h3>
          <p className="text-sm text-muted-foreground">
            <a
              href="https://agentgateway.dev"
              className="text-accent-foreground hover:text-accent/90 hover:underline"
            >
              Agentgateway
            </a>{" "}
            is an open source tool that helps you to connect, secure, and observe agent-to-agent and
            agent-to-tool communication across any agent framework and environment.
          </p>
        </div>
        <div className="space-y-2">
          <h3 className="font-medium">What you&apos;ll configure:</h3>
          <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
            <li>A2A or MCP listener settings for your gateway</li>
            <li>Target servers that your gateway will forward requests to</li>
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
