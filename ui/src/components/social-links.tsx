"use client";

import { Github, MessageSquare, Globe } from "lucide-react";
import Link from "next/link";
import { motion } from "framer-motion";

interface SocialLinksProps {
  className?: string;
  animated?: boolean;
}

export function SocialLinks({ className = "", animated = true }: SocialLinksProps) {
  const links = [
    {
      href: "https://github.com/placeholder/agentproxy",
      icon: Github,
      label: "GitHub",
    },
    {
      href: "https://discord.gg/placeholder",
      icon: MessageSquare,
      label: "Discord",
    },
    {
      href: "https://agentproxy.example.com",
      icon: Globe,
      label: "Website",
    },
  ];

  const Container = animated ? motion.div : "div";

  return (
    <Container
      className={`flex justify-center space-x-6 ${className}`}
      {...(animated
        ? {
            initial: { opacity: 0 },
            animate: { opacity: 1 },
            transition: { duration: 0.5, delay: 0.4 },
          }
        : {})}
    >
      {links.map((link) => (
        <Link
          key={link.label}
          href={link.href}
          target="_blank"
          rel="noopener noreferrer"
          className="text-muted-foreground hover:text-primary transition-colors"
        >
          <link.icon className="h-5 w-5" />
          <span className="sr-only">{link.label}</span>
        </Link>
      ))}
    </Container>
  );
}
