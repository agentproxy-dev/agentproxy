"use client"

import { ReactNode } from "react"
import { LoadingProvider, useLoading } from "@/lib/loading-context"
import { AppFooter } from "@/components/app-footer"

function FooterWrapper() {
  const { isLoading } = useLoading()
  
  if (isLoading) {
    return null
  }
  
  return <AppFooter />
}

export function LoadingWrapper({ children }: { children: ReactNode }) {
  return (
    <LoadingProvider>
      <main className="flex-1 overflow-auto">
        {children}
      </main>
      <FooterWrapper />
    </LoadingProvider>
  )
} 