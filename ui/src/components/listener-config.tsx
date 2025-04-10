"use client"

import { useState, useEffect } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Label } from "@/components/ui/label"
import { Input } from "@/components/ui/input"
import { Config } from "@/lib/types"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { AlertCircle, Info } from "lucide-react"

interface ListenerConfigProps {
  config: Config
  onChange: (config: Config) => void
  serverAddress?: string
  serverPort?: number
}

export function ListenerConfig({ config, onChange, serverAddress, serverPort }: ListenerConfigProps) {
  const [address, setAddress] = useState<string>("")
  const [port, setPort] = useState<string>("")
  const [error, setError] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  // Fetch listener configuration from the proxy API
  useEffect(() => {
    const fetchListenerConfig = async () => {
      if (!serverAddress || !serverPort) {
        setIsLoading(false)
        return
      }

      setIsLoading(true)
      setError(null)

      try {
        const baseUrl = `http://${serverAddress}:${serverPort}`
        const response = await fetch(`${baseUrl}/listeners`)
        
        if (!response.ok) {
          throw new Error(`Failed to fetch listener configuration: ${response.status} ${response.statusText}`)
        }
        
        const data = await response.json()
        
        // Extract the listener configuration from the response
        if (data && data.sse) {
          setAddress(data.sse.address || "0.0.0.0")
          setPort(data.sse.port?.toString() || "5555")
        } else {
          // Fallback to the config prop if the API response doesn't have the expected format
          setAddress(config.listeners[0]?.sse?.address || "0.0.0.0")
          setPort(config.listeners[0]?.sse?.port?.toString() || "5555")
        }
      } catch (err) {
        console.error("Error fetching listener configuration:", err)
        setError(err instanceof Error ? err.message : "Failed to fetch listener configuration")
        
        // Fallback to the config prop if the API request fails
        setAddress(config.listeners[0]?.sse?.address || "0.0.0.0")
        setPort(config.listeners[0]?.sse?.port?.toString() || "5555")
      } finally {
        setIsLoading(false)
      }
    }

    fetchListenerConfig()
  }, [serverAddress, serverPort, config])

  return (
    <Card>
      <CardHeader>
        <CardTitle>Listener Configuration</CardTitle>
        <CardDescription>
          Current SSE listener configuration for the proxy server
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        
        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
            <span className="ml-2">Loading listener configuration...</span>
          </div>
        ) : (
          <>
            <div className="space-y-2">
              <Label htmlFor="address">Address</Label>
              <Input
                id="address"
                value={address}
                readOnly
                className="bg-muted"
              />
              <p className="text-xs text-muted-foreground">
                The IP address the listener is bound to. 0.0.0.0 means it's listening on all interfaces.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="port">Port</Label>
              <Input
                id="port"
                value={port}
                readOnly
                className="bg-muted"
              />
              <p className="text-xs text-muted-foreground">
                The port number the listener is using.
              </p>
            </div>
            <Alert className="bg-blue-50 text-blue-800 border-blue-200">
              <Info className="h-4 w-4" />
              <AlertDescription>
                This configuration is read-only. Changes to the listener configuration require server restart.
              </AlertDescription>
            </Alert>
          </>
        )}
      </CardContent>
    </Card>
  )
}
