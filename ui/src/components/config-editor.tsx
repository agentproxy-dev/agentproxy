"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Clipboard, Download, Check } from "lucide-react";
import { Config } from "@/lib/types";

interface ConfigEditorProps {
  config: Config;
  setConfig: (config: Config) => void;
}

export function ConfigEditor({ config, setConfig }: ConfigEditorProps) {
  const [jsonText, setJsonText] = useState("");
  const [isValid, setIsValid] = useState(true);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    try {
      setJsonText(JSON.stringify(config, null, 2));
      setIsValid(true);
    } catch {
      setIsValid(false);
    }
  }, [config]);

  const handleChange = (_e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setJsonText(_e.target.value);
    try {
      const parsed = JSON.parse(_e.target.value);
      setConfig(parsed);
      setIsValid(true);
      setError(null);
    } catch {
      setError("Invalid JSON");
    }
  };

  const copyToClipboard = () => {
    navigator.clipboard.writeText(jsonText);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const downloadConfig = () => {
    const blob = new Blob([jsonText], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "mcp-proxy-config.json";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-end space-x-2 mb-2">
        <Button
          variant="outline"
          size="sm"
          onClick={copyToClipboard}
          className="flex items-center gap-1"
        >
          {copied ? <Check className="h-4 w-4" /> : <Clipboard className="h-4 w-4" />}
          {copied ? "Copied" : "Copy"}
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={downloadConfig}
          className="flex items-center gap-1"
        >
          <Download className="h-4 w-4" />
          Download
        </Button>
      </div>

      <Card className={`border-2 ${!isValid ? "border-red-500" : ""}`}>
        <textarea
          value={jsonText}
          onChange={handleChange}
          className="w-full h-[500px] p-4 font-mono text-sm bg-transparent outline-none resize-none"
          spellCheck="false"
        />
      </Card>

      {error && <p className="text-red-500 text-sm">{error}</p>}
    </div>
  );
}
