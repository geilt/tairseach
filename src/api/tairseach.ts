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
    status: () => call<AuthStatus>('auth_status_get'),
    providers: () => call<string[]>('auth_providers_list'),
    accounts: (provider: string | null = null) => call<AccountInfo[]>('auth_accounts_list', { provider }),
    getToken: (provider: string, account: string, scopes?: string[]) => call<TokenInfo>('auth_get_token', { provider, account, scopes }),
    refreshToken: (provider: string, account: string) => call<void>('auth_refresh_token', { provider, account }),
    revokeToken: (provider: string, account: string) => call<void>('auth_revoke_token', { provider, account }),
    storeToken: (record: TokenRecord) => call<void>('auth_store_token', { record }),
    credentialTypes: () => call<CredentialType[]>('auth_credential_types_list'),
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
    setDefaultVault: (vaultId: string) => call<void>('op_config_default_vault_set', { vaultId }),
  },
  config: {
    get: () => call<{ raw: OpenClawConfig; path: string }>('config_app_get'),
    set: (config: OpenClawConfig) => call<void>('config_app_set', { config }),
    providerModels: () => call<Record<string, ModelOption[]>>('config_models_list'),
    environment: () => call<EnvironmentInfo>('config_environment_get'),
    getNodeConfig: () => call<{ config: NodeConfig; path: string }>('config_node_get'),
    setNodeConfig: (config: NodeConfig) => call<void>('config_node_set', { config }),
    getExecApprovals: () => call<{ approvals: ExecApproval[]; path: string }>('config_exec_approvals_get'),
    setExecApprovals: (approvals: ExecApproval[]) => call<void>('config_exec_approvals_set', { approvals }),
  },
  permissions: {
    definitions: () => call<PermissionDefinition[]>('permissions_definitions_get'),
    all: () => call<Permission[]>('check_all_permissions'),
    check: (permissionId: string) => call<Permission>('permissions_single_check', { permissionId }),
    request: (permissionId: string) => call<void>('permissions_single_request', { permissionId }),
    openSettings: (pane: string) => call<void>('permissions_settings_open', { pane }),
  },
  mcp: {
    manifests: () => call<Manifest[]>('manifests_all_list'),
    testTool: (toolName: string, params: Record<string, unknown>) => call<unknown>('monitor_mcp_tool_test', { toolName, params }),
    installToOpenClaw: () => call<{ success: boolean; message: string; config_path?: string }>('monitor_openclaw_install'),
    manifestSummary: () => call<ManifestSummary>('monitor_manifest_summary_get'),
  },
  google: {
    getConfig: () => call<GoogleConfig | null>('config_google_oauth_get'),
    getStatus: () => call<GoogleStatus>('config_google_oauth_status_get'),
    saveConfig: (clientId: string, clientSecret: string) => call<void>('config_google_oauth_save', { clientId, clientSecret }),
    testConfig: (clientId: string, clientSecret: string) =>
      call<{ ok: boolean; message: string; error?: string }>('config_google_oauth_test', { clientId, clientSecret }),
  },
  events: {
    list: (limit: number) => call<ActivityEvent[]>('monitor_events_list', { limit }),
  },
  errors: {
    submit: (report: {
      ts: string
      source: string
      severity: string
      code: string
      message: string
      context?: Record<string, unknown>
      stack?: string
    }) => call<void>('error_report_submit', { report }),
  },
  system: {
    proxyStatus: () => call<ProxyStatus>('proxy_status_get'),
    socketAlive: () => call<SocketStatus>('monitor_socket_check'),
    namespaceStatuses: () => call<NamespaceStatus[]>('monitor_namespace_statuses_get'),
    invokeCommand: <T>(command: string, params?: Record<string, unknown>) => call<T>(command, params),
  },
}
