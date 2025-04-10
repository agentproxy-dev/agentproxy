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
  const [url, setUrl] = useState("")
  const [command, setCommand] = useState("npx")
  const [args, setArgs] = useState("")
  const [targetToDelete, setTargetToDelete] = useState<number | null>(null)

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
      try {
        const urlObj = new URL(url)
        targetConfig.sse = {
          host: urlObj.hostname,
          port: parseInt(urlObj.port || "80", 10),
          path: urlObj.pathname + urlObj.search,
          headers: {},
        }
      } catch (error) {
        console.error("Invalid URL:", error)
        return
      }
    } else if (serverType === "openapi") {
      targetConfig.openapi = {
        host: url,
        port: 80,
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
    setUrl("")
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

  const handleDeleteTarget = (index: number) => {
    setTargetToDelete(index)
  }

  const confirmDelete = () => {
    if (targetToDelete !== null) {
      removeTarget(targetToDelete)
      setTargetToDelete(null)
    }
  }

  const cancelDelete = () => {
    setTargetToDelete(null)
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
                onClick={() => handleDeleteTarget(index)}
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
                  <div className="space-y-2">
                    <Label htmlFor="url">Server URL</Label>
                    <Input
                      id="url"
                      type="url"
                      value={url}
                      onChange={(e) => setUrl(e.target.value)}
                      placeholder="http://localhost:3000/events"
                      required
                    />
                    <p className="text-sm text-muted-foreground">
                      Enter the full URL including protocol, hostname, port, and path
                    </p>
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
                  <div className="space-y-2">
                    <Label htmlFor="openapi-url">Server URL</Label>
                    <Input
                      id="openapi-url"
                      type="url"
                      value={url}
                      onChange={(e) => setUrl(e.target.value)}
                      placeholder="http://localhost:3000"
                      required
                    />
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

      <Dialog open={targetToDelete !== null} onOpenChange={(open) => !open && cancelDelete()}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Target Server</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this target server? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-2 mt-4">
            <Button variant="outline" onClick={cancelDelete}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={confirmDelete}>
              Delete
            </Button>
          </div>
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
    const path = target.sse.path || "/"
    const truncatedPath = path.length > 30 ? path.substring(0, 27) + "..." : path
    return (
      <div className="text-sm text-muted-foreground">
        <p>
          Host: {target.sse.host}:{target.sse.port}
        </p>
        <p>Path: {truncatedPath}</p>
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
