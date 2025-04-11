"use client";

import * as React from "react";
import { Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

interface ThemeToggleProps {
  asChild?: boolean;
  className?: string;
}

export function ThemeToggle({ asChild, className }: ThemeToggleProps) {
  const { theme, setTheme } = useTheme();
  const Comp = asChild ? "div" : Button;

  const toggleTheme = () => setTheme(theme === "light" ? "dark" : "light");

  const content = (
    <>
      <Sun className="h-4 w-4 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
      <Moon className="absolute h-4 w-4 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
      <span className="sr-only">Toggle theme</span>
    </>
  );

  if (asChild) {
    return (
      <div
        className={className}
        onClick={toggleTheme}
        role="button"
        tabIndex={0}
        onKeyDown={e => {
          if (e.key === "Enter" || e.key === " ") {
            toggleTheme();
          }
        }}
      >
        {content}
      </div>
    );
  }

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Comp variant="ghost" size="icon" onClick={toggleTheme} className={className}>
            {content}
          </Comp>
        </TooltipTrigger>
        <TooltipContent>
          <p>Toggle theme</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
