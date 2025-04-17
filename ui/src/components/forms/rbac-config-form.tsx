"use client";

import { useState } from "react";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Listener, Rule, ResourceType, Matcher } from "@/lib/types";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Trash2, Plus } from "lucide-react";

interface RBACConfigFormProps {
  listener: Listener | null;
  onSave: (updatedListener: Listener) => void;
  onCancel: () => void;
}

export function RBACConfigForm({ listener, onSave, onCancel }: RBACConfigFormProps) {
  const [rules, setRules] = useState<Rule[]>(
    listener?.sse?.rbac?.[0]?.rules?.map((rule) => ({
      key: rule.key || "",
      value: rule.value || "",
      resource: {
        type: rule.resource?.type || "TOOL",
        id: rule.resource?.id || "",
      },
      matcher: rule.matcher || "EQUALS",
    })) || []
  );

  const handleAddRule = () => {
    setRules([
      ...rules,
      {
        key: "",
        value: "",
        resource: {
          type: "TOOL",
          id: "",
        },
        matcher: "EQUALS" as Matcher,
      },
    ]);
  };

  const handleRemoveRule = (index: number) => {
    setRules(rules.filter((_, i) => i !== index));
  };

  const handleUpdateRule = (index: number, updates: Partial<Rule>) => {
    setRules(rules.map((rule, i) => (i === index ? { ...rule, ...updates } : rule)));
  };

  const handleSave = () => {
    if (!listener) return;

    const updatedListener: Listener = {
      ...listener,
      sse: {
        ...listener.sse,
        rbac: [
          {
            name: "default",
            namespace: "default",
            rules,
          },
        ],
      },
    };

    onSave(updatedListener);
  };

  return (
    <div className="space-y-4 py-4">
      <div className="space-y-4">
        {rules.map((rule, index) => (
          <div key={index} className="space-y-4 p-4 border rounded-lg">
            <div className="flex justify-between items-center">
              <h4 className="text-sm font-medium">Rule {index + 1}</h4>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => handleRemoveRule(index)}
                className="text-destructive hover:text-destructive"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>

            <div className="space-y-2">
              <Label htmlFor={`rule-key-${index}`}>Claim Key</Label>
              <Input
                id={`rule-key-${index}`}
                value={rule.key}
                onChange={(e) => handleUpdateRule(index, { key: e.target.value })}
                placeholder="e.g., role"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor={`rule-value-${index}`}>Claim Value</Label>
              <Input
                id={`rule-value-${index}`}
                value={rule.value}
                onChange={(e) => handleUpdateRule(index, { value: e.target.value })}
                placeholder="e.g., admin"
              />
            </div>

            <div className="space-y-2">
              <Label>Resource Type</Label>
              <Select
                value={rule.resource.type.toString()}
                onValueChange={(value) =>
                  handleUpdateRule(index, {
                    resource: {
                      ...rule.resource,
                      type: value as ResourceType,
                    },
                  })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={"TOOL"}>Tool</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor={`resource-id-${index}`}>Resource ID</Label>
              <Input
                id={`resource-id-${index}`}
                value={rule.resource.id}
                onChange={(e) =>
                  handleUpdateRule(index, {
                    resource: {
                      ...rule.resource,
                      id: e.target.value,
                    },
                  })
                }
                placeholder="e.g., tool-id"
              />
            </div>

            <div className="space-y-2">
              <Label>Matcher</Label>
              <Select
                value={rule.matcher}
                onValueChange={(value: Matcher) =>
                  handleUpdateRule(index, {
                    matcher: value,
                  })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="EQUALS">Equals</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        ))}
      </div>

      <Button variant="outline" className="w-full" onClick={handleAddRule}>
        <Plus className="h-4 w-4 mr-2" />
        Add Rule
      </Button>

      <div className="flex justify-end space-x-2 pt-4">
        <Button variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button onClick={handleSave}>Save Changes</Button>
      </div>
    </div>
  );
}
