import { z } from "zod";

export interface Target {
  // The name of the target.
  name: string;

  // The listeners which are allowed to connect to the target.
  listeners?: string[];

  // Only one of these fields will be set
  sse?: SseTarget;
  openapi?: OpenAPITarget;
  stdio?: StdioTarget;
  a2a?: A2aTarget;
}

export type TargetType = "mcp" | "sse" | "openapi" | "stdio" | "a2a" | "unknown";

export interface SseTarget {
  // The host of the target.
  host: string;
  // The port of the target.
  port: number;
  // The path of the target.
  path: string;
  // The headers of the target.
  headers?: Header[];
  // The auth of the target.
  auth?: BackendAuth;
  // The tls of the target.
  tls?: BackendTls;
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
  // The auth of the target.
  auth?: BackendAuth;
  // The tls of the target.
  tls?: BackendTls;
  // The headers of the target.
  headers?: Header[];
}

export interface A2aTarget {
  // The host of the target.
  host: string;
  // The port of the target.
  port: number;
  // The path of the target.
  path: string;
  // The headers of the target.
  headers?: Header[];
  // The auth of the target.
  auth?: BackendAuth;
  // The tls of the target.
  tls?: BackendTls;
}

export interface Header {
  key: string;
  value: {
    string_value?: string;
    env_value?: string;
  };
}

export interface BackendAuth {
  passthrough?: boolean;
}

export interface BackendTls {
  insecure_skip_verify: boolean;
}

export interface Listener {
  // The name of the listener
  name: string;
  // SSE is the only listener we can configure through UI
  sse: SseListener;
  // The policies attached to this listener
  policies?: RBACConfig[];
  protocol: ListenerProtocol;
}

export enum ListenerProtocol {
  MCP = "mcp",
  A2A = "a2a",
}

export interface RemoteDataSource {
  url: string;
}

export interface JwtConfig {
  issuer: string[];
  audience: string[];
  localJwks?: LocalDataSource;
  remoteJwks?: RemoteDataSource;
}

export interface Authn {
  jwt: JwtConfig;
}

export interface SseListener {
  // The address can be either 'address' or 'host'
  address?: string;
  host?: string;
  port: number;
  tls?: TlsConfig;
  authn?: Authn;
  // RBAC configuration is now part of the listener
  rbac?: RuleSet[];
}

export interface TlsConfig {
  key_pem: LocalDataSource;
  cert_pem: LocalDataSource;
}

export interface StdioListener {
  // Empty interface as the message has no fields
  // eslint-disable-next-line @typescript-eslint/no-empty-object-type
}

// Enum for matcher types
export type Matcher =
  // The value must be equal to the value in the claims.
  "EQUALS";
//"CONTAINS" |
//"STARTS_WITH" |
//"ENDS_WITH"

export type ResourceType = "TOOL"; //|
// "PROMPT" |
// "RESOURCE"

export interface Rule {
  key: string;
  value: string;
  resource: {
    type: ResourceType;
    target: string;
    id: string;
  };
  matcher: Matcher;
}

export interface RBACConfig {
  name: string;
  namespace: string;
  rules: Rule[];
}

export interface RuleSet {
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
  targets: TargetWithType[];
}

export type TargetWithType = Target & { type: "mcp" | "a2a" | "openapi" | "stdio" | "sse" };

// Schema specifically for Playground UI representation, might differ from backend config Listener
export const PlaygroundListenerSchema = z.object({
  name: z.string(),
  protocol: z.nativeEnum(ListenerProtocol).optional(),
  sse: z
    .object({
      port: z.number(),
      host: z.string(),
      tls: z.boolean().optional(),
    })
    .optional(),
});

// Type derived from the Playground-specific schema
export type PlaygroundListener = z.infer<typeof PlaygroundListenerSchema>;

// Type for listener info including protocol, extending the original Listener type
// It combines the config Listener with the derived displayEndpoint for UI purposes
export interface ListenerInfo extends Listener {
  displayEndpoint: string;
}
