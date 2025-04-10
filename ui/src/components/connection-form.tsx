"use client";

import { ButtonHTMLAttributes, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface ConnectionFormProps {
  onConnect: (address: string, port: number) => Promise<boolean>;
}

export function ConnectionForm({ onConnect }: ConnectionFormProps) {
  const [address, setAddress] = useState("localhost");
  const [port, setPort] = useState(3000);
  const [isConnecting, setIsConnecting] = useState(false);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setIsConnecting(true);

    try {
      await onConnect(address, port);
    } finally {
      setIsConnecting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label htmlFor="address">Server Address</Label>
          <Input id="address" value={address} onChange={(e) => setAddress(e.target.value)} placeholder="localhost or IP address" required />
        </div>

        <div className="space-y-2">
          <Label htmlFor="port">Port</Label>
          <Input id="port" value={port} onChange={(e) => setPort(parseInt(e.target.value))} placeholder="Port number" required />
        </div>
      </div>

      <Button type="submit" disabled={isConnecting} className="w-full">
        {isConnecting ? "Connecting..." : "Connect to Server"}
      </Button>
    </form>
  );
}
