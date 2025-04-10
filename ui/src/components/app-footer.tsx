"use client"

import { Github, Globe, MessageSquare } from "lucide-react"
import { Button } from "@/components/ui/button"

export function AppFooter() {
  return (
    <footer className="border-t py-4 px-6">
      <div className="flex items-center justify-center space-x-4">
        <Button
          variant="ghost"
          size="sm"
          className="text-muted-foreground hover:text-foreground"
          onClick={() => window.open("https://github.com/your-org/mcp-proxy", "_blank")}
        >
          <Github className="h-4 w-4 mr-2" />
          GitHub
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="text-muted-foreground hover:text-foreground"
          onClick={() => window.open("https://your-website.com", "_blank")}
        >
          <Globe className="h-4 w-4 mr-2" />
          Website
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="text-muted-foreground hover:text-foreground"
          onClick={() => window.open("https://discord.gg/your-server", "_blank")}
        >
          <MessageSquare className="h-4 w-4 mr-2" />
          Discord
        </Button>
      </div>
    </footer>
  )
} 