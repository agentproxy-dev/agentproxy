import { useEffect, useState } from "react";
import { Listener } from "@/lib/types";
import { Label } from "@/components/ui/label";
import { fetchListeners } from "@/lib/api";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
} from "@/components/ui/command";
import { Badge } from "@/components/ui/badge";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";

interface ListenerSelectProps {
  selectedListeners: string[];
  onListenersChange: (listeners: string[]) => void;
}

export function ListenerSelect({ selectedListeners, onListenersChange }: ListenerSelectProps) {
  const [listeners, setListeners] = useState<Listener[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const loadListeners = async () => {
      try {
        setLoading(true);
        const fetchedListeners = await fetchListeners();
        setListeners(fetchedListeners);
      } catch (err) {
        console.error("Error fetching listeners:", err);
        setError("Failed to load listeners");
      } finally {
        setLoading(false);
      }
    };

    loadListeners();
  }, []);

  if (loading) {
    return <div className="text-sm text-muted-foreground">Loading listeners...</div>;
  }

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <Label>Listeners *</Label>
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger asChild>
            <Button
              variant="outline"
              role="combobox"
              aria-expanded={open}
              className="w-full justify-between"
            >
              {selectedListeners.length > 0 ? (
                <div className="flex gap-1 flex-wrap">
                  {selectedListeners.map((listener) => (
                    <Badge
                      variant="secondary"
                      key={listener}
                      className="mr-1"
                      onClick={(e) => {
                        e.stopPropagation();
                        onListenersChange(selectedListeners.filter((l) => l !== listener));
                      }}
                    >
                      {listener}
                      <X className="ml-1 h-3 w-3" />
                    </Badge>
                  ))}
                </div>
              ) : (
                "Select listeners..."
              )}
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-full p-0">
            <Command>
              <CommandInput placeholder="Search listeners..." className="h-9" />
              <CommandEmpty>No listener found.</CommandEmpty>
              <CommandGroup>
                <CommandItem
                  onSelect={() => {
                    const allSelected = selectedListeners.length === listeners.length;
                    if (allSelected) {
                      onListenersChange([]);
                    } else {
                      onListenersChange(listeners.map((l) => l.name));
                    }
                  }}
                >
                  <div
                    className={cn(
                      "mr-2 flex h-4 w-4 items-center justify-center rounded-sm border border-primary",
                      selectedListeners.length === listeners.length
                        ? "bg-primary text-primary-foreground"
                        : "opacity-50 [&_svg]:invisible"
                    )}
                  >
                    <X className={cn("h-4 w-4")} />
                  </div>
                  Select All
                </CommandItem>
                <CommandItem className="h-px bg-muted" />
                {listeners.map((listener) => (
                  <CommandItem
                    key={listener.name}
                    onSelect={() => {
                      if (selectedListeners.includes(listener.name)) {
                        onListenersChange(selectedListeners.filter((l) => l !== listener.name));
                      } else {
                        onListenersChange([...selectedListeners, listener.name]);
                      }
                    }}
                  >
                    <div
                      className={cn(
                        "mr-2 flex h-4 w-4 items-center justify-center rounded-sm border border-primary",
                        selectedListeners.includes(listener.name)
                          ? "bg-primary text-primary-foreground"
                          : "opacity-50 [&_svg]:invisible"
                      )}
                    >
                      <X className={cn("h-4 w-4")} />
                    </div>
                    {listener.name}
                  </CommandItem>
                ))}
              </CommandGroup>
            </Command>
          </PopoverContent>
        </Popover>
        {selectedListeners.length === 0 && (
          <p className="text-sm text-destructive">Please select at least one listener</p>
        )}
      </div>
    </div>
  );
}
