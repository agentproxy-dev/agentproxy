"use client"

import { Alert, AlertDescription } from "@/components/ui/alert"
import { AlertCircle } from "lucide-react"
import { ConnectionForm } from "@/components/connection-form"
import { MCPLogo } from "@/components/mcp-logo"
import { motion } from "framer-motion"

interface NotConnectedStateProps {
  onConnect: (address: string, port: number) => Promise<boolean>
  connectionError: string
}

export function NotConnectedState({ onConnect, connectionError }: NotConnectedStateProps) {
  return (
    <div className="flex-1 flex flex-col items-center justify-center p-6 bg-gradient-to-br from-background via-background/95 to-muted/30 min-h-screen">
      <div className="w-full max-w-md mx-auto space-y-10">
        <motion.div 
          className="text-center"
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
        >
          <div className="mb-6">
            <MCPLogo className="h-16 w-auto mx-auto" />
          </div>
          <p className="text-sm text-muted-foreground max-w-xs mx-auto">Enter the address and port of your MCP proxy server to get started</p>
        </motion.div>
        
        <motion.div 
          className="space-y-4"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
        >
          <div className="bg-card/50 backdrop-blur-sm p-6 rounded-lg border border-border/50 shadow-sm">
            <ConnectionForm onConnect={onConnect} />
          </div>
          
          {connectionError && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              transition={{ duration: 0.3 }}
            >
              <Alert variant="destructive" className="mt-4">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{connectionError}</AlertDescription>
              </Alert>
            </motion.div>
          )}
        </motion.div>
              </div>
    </div>
  )
} 