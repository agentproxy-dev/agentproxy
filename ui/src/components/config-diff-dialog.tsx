"use client"

import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Config } from "@/lib/types"
import { Loader2 } from "lucide-react"

interface ConfigDiffDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentConfig: Config | null
  newConfig: Config
  onConfirm: () => void
  isPushing: boolean
}

export function ConfigDiffDialog({
  open,
  onOpenChange,
  currentConfig,
  newConfig,
  onConfirm,
  isPushing,
}: ConfigDiffDialogProps) {
  const renderDiff = () => {
    if (!currentConfig) {
      return (
        <div className="space-y-4">
          <h4 className="font-medium">New Configuration</h4>
          <pre className="bg-muted p-4 rounded-md overflow-auto max-h-[400px] text-sm">
            {JSON.stringify(newConfig, null, 2)}
          </pre>
        </div>
      )
    }

    return (
      <div className="space-y-4">
        <div>
          <h4 className="font-medium mb-2">Current Configuration</h4>
          <pre className="bg-muted p-4 rounded-md overflow-auto max-h-[200px] text-sm">
            {JSON.stringify(currentConfig, null, 2)}
          </pre>
        </div>
        <div>
          <h4 className="font-medium mb-2">New Configuration</h4>
          <pre className="bg-muted p-4 rounded-md overflow-auto max-h-[200px] text-sm">
            {JSON.stringify(newConfig, null, 2)}
          </pre>
        </div>
      </div>
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>Review Configuration Changes</DialogTitle>
          <DialogDescription>
            Please review the changes before pushing the configuration to the proxy.
          </DialogDescription>
        </DialogHeader>

        <div className="mt-4">{renderDiff()}</div>

        <div className="flex justify-end gap-2 mt-4">
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isPushing}>
            Cancel
          </Button>
          <Button onClick={onConfirm} disabled={isPushing}>
            {isPushing ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Pushing...
              </>
            ) : (
              "Push Configuration"
            )}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
} 