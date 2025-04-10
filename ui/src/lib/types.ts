export interface Target {
  // The name of the target.
  name: string;

  // Only one of these fields will be set
  sse?: SseTarget;
  openapi?: OpenAPITarget;
  stdio?: StdioTarget;
}

export type TargetType = "sse" | "openapi" | "stdio";

export interface SseTarget {
  // The host of the target.
  host: string;
  // The port of the target.
  port: number;
  // The path of the target.
  path: string;
  // The headers of the target.
  headers?: { [key: string]: string };
}

export interface StdioTarget {
  // The command of the target.
  cmd: string;
  // The arguments of the target.
  args: string[];
  // The environment variables of the target.
  env: { [key: string]: string };
}

export interface LocalDataSource {
  // Only one of these fields will be set
  file_path?: string;
  inline?: Uint8Array; // For bytes in proto3, we use Uint8Array in TypeScript
}

export interface OpenAPITarget {
  // The host of the target.
  host: string;
  // The port of the target.
  port: number;
  // The schema of the target.
  schema: LocalDataSource;
}

export interface Listener {
  // SSE is the only listener we can configure through UI
  sse: SseListener;
}

export interface SseListener {
  address: string;
  port: number;
  tls?: SseListener.TlsConfig;
}

// Nested type definition
export namespace SseListener {
  export interface TlsConfig {
    key_pem: LocalDataSource;
    cert_pem: LocalDataSource;
  }
}

export interface StdioListener {
  // Empty interface as the message has no fields
}

// Enum for matcher types
export enum Matcher {
  // The value must be equal to the value in the claims.
  EQUALS = 0,
  CONTAINS = 1,
  STARTS_WITH = 2,
  ENDS_WITH = 3,
}

export enum ResourceType {
  TOOL = 0,
  PROMPT = 1,
  RESOURCE = 2,
}

export interface Rule {
  key: string;
  value: string;
  resource: {
    type: ResourceType;
    id: string;
  };
  matcher: Matcher;
}

export interface RBACConfig {
  name: string;
  namespace: string;
  rules: Rule[];
}

type ConfigType = "static";

export interface Config {
  // The type of the configuration.
  type: ConfigType;
  // The listeners for the configuration.
  listeners: Listener[];
  // The policies for the configuration.
  policies?: RBACConfig[];
  // The targets for the configuration.
  targets: Target[];
}