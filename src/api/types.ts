export interface CredentialField {
  name: string
  display_name: string
  type: 'string' | 'secret'
  required: boolean
}

export interface CredentialType {
  type: string
  display_name: string
  fields: CredentialField[]
  supports_multiple: boolean
}

export interface CredentialMetadata {
  type: string
  label: string
  created_at?: string
  updated_at?: string
}

export interface Vault {
  id: string
  name: string
}

export interface AuthStatus {
  initialized: boolean
  master_key_available: boolean
  account_count: number
  gog_passphrase_set: boolean
}

export interface AccountInfo {
  provider: string
  account: string
  scopes: string[]
  expiry: string
  last_refreshed?: string
}

export interface TokenInfo {
  access_token: string
  token_type: string
  expiry: string
}

export interface TokenRecord {
  provider: string
  account: string
  client_id: string
  client_secret: string
  token_type: string
  access_token: string
  refresh_token: string
  expiry: string
  scopes: string[]
  issued_at?: string
  last_refreshed?: string
}

export interface ProxyStatus {
  running: boolean
  socket_path?: string | null
}

export interface ManifestSummary {
  capabilities_loaded: number
  tools_available: number
  mcp_exposed: number
}

export interface ActivityEvent {
  id: string
  timestamp: string
  event_type: string
  source: string
  message: string
  metadata?: Record<string, unknown>
}

export interface GoogleStatus {
  status: 'connected' | 'not_configured' | 'token_expired' | string
  configured: boolean
  has_token: boolean
  message: string
}

export interface GoogleConfig {
  client_id: string
  client_secret: string
  updated_at: string
}

export interface Tool {
  name: string
  description: string
  inputSchema: Record<string, unknown>
  outputSchema?: Record<string, unknown>
  annotations?: Record<string, unknown>
}

export interface Manifest {
  id: string
  name: string
  description: string
  category: string
  version: string
  tools: Tool[]
  requires?: {
    permissions?: Array<{ name: string; optional?: boolean; reason?: string }>
    credentials?: Array<{ id?: string; provider?: string; kind?: string; scopes?: string[]; optional?: boolean }>
  }
}

export interface Permission {
  id: string
  name: string
  description: string
  status: string
  critical: boolean
  last_checked?: string
}

export interface PermissionDefinition {
  id: string
  name: string
  description: string
  critical: boolean
  icon: string
  system_pref_pane: string
}

export interface NamespaceStatus {
  namespace: string
  connected: boolean
  tool_count: number
}

export interface SocketStatus {
  alive: boolean
}

export interface AgentConfig {
  id: string
  workspace?: string
  default?: boolean
  model?: {
    primary?: string
  }
  models?: Record<string, { alias?: string }>
}

export interface ProviderConfig {
  baseUrl?: string
  apiKey?: string
  models?: ModelConfig[]
}

export interface ModelConfig {
  id: string
  name: string
  api?: string
  reasoning?: boolean
  input?: string[]
  cost?: {
    input: number
    output: number
    cacheRead?: number
    cacheWrite?: number
  }
  contextWindow?: number
  maxTokens?: number
}

export interface ModelOption {
  id: string
  name: string
  description?: string
}

export interface OpenClawConfig {
  meta?: { lastTouchedVersion?: string; lastTouchedAt?: string }
  agents?: {
    defaults?: {
      model?: { primary?: string }
      workspace?: string
      memorySearch?: Record<string, unknown>
      compaction?: { mode?: string }
      heartbeat?: Record<string, unknown>
      maxConcurrent?: number
      subagents?: { maxConcurrent?: number }
    }
    list?: AgentConfig[]
  }
  models?: { mode?: string; providers?: Record<string, ProviderConfig> }
  gateway?: { port?: number; mode?: string; bind?: string; auth?: { mode?: string; token?: string } }
  channels?: Record<string, unknown>
  tools?: Record<string, unknown>
  bindings?: Array<{ agentId: string; match: { channel: string; accountId?: string } }>
  messages?: Record<string, unknown>
  hooks?: Record<string, unknown>
  plugins?: Record<string, unknown>
  [key: string]: unknown
}

export interface NodeConfig {
  version: number
  nodeId: string
  displayName: string
  gateway: {
    host: string
    port: number
    tls: boolean
  }
}

export interface ExecApproval {
  pattern: string
  approved: boolean
  timestamp?: string
}

export interface EnvironmentInfo {
  type: 'gateway' | 'node' | 'unknown'
  files: Array<{ name: string; path: string }>
}
