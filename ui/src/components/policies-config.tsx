"use client"

import { useState } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Trash2, Shield, Plus } from "lucide-react"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog"
import { RBACConfig, Rule, Matcher, ResourceType } from "@/lib/types"

interface PoliciesConfigProps {
  policies: RBACConfig[]
  addPolicy: (policy: RBACConfig) => void
  removePolicy: (index: number) => void
}

export function PoliciesConfig({ policies, addPolicy, removePolicy }: PoliciesConfigProps) {
  const [isAddingPolicy, setIsAddingPolicy] = useState(false)
  const [policyToDelete, setPolicyToDelete] = useState<number | null>(null)
  const [name, setName] = useState("")
  const [namespace, setNamespace] = useState("")
  const [key, setKey] = useState("sub")
  const [value, setValue] = useState("")
  const [resourceType, setResourceType] = useState<ResourceType>(ResourceType.TOOL)
  const [resourceId, setResourceId] = useState("")

  const handleAddPolicy = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()

    const rule: Rule = {
      key,
      value,
      resource: {
        id: resourceId,
        type: resourceType,
      },
      matcher: Matcher.EQUALS,
    }

    const policy: RBACConfig = {
      name,
      namespace,
      rules: [rule],
    }

    addPolicy(policy)
    resetForm()
    setIsAddingPolicy(false)
  }

  const resetForm = () => {
    setName("")
    setNamespace("")
    setKey("sub")
    setValue("")
    setResourceType(ResourceType.TOOL)
    setResourceId("")
  }

  const handleDeletePolicy = (index: number) => {
    setPolicyToDelete(index)
  }

  const confirmDelete = () => {
    if (policyToDelete !== null) {
      removePolicy(policyToDelete)
      setPolicyToDelete(null)
    }
  }

  const cancelDelete = () => {
    setPolicyToDelete(null)
  }

  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h3 className="text-lg font-medium mb-2">Security Policies</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Configure access control policies for your MCP proxy
        </p>
      </div>

      {policies.length === 0 ? (
        <Alert>
          <AlertDescription>
            No policies configured. Add a policy to control access to your MCP proxy resources.
          </AlertDescription>
        </Alert>
      ) : (
        <div className="space-y-4">
          {policies.map((policy, index) => (
            <Card key={index} id={`policy-${index}`} className="relative border border-muted">
              <CardContent className="p-4">
                <div className="flex justify-between items-start">
                  <div className="space-y-2 w-full">
                    <div className="flex items-center">
                      <Shield className="h-4 w-4 mr-2 text-muted-foreground" />
                      <h3 className="font-medium">{policy.name}</h3>
                    </div>

                    <div className="grid grid-cols-1 md:grid-cols-2 gap-x-4 gap-y-2 text-sm">
                      <div>
                        <span className="font-medium">Namespace:</span> {policy.namespace}
                      </div>
                      {policy.rules.map((rule, ruleIndex) => (
                        <div key={ruleIndex} className="col-span-2 border-t pt-2 mt-2">
                          <div>
                            <span className="font-medium">Key:</span> {rule.key}
                          </div>
                          <div>
                            <span className="font-medium">Value:</span> {rule.value}
                          </div>
                          <div>
                            <span className="font-medium">Resource Type:</span> {ResourceType[rule.resource.type]}
                          </div>
                          <div>
                            <span className="font-medium">Resource ID:</span> {rule.resource.id}
                          </div>
                          <div>
                            <span className="font-medium">Matcher:</span> {Matcher[rule.matcher]}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>

                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => handleDeletePolicy(index)}
                    className="text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <Button onClick={() => setIsAddingPolicy(true)} className="flex items-center">
        <Plus className="h-4 w-4 mr-2" />
        Add Policy
      </Button>

      <Dialog open={isAddingPolicy} onOpenChange={setIsAddingPolicy}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Policy</DialogTitle>
            <DialogDescription>
              Create a new access control policy for your MCP proxy
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleAddPolicy} className="space-y-4 mt-6">
            <div className="space-y-2">
              <Label htmlFor="name">Policy Name</Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Enter policy name"
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="namespace">Namespace</Label>
              <Input
                id="namespace"
                value={namespace}
                onChange={(e) => setNamespace(e.target.value)}
                placeholder="Enter namespace"
                required
              />
            </div>

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
              <Select 
                value={ResourceType[resourceType]} 
                onValueChange={(value) => setResourceType(ResourceType[value as keyof typeof ResourceType])}
              >
                <SelectTrigger id="resourceType">
                  <SelectValue placeholder="Select resource type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="TOOL">Tool</SelectItem>
                  <SelectItem value="PROMPT">Prompt</SelectItem>
                  <SelectItem value="RESOURCE">Resource</SelectItem>
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

            <div className="flex justify-end">
              <Button type="submit">Add Policy</Button>
            </div>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={policyToDelete !== null} onOpenChange={(open) => !open && cancelDelete()}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Policy</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this policy? This action cannot be undone.
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
