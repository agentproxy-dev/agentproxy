"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Target, Listener, TargetType } from "@/lib/types"

interface AddServerFormProps {
  addTarget?: (target: Target) => void
  updateListener?: (listener: Listener) => void
  currentListener?: Listener
}

export function AddServerForm({ addTarget, updateListener, currentListener }: AddServerFormProps) {
  const [serverType, setServerType] = useState<TargetType>("sse")
  const [serverName, setServerName] = useState("")
  const [host, setHost] = useState("localhost")
  const [port, setPort] = useState(3000)
  const [path, setPath] = useState("/")
  const [command, setCommand] = useState("npx")
  const [args, setArgs] = useState("")
  const [schemaPath, setSchemaPath] = useState("")

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()

    if (addTarget) {
      const targetConfig: Target = {
        name: serverName,
      }

      if (serverType === "stdio") {
        targetConfig.stdio = {
          cmd: command,
          args: args.split(" ").filter((arg) => arg.trim() !== ""),
          env: {},
        }
      } else if (serverType === "sse") {
        targetConfig.sse = {
          host,
          port: port,
          path: path,
          headers: {},
        }
      } else if (serverType === "openapi") {
        targetConfig.openapi = {
          host,
          port: port,
          schema: {
            file_path: schemaPath,
          },
        }
      }
      addTarget(targetConfig)
      setServerName("")
      setArgs("")
      setSchemaPath("")
    } else if (updateListener) {
      const listenerConfig: Listener = {
        sse: {
          address: host,
          port: port,
        }
      }
      updateListener(listenerConfig)
    }
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="space-y-2">
        <Label htmlFor="name">Server Name</Label>
        <Input
          id="name"
          value={serverName}
          onChange={(e) => setServerName(e.target.value)}
          placeholder="Enter server name"
          required
        />
      </div>


      <div className="space-y-2">
        <Label>Server Type</Label>
        <Tabs value={serverType} onValueChange={(value) => setServerType(value as TargetType)} className="w-full">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="stdio">Stdio</TabsTrigger>
            <TabsTrigger value="sse">SSE</TabsTrigger>
            <TabsTrigger value="openapi">OpenAPI</TabsTrigger>
          </TabsList>

          <TabsContent value="stdio" className="space-y-4 mt-4">
            <>
              <div className="space-y-2">
                <Label htmlFor="command">Command</Label>
                <Input
                  id="command"
                  value={command}
                  onChange={(e) => setCommand(e.target.value)}
                  placeholder="Command to execute"
                  required
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="args">Arguments</Label>
                <Input
                  id="args"
                  value={args}
                  onChange={(e) => setArgs(e.target.value)}
                  placeholder="Space-separated arguments"
                />
              </div>
            </>
          </TabsContent>

          <TabsContent value="sse" className="space-y-4 mt-4">
            <div className="space-y-2">
              <Label htmlFor="host">Host</Label>
              <Input
                id="host"
                value={host}
                onChange={(e) => setHost(e.target.value)}
                placeholder="Host address"
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="port">Port</Label>
              <Input
                id="port"
                type="number"
                value={port}
                onChange={(e) => setPort(Number(e.target.value))}
                placeholder="Port number"
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="path">Path</Label>
              <Input
                id="path"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                placeholder="/"
              />
            </div>
          </TabsContent>

          <TabsContent value="openapi" className="space-y-4 mt-4">
            <div className="space-y-2">
              <Label htmlFor="openapi-host">Host</Label>
              <Input
                id="openapi-host"
                value={host}
                onChange={(e) => setHost(e.target.value)}
                placeholder="Host address"
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="openapi-port">Port</Label>
              <Input
                id="openapi-port"
                type="number"
                value={port}
                onChange={(e) => setPort(Number(e.target.value))}
                placeholder="Port number"
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="schema-path">Schema Path</Label>
              <Input
                id="schema-path"
                value={schemaPath}
                onChange={(e) => setSchemaPath(e.target.value)}
                placeholder="Path to OpenAPI schema file"
                required
              />
            </div>
          </TabsContent>

        </Tabs>
      </div>

      <Button type="submit" className="w-full">
        Add Server
      </Button>
    </form>
  )
}
