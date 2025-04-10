"use client"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Listener } from "@/lib/types"

interface ListenerConfigProps {
  listener: Listener
  updateListener: (listener: Listener) => void
}

export function ListenerConfig({ listener, updateListener }: ListenerConfigProps) {
  const handleAddressChange = (address: string) => {
    updateListener({
      ...listener,
      sse: {
        ...listener.sse,
        address,
      },
    })
  }

  const handlePortChange = (port: number) => {
    updateListener({
      ...listener,
      sse: {
        ...listener.sse,
        port,
      },
    })
  }

  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h3 className="text-lg font-medium mb-2">Listener Configuration</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Configure the SSE listener for your MCP proxy
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>SSE Listener</CardTitle>
          <CardDescription>Configure the Server-Sent Events listener</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="address">Address</Label>
            <Input
              id="address"
              value={listener.sse.address}
              onChange={(e) => handleAddressChange(e.target.value)}
              placeholder="0.0.0.0"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="port">Port</Label>
            <Input
              id="port"
              type="number"
              value={listener.sse.port}
              onChange={(e) => handlePortChange(parseInt(e.target.value))}
              placeholder="3000"
            />
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
