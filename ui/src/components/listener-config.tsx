"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Check, X } from "lucide-react"
import { Listener, SseListener } from "@/lib/types"

interface ListenerConfigProps {
  listener: Listener | null
  updateListener: (listener: Listener | null) => void
}

export function ListenerConfig({ listener, updateListener }: ListenerConfigProps) {
  const [editingListener, setEditingListener] = useState<Listener | null>(null)

  const handleAddListener = () => {
    setEditingListener({
      sse: {
        address: "0.0.0.0",
        port: 3000
      }
    })
  }

  const handleSaveListener = () => {
    if (editingListener) {
      updateListener(editingListener)
      setEditingListener(null)
    }
  }

  const renderListenerForm = (listener: Listener) => {
    return (
      <div className="space-y-4 max-w-md">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <Label htmlFor="sse-address">Host</Label>
            <Input
              id="sse-address"
              value={listener.sse.address}
              onChange={(e) => {
                setEditingListener({
                  ...listener,
                  sse: { ...listener.sse, address: e.target.value }
                })
              }}
            />
          </div>
          <div>
            <Label htmlFor="sse-port">Port</Label>
            <Input
              id="sse-port"
              type="number"
              value={listener.sse.port}
              onChange={(e) => {
                setEditingListener({
                  ...listener,
                  sse: { ...listener.sse, port: parseInt(e.target.value) }
                })
              }}
            />
          </div>
        </div>

        <div className="flex justify-end gap-2">
          <Button
            variant="outline"
            onClick={() => setEditingListener(null)}
          >
            <X className="h-4 w-4 mr-2" />
            Cancel
          </Button>
          <Button
            onClick={handleSaveListener}
          >
            <Check className="h-4 w-4 mr-2" />
            Save
          </Button>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-4 max-w-md">
      <div>
        <h3 className="text-lg font-medium mb-2">Listener Configuration</h3>
        <p className="text-sm text-muted-foreground mb-4">Configure how the proxy listens for connections</p>
      </div>

      {editingListener ? (
        renderListenerForm(editingListener)
      ) : listener ? (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <h4 className="font-medium">Server-Sent Events (SSE)</h4>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-muted-foreground">Host:</span> {listener.sse.address}
                </div>
                <div>
                  <span className="text-muted-foreground">Port:</span> {listener.sse.port}
                </div>
              </div>
            </div>
            <Button
              variant="outline"
              onClick={() => setEditingListener(listener)}
            >
              Edit
            </Button>
          </div>
        </div>
      ) : (
        <Button onClick={handleAddListener}>
          Add Listener
        </Button>
      )}
    </div>
  )
}
