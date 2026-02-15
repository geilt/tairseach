import { invoke } from '@tauri-apps/api/core'
import type {
  AccountInfo,
  ActivityEvent,
  AuthStatus,
  CredentialMetadata,
  CredentialType,
  EnvironmentInfo,
  ExecApproval,
  GoogleConfig,
  GoogleStatus,
  Manifest,
  ManifestSummary,
  ModelOption,
  NamespaceStatus,
  NodeConfig,
  OpenClawConfig,
  Permission,
  PermissionDefinition,
  ProxyStatus,
  SocketStatus,
  TokenInfo,
  TokenRecord,
  Vault,
} from './types'

async function call<T>(command: string, params?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, params)
  } catch (error) {
    throw new Error(error instanceof Error ? error.message : String(error))
  }
}

export const api = {
  auth: {
    status: () => call<AuthStatus>('auth_status'),
    providers: () => call<string[]>('auth_providers'),
    accounts: (provider: string | null = null) => call<AccountInfo[]>('auth_accounts', { provider }),
    getToken: (provider: string, account: string, scopes?: string[]) => call<TokenInfo>('auth_get_token', { provider, account, scopes }),
    refreshToken: (provider: string, account: string) => call<void>('auth_refresh_token', { provider, account }),
    revokeToken: (provider: string, account: string) => call<void>('auth_revoke_token', { provider, account }),
    storeToken: (record: TokenRecord) => call<void>('auth_store_token', { record }),
    credentialTypes: () => call<CredentialType[]>('auth_credential_types'),
    credentialsList: (credType: string | null = null) => call<CredentialMetadata[]>('auth_credentials_list', { credType }),
    credentialsStore: (credType: string, label: string, fields: Record<string, string>) =>
      call<void>('auth_credentials_store', { credType, label, fields }),
    credentialsDelete: (credType: string, label: string) => call<void>('auth_credentials_delete', { credType, label }),
    credentialsRename: (credType: string, oldLabel: string, newLabel: string) =>
      call<void>('auth_credentials_rename', { credType, oldLabel, newLabel }),
    customCredentialTypeCreate: (type: string, displayName: string, fields: CredentialType['fields']) =>
      call<void>('auth_credential_types_custom_create', { type, displayName, fields }),
    startGoogleOauth: () => call<{ success: boolean; account: string }>('auth_start_google_oauth'),
  },
  onePassword: {
    listVaults: () => call<{ vaults: Vault[]; default_vault: string | null }>('op_vaults_list'),
    setDefaultVault: (vaultId: string) => call<void>('op_config_set_default_vault', { vaultId }),
  },
  config: {
    get: () => call<{ raw: OpenClawConfig; path: string }>('get_config'),
    set: (config: OpenClawConfig) => call<void>('set_config', { config }),
    providerModels: () => call<Record<string, ModelOption[]>>('get_provider_models'),
    environment: () => call<EnvironmentInfo>('get_environment'),
    getNodeConfig: () => call<{ config: NodeConfig; path: string }>('get_node_config'),
    setNodeConfig: (config: NodeConfig) => call<void>('set_node_config', { config }),
    getExecApprovals: () => call<{ approvals: ExecApproval[]; path: string }>('get_exec_approvals'),
    setExecApprovals: (approvals: ExecApproval[]) => call<void>('set_exec_approvals', { approvals }),
  },
  permissions: {
    definitions: () => call<PermissionDefinition[]>('get_permission_definitions'),
    all: () => call<Permission[]>('check_all_permissions'),
    check: (permissionId: string) => call<Permission>('check_permission', { permissionId }),
    request: (permissionId: string) => call<void>('request_permission', { permissionId }),
    openSettings: (pane: string) => call<void>('open_permission_settings', { pane }),
  },
  mcp: {
    manifests: () => call<Manifest[]>('get_all_manifests'),
    testTool: (toolName: string, params: Record<string, unknown>) => call<unknown>('test_mcp_tool', { toolName, params }),
    installToOpenClaw: () => call<{ success: boolean; message: string; config_path?: string }>('install_tairseach_to_openclaw'),
    manifestSummary: () => call<ManifestSummary>('get_manifest_summary'),
  },
  google: {
    getConfig: () => call<GoogleConfig | null>('get_google_oauth_config'),
    getStatus: () => call<GoogleStatus>('get_google_oauth_status'),
    saveConfig: (clientId: string, clientSecret: string) => call<void>('save_google_oauth_config', { clientId, clientSecret }),
    testConfig: (clientId: string, clientSecret: string) =>
      call<{ ok: boolean; message: string; error?: string }>('test_google_oauth_config', { clientId, clientSecret }),
  },
  events: {
    list: (limit: number) => call<ActivityEvent[]>('get_events', { limit }),
  },
  system: {
    proxyStatus: () => call<ProxyStatus>('get_proxy_status'),
    socketAlive: () => call<SocketStatus>('check_socket_alive'),
    namespaceStatuses: () => call<NamespaceStatus[]>('get_namespace_statuses'),
    invokeCommand: <T>(command: string, params?: Record<string, unknown>) => call<T>(command, params),
  },
}
