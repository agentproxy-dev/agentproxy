"use client"

import { useState } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Trash2, Shield, Plus } from "lucide-react"
import { Alert, AlertDescription } from "@/components/ui/alert"

interface PoliciesConfigProps {
  policies: any[]
  addPolicy: (policy: any) => void
  removePolicy: (index: number) => void
}

export function PoliciesConfig({ policies, addPolicy, removePolicy }: PoliciesConfigProps) {
  const [key, setKey] = useState("sub")
  const [value, setValue] = useState("")
  const [resourceType, setResourceType] = useState("tool")
  const [resourceId, setResourceId] = useState("")
  const [matcherType, setMatcherType] = useState("equals")

  const handleAddPolicy = (e) => {
    e.preventDefault()

    const policy = {
      key,
      value,
      resource: {
        [resourceType]: {
          id: resourceId,
        },
      },
      matcher: {
        [matcherType]: {},
      },
    }

    addPolicy(policy)
    setValue("")
    setResourceId("")
  }

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <div className="lg:col-span-2">
        <Card>
          <CardHeader>
            <CardTitle>Security Policies</CardTitle>
            <CardDescription>Manage access control policies for your MCP proxy</CardDescription>
          </CardHeader>
          <CardContent>
            {policies.length > 0 ? (
              <div className="space-y-4">
                {policies.map((policy, index) => (
                  <Card key={index} id={`policy-${index}`} className="relative border border-muted">
                    <CardContent className="p-4">
                      <div className="flex justify-between items-start">
                        <div className="space-y-2 w-full">
                          <div className="flex items-center">
                            <Shield className="h-4 w-4 mr-2 text-muted-foreground" />
                            <h3 className="font-medium">Policy {index + 1}</h3>
                          </div>

                          <div className="grid grid-cols-1 md:grid-cols-2 gap-x-4 gap-y-2 text-sm">
                            <div>
                              <span className="font-medium">Key:</span> {policy.key}
                            </div>
                            <div>
                              <span className="font-medium">Value:</span> {policy.value}
                            </div>
                            <div>
                              <span className="font-medium">Resource Type:</span> {Object.keys(policy.resource)[0]}
                            </div>
                            <div>
                              <span className="font-medium">Resource ID:</span>{" "}
                              {policy.resource[Object.keys(policy.resource)[0]].id}
                            </div>
                            <div>
                              <span className="font-medium">Matcher:</span> {Object.keys(policy.matcher)[0]}
                            </div>
                          </div>
                        </div>

                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => removePolicy(index)}
                          className="text-muted-foreground hover:text-destructive"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            ) : (
              <Alert>
                <AlertDescription>
                  No policies configured. Add a policy to control access to your MCP proxy resources.
                </AlertDescription>
              </Alert>
            )}
          </CardContent>
        </Card>
      </div>

      <div>
        <Card id="add-policy-form">
          <CardHeader>
            <CardTitle>Add Policy</CardTitle>
            <CardDescription>Create a new access control policy</CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleAddPolicy} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="key">Key</Label>
                <Select value={key} onValueChange={setKey}>
                  <SelectTrigger id="key">
                    <SelectValue placeholder="Select key" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="sub">Subject (sub)</SelectItem>
                    <SelectItem value="iss">Issuer (iss)</SelectItem>
                    <SelectItem value="aud">Audience (aud)</SelectItem>
                    <SelectItem value="role">Role</SelectItem>
                    <SelectItem value="scope">Scope</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="value">Value</Label>
                <Input
                  id="value"
                  value={value}
                  onChange={(e) => setValue(e.target.value)}
                  placeholder="Value to match"
                  required
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="resourceType">Resource Type</Label>
                <Select value={resourceType} onValueChange={setResourceType}>
                  <SelectTrigger id="resourceType">
                    <SelectValue placeholder="Select resource type" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="tool">Tool</SelectItem>
                    <SelectItem value="target">Target</SelectItem>
                    <SelectItem value="api">API</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="resourceId">Resource ID</Label>
                <Input
                  id="resourceId"
                  value={resourceId}
                  onChange={(e) => setResourceId(e.target.value)}
                  placeholder="Resource identifier"
                  required
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="matcherType">Matcher Type</Label>
                <Select value={matcherType} onValueChange={setMatcherType}>
                  <SelectTrigger id="matcherType">
                    <SelectValue placeholder="Select matcher type" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="equals">Equals</SelectItem>
                    <SelectItem value="contains">Contains</SelectItem>
                    <SelectItem value="regex">Regex</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <Button type="submit" className="w-full">
                <Plus className="h-4 w-4 mr-2" />
                Add Policy
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
