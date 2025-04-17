"use client";

import { useServer } from "@/lib/server-context";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, ExternalLink, ChevronDown, ChevronUp } from "lucide-react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Listener, RuleSet, Rule } from "@/lib/types";
import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface PolicyWithListeners extends RuleSet {
  listeners: Array<{
    name: string;
  }>;
}

export default function PoliciesPage() {
  const { listeners, connectionError } = useServer();
  const [selectedPolicy, setSelectedPolicy] = useState<PolicyWithListeners | null>(null);

  // Collect all unique policies across listeners
  const policiesMap = new Map<string, PolicyWithListeners>();
  listeners?.forEach((listener: Listener) => {
    const policies = listener.sse?.rbac || [];
    policies.forEach((policy: RuleSet) => {
      if (!policiesMap.has(policy.name)) {
        policiesMap.set(policy.name, {
          ...policy,
          listeners: [{ name: listener.name }],
        });
      } else {
        const existingPolicy = policiesMap.get(policy.name);
        if (existingPolicy) {
          existingPolicy.listeners.push({
            name: listener.name,
          });
        }
      }
    });
  });

  const allPolicies = Array.from(policiesMap.values());
  const handlePolicyClick = (policy: PolicyWithListeners) => {
    setSelectedPolicy(selectedPolicy?.name === policy.name ? null : policy);
  };

  return (
    <div className="container mx-auto py-6">
      {connectionError ? (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{connectionError}</AlertDescription>
        </Alert>
      ) : (
        <div className="space-y-6">
          <div className="flex justify-between items-center">
            <div>
              <h1 className="text-2xl font-bold tracking-tight">Policies Overview</h1>
              <p className="text-lg text-muted-foreground mt-1">
                View and manage security policies across all listeners
              </p>
            </div>
          </div>

          {allPolicies.length === 0 ? (
            <Alert>
              <AlertDescription>
                No policies configured yet. Add policies through the listener configuration.
              </AlertDescription>
            </Alert>
          ) : (
            <div className="space-y-6">
              <div className="border rounded-lg">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Policy Name</TableHead>
                      <TableHead>Rules</TableHead>
                      <TableHead>Applied To</TableHead>
                      <TableHead className="text-right">Configure</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {allPolicies.map((policy) => (
                      <TableRow
                        key={policy.name}
                        className="cursor-pointer hover:bg-muted/50"
                        onClick={() => handlePolicyClick(policy)}
                      >
                        <TableCell className="font-medium">
                          <div className="flex items-center">
                            {selectedPolicy?.name === policy.name ? (
                              <ChevronUp className="h-4 w-4 mr-2" />
                            ) : (
                              <ChevronDown className="h-4 w-4 mr-2" />
                            )}
                            {policy.name}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary">{policy.rules?.length || 0} rules</Badge>
                        </TableCell>
                        <TableCell>
                          <div className="flex flex-wrap gap-2">
                            {policy.listeners.map((listener) => (
                              <Badge key={listener.name} variant="outline">
                                {listener.name}
                              </Badge>
                            ))}
                          </div>
                        </TableCell>
                        <TableCell className="text-right">
                          {policy.listeners.map((listener, idx) =>
                            listener.name ? (
                              <Link
                                key={idx}
                                href={`/listeners/${listener.name}`}
                                className="inline-block"
                                onClick={(e) => e.stopPropagation()}
                              >
                                <Button variant="ghost" size="sm">
                                  <ExternalLink className="h-4 w-4 mr-2" />
                                  Configure
                                </Button>
                              </Link>
                            ) : null
                          )}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>

              {selectedPolicy && (
                <Card>
                  <CardHeader>
                    <CardTitle>Policy Details: {selectedPolicy.name}</CardTitle>
                    <CardDescription>View the rules for this policy</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-4">
                      <div>
                        <h3 className="text-sm font-medium mb-2">Rules</h3>
                        <div className="space-y-2">
                          {selectedPolicy.rules.map((rule: Rule, index: number) => (
                            <div key={index} className="border rounded-md p-3">
                              <div className="grid grid-cols-2 gap-4">
                                <div>
                                  <span className="text-sm font-medium">Key:</span>
                                  <span className="ml-2">{rule.key}</span>
                                </div>
                                <div>
                                  <span className="text-sm font-medium">Value:</span>
                                  <span className="ml-2">{rule.value}</span>
                                </div>
                                <div>
                                  <span className="text-sm font-medium">Resource Type:</span>
                                  <span className="ml-2">{rule.resource.type}</span>
                                </div>
                                <div>
                                  <span className="text-sm font-medium">Resource ID:</span>
                                  <span className="ml-2">{rule.resource.id}</span>
                                </div>
                                <div>
                                  <span className="text-sm font-medium">Matcher:</span>
                                  <span className="ml-2">{rule.matcher}</span>
                                </div>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                      <div>
                        <h3 className="text-sm font-medium mb-2">Applied to Listeners</h3>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
