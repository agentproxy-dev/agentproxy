"use client";

import { useServer } from "@/lib/server-context";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, ChevronDown, ChevronUp } from "lucide-react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
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
    <div className="container mx-auto py-8 px-4">
      {connectionError ? (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{connectionError}</AlertDescription>
        </Alert>
      ) : (
        <div className="space-y-6">
          <div className="flex justify-between items-center mb-6">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Policies Overview</h1>
              <p className="text-muted-foreground mt-1">
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
                    <h3 className="text-lg font-medium mb-4">Rules</h3>
                    {selectedPolicy.rules && selectedPolicy.rules.length > 0 ? (
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead>Key</TableHead>
                            <TableHead>Value</TableHead>
                            <TableHead>Resource Type</TableHead>
                            <TableHead>Resource ID</TableHead>
                            <TableHead>Target</TableHead>
                            <TableHead>Matcher</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {selectedPolicy.rules.map((rule: Rule, index: number) => (
                            <TableRow key={index}>
                              <TableCell>{rule.key}</TableCell>
                              <TableCell>{rule.value}</TableCell>
                              <TableCell>
                                <Badge variant="outline">{rule.resource.type}</Badge>
                              </TableCell>
                              <TableCell>{rule.resource.id || "-"}</TableCell>
                              <TableCell>{rule.resource.target || "-"}</TableCell>
                              <TableCell>{rule.matcher || "EQUALS"}</TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    ) : (
                      <p className="text-sm text-muted-foreground">
                        No rules defined for this policy.
                      </p>
                    )}
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
