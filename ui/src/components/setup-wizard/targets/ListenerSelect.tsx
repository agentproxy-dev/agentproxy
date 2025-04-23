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
  CommandList,
} from "@/components/ui/command";
import { Badge } from "@/components/ui/badge";
import { Check, ChevronsUpDown, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";

interface ListenerSelectProps {
  // undefined: all, string[]: specific (can be empty)
  selectedListeners: string[] | undefined;
  onListenersChange: (listeners: string[] | undefined) => void;
}

export function ListenerSelect({ selectedListeners, onListenersChange }: ListenerSelectProps) {
  const [allAvailableListeners, setAllAvailableListeners] = useState<Listener[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [openCombobox, setOpenCombobox] = useState(false);

  // --- State Checks ---
  const isAllSelected = selectedListeners === undefined;
  // Treat empty array as a specific (but empty) selection
  const isSpecificSelected = Array.isArray(selectedListeners);
  const specificSelectedListeners = isSpecificSelected ? selectedListeners : [];
  const specificSelectedSet = new Set(specificSelectedListeners);

  useEffect(() => {
    const loadListeners = async () => {
      try {
        setLoading(true);
        setError(null);
        const fetchedListeners = await fetchListeners();
        setAllAvailableListeners(fetchedListeners);

        // Prune selection if a selected listener no longer exists
        if (Array.isArray(selectedListeners)) {
          const availableNames = new Set(fetchedListeners.map((l) => l.name));
          const validSelection = selectedListeners.filter((name) => availableNames.has(name));
          // Only update if the array content actually changed
          if (validSelection.length !== selectedListeners.length) {
            onListenersChange(validSelection);
          }
        }

        if (fetchedListeners.length === 0 && selectedListeners === undefined) {
          onListenersChange([]);
        }
      } catch (err) {
        console.error("Error fetching listeners:", err);
        setError("Failed to load available listeners. Please ensure the proxy server is running.");
        onListenersChange([]);
      } finally {
        setLoading(false);
      }
    };

    loadListeners();
    // Intentionally excluding onListenersChange from deps to avoid loops on initial load/error
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSelect = (type: "all" | "specific", value?: string) => {
    if (type === "all") {
      onListenersChange(undefined);
      setOpenCombobox(false); // Close after selecting All
    } else if (type === "specific" && value) {
      let newSelection: string[];
      if (isAllSelected) {
        newSelection = [value];
      } else {
        const currentSelection = [...specificSelectedListeners];
        const index = currentSelection.indexOf(value);
        if (index > -1) {
          currentSelection.splice(index, 1); // Remove
        } else {
          currentSelection.push(value); // Add
        }
        newSelection = currentSelection;
      }
      onListenersChange(newSelection);
    }
  };

  const getButtonLabel = () => {
    if (isAllSelected) return "All Listeners";
    if (isSpecificSelected) {
      if (specificSelectedListeners.length === 0) {
        return <span className="text-muted-foreground">No Specific Listeners</span>;
      }
      // Show badges for selected listeners
      return (
        <div className="flex gap-1 flex-wrap mr-2 overflow-hidden">
          {specificSelectedListeners.slice(0, 3).map((listenerName) => (
            <Badge
              variant="secondary"
              key={listenerName}
              className="mr-1 mb-1"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                handleSelect("specific", listenerName);
              }}
            >
              {listenerName}
              <X className="ml-1 h-3 w-3 cursor-pointer" />
            </Badge>
          ))}
          {specificSelectedListeners.length > 3 && (
            <Badge variant="secondary" className="mr-1 mb-1">
              +{specificSelectedListeners.length - 3} more
            </Badge>
          )}
        </div>
      );
    }
    return "Select listeners...";
  };

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

  if (allAvailableListeners.length === 0 && !loading && !error) {
    if (!isSpecificSelected || specificSelectedListeners.length > 0) {
      onListenersChange([]);
    }
    return (
      <div className="space-y-2">
        <Label>Attach Target To Listeners</Label>
        <Alert variant="default">
          <AlertDescription>
            No listeners are available. Target cannot attach to any listeners.
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <Label>Attach Target To Listeners</Label>
      <Popover open={openCombobox} onOpenChange={setOpenCombobox}>
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            role="combobox"
            aria-expanded={openCombobox}
            className="w-full justify-between h-auto min-h-[2.5rem]" // Allow button to grow
            disabled={allAvailableListeners.length === 0} // Disable if no listeners fetched
          >
            <div className="flex-1 text-left">
              {" "}
              {/* Ensure label aligns left */}
              {getButtonLabel()}
            </div>
            <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-[--radix-popover-trigger-width] p-0">
          <Command>
            <CommandInput placeholder="Search listeners..." />
            <CommandList>
              <CommandEmpty>No listener found.</CommandEmpty>
              <CommandGroup>
                {/* Option for ALL listeners */}
                <CommandItem key="--all--" value="--all--" onSelect={() => handleSelect("all")}>
                  <Check
                    className={cn("mr-2 h-4 w-4", isAllSelected ? "opacity-100" : "opacity-0")}
                  />
                  All Listeners
                </CommandItem>
                {allAvailableListeners.length > 0 && <div className="my-1 h-px bg-muted" />}
                {allAvailableListeners.map((listener) => (
                  <CommandItem
                    key={listener.name}
                    value={listener.name}
                    onSelect={(currentValue) => handleSelect("specific", currentValue)}
                  >
                    <Check
                      className={cn(
                        "mr-2 h-4 w-4",
                        isSpecificSelected && specificSelectedSet.has(listener.name)
                          ? "opacity-100"
                          : "opacity-0"
                      )}
                    />
                    {listener.name}
                  </CommandItem>
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
      <p className="text-sm text-muted-foreground">
        {isAllSelected && "This target will process requests from all available listeners."}
        {isSpecificSelected &&
          specificSelectedListeners.length > 0 &&
          `This target will process requests only from the selected ${specificSelectedListeners.length} listener(s).`}
        {isSpecificSelected &&
          specificSelectedListeners.length === 0 &&
          "This target is not attached to any specific listeners."}
      </p>
    </div>
  );
}
