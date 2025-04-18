"use client";

import { useState, useEffect } from "react";
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
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"
import { Trash2, Plus, ChevronsUpDown, Check } from "lucide-react";
import { fetchMcpTargets, fetchA2aTargets } from "@/lib/api";
import { cn } from "@/lib/utils";

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
        target: rule.resource?.target || "",
        id: rule.resource?.id || "",
      },
      matcher: rule.matcher || "EQUALS",
    })) || []
  );
  const [allTargetNames, setAllTargetNames] = useState<string[]>([]);
  const [loadingTargets, setLoadingTargets] = useState(true);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [popoverOpenStates, setPopoverOpenStates] = useState<boolean[]>([]);

  useEffect(() => {
    const loadTargets = async () => {
      setLoadingTargets(true);
      setFetchError(null);
      try {
        const [mcpTargets, a2aTargets] = await Promise.all([
          fetchMcpTargets(),
          fetchA2aTargets(),
        ]);
        const mcpNames = mcpTargets.map((t) => t.name).filter(Boolean);
        const a2aNames = a2aTargets.map((t) => t.name).filter(Boolean);
        const uniqueNames = Array.from(new Set([...mcpNames, ...a2aNames]));
        setAllTargetNames(uniqueNames);
      } catch (error) {
        console.error("Failed to fetch targets:", error);
        setFetchError("Failed to load target list.");
      } finally {
        setLoadingTargets(false);
      }
    };
    loadTargets();
  }, []);

  useEffect(() => {
    setPopoverOpenStates(Array(rules.length).fill(false));
  }, [rules.length]);

  const setPopoverOpen = (index: number, open: boolean) => {
    setPopoverOpenStates(prev => prev.map((state, i) => i === index ? open : state));
  };

  const handleAddRule = () => {
    setRules([
      ...rules,
      {
        key: "",
        value: "",
        resource: {
          type: "TOOL",
          target: "",
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
                placeholder="e.g., echo-tool"
                onChange={(e) =>
                  handleUpdateRule(index, {
                    resource: { ...rule.resource, id: e.target.value },
                  })
                }
              />
            </div>

            <div className="space-y-1">
              <Label htmlFor={`resource-target-${index}`}>Resource Target</Label>
              <Popover open={popoverOpenStates[index]} onOpenChange={(open) => setPopoverOpen(index, open)}>
                <PopoverTrigger asChild>
                  <Button
                    variant="outline"
                    role="combobox"
                    aria-expanded={popoverOpenStates[index]}
                    className="w-full justify-between font-normal"
                    disabled={loadingTargets}
                  >
                    {rule.resource.target || (loadingTargets ? "Loading..." : "Select or type target...")}
                    <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-[--radix-popover-trigger-width] p-0">
                  <Command shouldFilter={false}>
                    <CommandInput 
                      placeholder="Search target or type name..." 
                      value={rule.resource.target} 
                      onValueChange={(search) => {
                        handleUpdateRule(index, { resource: { ...rule.resource, target: search } });
                      }}
                    />
                    <CommandList>
                      <CommandEmpty>{loadingTargets ? "Loading..." : (fetchError || "No target found.")}</CommandEmpty>
                      <CommandGroup heading="Suggestions">
                        {allTargetNames
                           .filter(name => name.toLowerCase().includes(rule.resource.target?.toLowerCase() ?? ''))
                           .map((name) => (
                          <CommandItem
                            key={name}
                            value={name}
                            onSelect={(currentValue) => {
                              handleUpdateRule(index, { resource: { ...rule.resource, target: currentValue } });
                              setPopoverOpen(index, false);
                            }}
                          >
                            <Check
                              className={cn(
                                "mr-2 h-4 w-4",
                                rule.resource.target === name ? "opacity-100" : "opacity-0"
                              )}
                            />
                            {name}
                          </CommandItem>
                        ))}
                      </CommandGroup>
                    </CommandList>
                  </Command>
                </PopoverContent>
              </Popover>
              {fetchError && <p className="text-xs text-destructive mt-1">{fetchError}</p>}
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
