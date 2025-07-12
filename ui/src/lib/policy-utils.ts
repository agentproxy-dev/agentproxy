/**
 * Helper function to handle comma-separated array input
 */
export const handleArrayInput = (value: string): string[] => {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
};

/**
 * Helper function to format array as comma-separated string
 */
export const formatArrayForInput = (arr: any): string => {
  return Array.isArray(arr) ? arr.join(", ") : arr || "";
};

/**
 * Helper function to add default port if not present
 */
export const ensurePort = (value: string, defaultPort: string = "80"): string => {
  return value && !value.includes(":") ? `${value}:${defaultPort}` : value;
};

/**
 * Helper function to handle number array input (e.g., for HTTP status codes)
 */
export const handleNumberArrayInput = (value: string): number[] => {
  return value
    .split(",")
    .map((s) => parseInt(s.trim()))
    .filter((n) => !isNaN(n));
};
