import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Trash2, Plus } from "lucide-react";
import { handleArrayInput, formatArrayForInput, ensurePort } from "@/lib/policy-utils";

interface ArrayInputProps {
  id: string;
  label: string;
  value: any;
  onChange: (array: string[]) => void;
  placeholder?: string;
}

/**
 * Array input component with live/blur handling for comma-separated values
 */
export const ArrayInput = ({ id, label, value, onChange, placeholder }: ArrayInputProps) => (
  <div className="space-y-3">
    <Label htmlFor={id}>{label}</Label>
    <Input
      id={id}
      value={formatArrayForInput(value)}
      onChange={(e) => onChange(e.target.value as any)} // Allow string during typing
      onBlur={(e) => onChange(handleArrayInput(e.target.value))}
      placeholder={placeholder}
    />
  </div>
);

interface TargetInputProps {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  required?: boolean;
}

/**
 * Target input component with automatic port defaulting
 */
export const TargetInput = ({
  id,
  label,
  value,
  onChange,
  placeholder,
  required = false,
}: TargetInputProps) => (
  <div className="space-y-3">
    <Label htmlFor={id}>
      {label} {required && "*"}
    </Label>
    <Input
      id={id}
      value={value || ""}
      onChange={(e) => onChange(e.target.value)}
      onBlur={(e) => {
        const val = e.target.value.trim();
        if (val && !val.includes(":")) {
          onChange(ensurePort(val));
        }
      }}
      placeholder={placeholder}
    />
  </div>
);

interface KeyValueManagerProps {
  title: string;
  description?: string;
  data: Record<string, any>;
  onChange: (data: Record<string, any>) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
  addButtonText?: string;
}

/**
 * Generic key-value pair management component
 */
export const KeyValueManager = ({
  title,
  description,
  data,
  onChange,
  keyPlaceholder = "Key",
  valuePlaceholder = "Value",
  addButtonText = "Add Item",
}: KeyValueManagerProps) => {
  const addItem = () => {
    const newData = { ...data };
    newData[`key${Object.keys(newData).length + 1}`] = "";
    onChange(newData);
  };

  const removeItem = (key: string) => {
    const newData = { ...data };
    delete newData[key];
    onChange(newData);
  };

  const updateKey = (oldKey: string, newKey: string) => {
    const newData = { ...data };
    const value = newData[oldKey];
    delete newData[oldKey];
    newData[newKey] = value;
    onChange(newData);
  };

  const updateValue = (key: string, value: string) => {
    const newData = { ...data };
    newData[key] = value;
    onChange(newData);
  };

  return (
    <div className="space-y-3">
      <Label>{title}</Label>
      {description && <p className="text-sm text-muted-foreground">{description}</p>}
      {Object.entries(data || {}).map(([key, value], index) => (
        <div key={index} className="flex space-x-2">
          <Input
            placeholder={keyPlaceholder}
            value={key}
            onChange={(e) => updateKey(key, e.target.value)}
          />
          <Input
            placeholder={valuePlaceholder}
            value={value as string}
            onChange={(e) => updateValue(key, e.target.value)}
          />
          <Button variant="ghost" size="sm" onClick={() => removeItem(key)}>
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      ))}
      <Button variant="outline" size="sm" onClick={addItem}>
        <Plus className="h-4 w-4 mr-2" />
        {addButtonText}
      </Button>
    </div>
  );
};

interface HeaderPairListProps {
  title: string;
  headers: [string, string][];
  onChange: (headers: [string, string][]) => void;
  buttonText: string;
  namePlaceholder?: string;
  valuePlaceholder?: string;
}

/**
 * Component for managing header name-value pairs
 */
export const HeaderPairList = ({
  title,
  headers,
  onChange,
  buttonText,
  namePlaceholder = "Header name",
  valuePlaceholder = "Header value",
}: HeaderPairListProps) => (
  <div className="space-y-3">
    <Label>{title}</Label>
    <div className="space-y-2">
      {headers.map((header, index) => (
        <div key={index} className="flex items-center space-x-2">
          <Input
            value={header[0] || ""}
            onChange={(e) => {
              const newHeaders = [...headers];
              newHeaders[index] = [e.target.value, header[1] || ""];
              onChange(newHeaders);
            }}
            placeholder={namePlaceholder}
            className="flex-1"
          />
          <Input
            value={header[1] || ""}
            onChange={(e) => {
              const newHeaders = [...headers];
              newHeaders[index] = [header[0] || "", e.target.value];
              onChange(newHeaders);
            }}
            placeholder={valuePlaceholder}
            className="flex-1"
          />
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              const newHeaders = [...headers];
              newHeaders.splice(index, 1);
              onChange(newHeaders);
            }}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      ))}
      <Button variant="outline" size="sm" onClick={() => onChange([...headers, ["", ""]])}>
        <Plus className="h-4 w-4 mr-2" />
        {buttonText}
      </Button>
    </div>
  </div>
);

interface StringListProps {
  title: string;
  items: string[];
  onChange: (items: string[]) => void;
  buttonText: string;
  placeholder?: string;
}

/**
 * Component for managing simple string arrays
 */
export const StringList = ({
  title,
  items,
  onChange,
  buttonText,
  placeholder = "Enter value",
}: StringListProps) => (
  <div className="space-y-3">
    <Label>{title}</Label>
    <div className="space-y-2">
      {items.map((item, index) => (
        <div key={index} className="flex items-center space-x-2">
          <Input
            value={item || ""}
            onChange={(e) => {
              const newItems = [...items];
              newItems[index] = e.target.value;
              onChange(newItems);
            }}
            placeholder={placeholder}
            className="flex-1"
          />
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              const newItems = [...items];
              newItems.splice(index, 1);
              onChange(newItems);
            }}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      ))}
      <Button variant="outline" size="sm" onClick={() => onChange([...items, ""])}>
        <Plus className="h-4 w-4 mr-2" />
        {buttonText}
      </Button>
    </div>
  </div>
);
