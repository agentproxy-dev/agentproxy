"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Badge } from "@/components/ui/badge"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Trash2, PlusCircle, Server, Globe, Terminal } from "lucide-react"
import { Target, TargetType } from "@/lib/types"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog"

interface TargetsConfigProps {
  targets: Target[]
  addTarget: (target: Target) => void
  removeTarget: (index: number) => void
}

export function TargetsConfig({ targets, addTarget, removeTarget }: TargetsConfigProps) {
  const [isAddingTarget, setIsAddingTarget] = useState(false)
  const [serverType, setServerType] = useState<TargetType>("sse")
  const [serverName, setServerName] = useState("")
  const [host, setHost] = useState("localhost")
  const [port, setPort] = useState(3000)
  const [path, setPath] = useState("/")
  const [command, setCommand] = useState("npx")
  const [args, setArgs] = useState("")

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()

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
          file_path: "",
        },
      }
    }

    addTarget(targetConfig)
    resetForm()
    setIsAddingTarget(false)
  }

  const resetForm = () => {
    setServerName("")
    setHost("localhost")
    setPort(3000)
    setPath("/")
    setCommand("npx")
    setArgs("")
  }

  const getTargetIcon = (type: TargetType) => {
    switch (type) {
      case "sse":
        return <Globe className="h-4 w-4" />
      case "stdio":
        return <Terminal className="h-4 w-4" />
      case "openapi":
        return <Server className="h-4 w-4" />
      default:
        return <Server className="h-4 w-4" />
    }
  }

  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h3 className="text-lg font-medium mb-2">Target Servers</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Configure MCP servers that the proxy connects to
        </p>
      </div>

      {targets.length === 0 && !isAddingTarget ? (
        <Alert>
          <AlertDescription>No target servers configured. Add a server to get started.</AlertDescription>
        </Alert>
      ) : (
        <div className="space-y-4">
          {targets.map((target, index) => (
            <div
              key={index}
              id={`target-${index}`}
              className="border rounded-lg p-4 flex justify-between items-start"
            >
              <div>
                <h4 className="font-medium">{target.name}</h4>
                <div className="flex items-center mt-1">
                  <Badge variant="outline" className="mr-2 flex items-center">
                    {getTargetIcon(getTargetType(target))}
                    <span className="ml-1">{getTargetType(target)}</span>
                  </Badge>
                  {renderTargetDetails(target)}
                </div>
              </div>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => removeTarget(index)}
                className="text-muted-foreground hover:text-destructive"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ))}
        </div>
      )}

      <Button onClick={() => setIsAddingTarget(true)} className="flex items-center">
        <PlusCircle className="h-4 w-4 mr-2" />
        Add Target Server
      </Button>

      <Dialog open={isAddingTarget} onOpenChange={setIsAddingTarget}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Target Server</DialogTitle>
            <DialogDescription>
              Configure a new target server for the proxy to connect to.
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSubmit} className="space-y-4 mt-6">
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
              <Tabs value={serverType} onValueChange={(value) => setServerType(value as TargetType)}>
                <TabsList className="grid grid-cols-3">
                  <TabsTrigger value="sse" className="flex items-center">
                    <Globe className="h-4 w-4 mr-2" />
                    SSE
                  </TabsTrigger>
                  <TabsTrigger value="stdio" className="flex items-center">
                    <Terminal className="h-4 w-4 mr-2" />
                    stdio
                  </TabsTrigger>
                  <TabsTrigger value="openapi" className="flex items-center">
                    <Server className="h-4 w-4 mr-2" />
                    OpenAPI
                  </TabsTrigger>
                </TabsList>

                <TabsContent value="sse" className="space-y-4 pt-4">
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label htmlFor="host">Host</Label>
                      <Input
                        id="host"
                        value={host}
                        onChange={(e) => setHost(e.target.value)}
                        placeholder="localhost"
                        required
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="port">Port</Label>
                      <Input
                        id="port"
                        type="number"
                        value={port}
                        onChange={(e) => setPort(parseInt(e.target.value))}
                        placeholder="3000"
                        required
                      />
                    </div>
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

                <TabsContent value="stdio" className="space-y-4 pt-4">
                  <div className="space-y-2">
                    <Label htmlFor="command">Command</Label>
                    <Input
                      id="command"
                      value={command}
                      onChange={(e) => setCommand(e.target.value)}
                      placeholder="npx"
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="args">Arguments</Label>
                    <Input
                      id="args"
                      value={args}
                      onChange={(e) => setArgs(e.target.value)}
                      placeholder="--port 3000"
                    />
                  </div>
                </TabsContent>

                <TabsContent value="openapi" className="space-y-4 pt-4">
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label htmlFor="openapi-host">Host</Label>
                      <Input
                        id="openapi-host"
                        value={host}
                        onChange={(e) => setHost(e.target.value)}
                        placeholder="localhost"
                        required
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="openapi-port">Port</Label>
                      <Input
                        id="openapi-port"
                        type="number"
                        value={port}
                        onChange={(e) => setPort(parseInt(e.target.value))}
                        placeholder="3000"
                        required
                      />
                    </div>
                  </div>
                </TabsContent>
              </Tabs>

              <div className="flex justify-end">
                <Button type="submit">Add Server</Button>
              </div>
            </div>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  )
}

function getTargetType(target: Target): TargetType {
  if (target.stdio) return "stdio"
  if (target.sse) return "sse"
  if (target.openapi) return "openapi"
  return "sse"
}

function renderTargetDetails(target: Target) {
  if (target.stdio) {
    return (
      <div className="text-sm text-muted-foreground">
        <p>
          Command: {target.stdio.cmd} {target.stdio.args?.join(" ")}
        </p>
      </div>
    )
  }

  if (target.sse) {
    return (
      <div className="text-sm text-muted-foreground">
        <p>
          Host: {target.sse.host}:{target.sse.port}
        </p>
        <p>Path: {target.sse.path || "/"}</p>
      </div>
    )
  }

  if (target.openapi) {
    return (
      <div className="text-sm text-muted-foreground">
        <p>
          Host: {target.openapi.host}:{target.openapi.port}
        </p>
        <p>Schema: {target.openapi.schema?.file_path || "Inline schema"}</p>
      </div>
    )
  }

  return null
}
