<script setup lang="ts">
import { computed, onActivated, onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useAuthStore } from '@/stores/auth'
import type { AccountInfo } from '@/stores/auth'

// ═══════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════

interface CredentialField {
  name: string
  display_name: string
  type: 'string' | 'secret'
  required: boolean
}

interface CredentialType {
  type: string
  display_name: string
  fields: CredentialField[]
  supports_multiple: boolean
}

interface CredentialMetadata {
  type: string
  label: string
  created_at?: string
  updated_at?: string
}

interface Vault {
  id: string
  name: string
}

// ═══════════════════════════════════════════════════════════════
// STATE
// ═══════════════════════════════════════════════════════════════

const store = useAuthStore()
const route = useRoute()
const actionMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)
const isConnectingOAuth = ref(false)
const googleSectionOpen = ref(false)
const googleConfigOpen = ref(false)
const googleClientId = ref('')
const googleClientSecret = ref('')
const googleJsonInput = ref('')
const googleDragOver = ref(false)
const savingGoogleConfig = ref(false)
const testingGoogleConfig = ref(false)
const googleStatus = ref<{ status: string; configured: boolean; has_token: boolean; message: string } | null>(null)
let actionMessageTimer: number | null = null

function scheduleActionMessageClear(delayMs = 3000) {
  if (actionMessageTimer) clearTimeout(actionMessageTimer)
  actionMessageTimer = window.setTimeout(() => {
    actionMessage.value = null
    actionMessageTimer = null
  }, delayMs)
}

// Credential types
const credentialTypes = ref<CredentialType[]>([])
const loadingTypes = ref(false)
const credentialsByType = ref<Record<string, CredentialMetadata[]>>({})
const loadingCredentials = ref(false)

// Active credential forms
const activeForm = ref<string | null>(null)
const formData = ref<Record<string, any>>({})
const savingCredential = ref(false)
const renamingCredential = ref<{ typeId: string; oldLabel: string } | null>(null)
const renameLabel = ref('')
const savingRename = ref(false)

// 1Password specific
const vaults = ref<Vault[]>([])
const loadingVaults = ref(false)
const defaultVault = ref<string | null>(null)
const settingDefaultVault = ref(false)

// Custom credential type creation
const showCustomTypeForm = ref(false)
const customTypeName = ref('')
const customTypeDisplayName = ref('')
const customTypeFields = ref<CredentialField[]>([])


onUnmounted(() => {
  if (actionMessageTimer) {
    clearTimeout(actionMessageTimer)
    actionMessageTimer = null
  }
})

// ═══════════════════════════════════════════════════════════════
// COMPUTED
// ═══════════════════════════════════════════════════════════════

const statusColor = computed(() => {
  if (!store.status) return 'text-naonur-smoke'
  return store.status.initialized ? 'text-naonur-moss' : 'text-naonur-rust'
})

const statusText = computed(() => {
  if (!store.status) return 'Unknown'
  if (!store.status.initialized) return 'Not Initialized'
  if (!store.status.master_key_available) return 'Master Key Missing'
  return 'Ready'
})

// ═══════════════════════════════════════════════════════════════
// LIFECYCLE
// ═══════════════════════════════════════════════════════════════

onMounted(() => {
  void store.init()
  void loadCredentialTypes()
  void loadAllCredentials()
  void loadGoogleConfigStatus()

  const requested = String(route.query.credential || '')
  if (requested) {
    openCredentialForm(requested)
  }
})

onActivated(() => {
  void store.loadStatus({ silent: true })
  void store.loadAccounts({ silent: true })
  void loadAllCredentials()
})

// ═══════════════════════════════════════════════════════════════
// CREDENTIAL TYPE METHODS
// ═══════════════════════════════════════════════════════════════

async function loadCredentialTypes() {
  loadingTypes.value = true
  try {
    const [types, manifests] = await Promise.all([
      invoke<CredentialType[]>('auth_credential_types'),
      invoke<any[]>('get_all_manifests').catch(() => [])
    ])
    const dynamicTypes = loadManifestCredentialTypes(manifests)
    const existing = new Set(types.map(t => t.type))
    credentialTypes.value = [...types, ...dynamicTypes.filter(t => !existing.has(t.type))]
  } catch (e) {
    console.warn('Backend credential types not implemented yet:', e)
    const defaultTypes: CredentialType[] = [
      {
        type: '1password',
        display_name: '1Password',
        supports_multiple: false,
        fields: [
          { name: 'service_account_token', display_name: 'Service Account Token', type: 'secret' as const, required: true }
        ]
      },
      {
        type: 'jira',
        display_name: 'Jira',
        supports_multiple: true,
        fields: [
          { name: 'host', display_name: 'Host (e.g., company.atlassian.net)', type: 'string' as const, required: true },
          { name: 'email', display_name: 'Email', type: 'string' as const, required: true },
          { name: 'api_token', display_name: 'API Token', type: 'secret' as const, required: true }
        ]
      },
      {
        type: 'oura',
        display_name: 'Oura',
        supports_multiple: false,
        fields: [
          { name: 'access_token', display_name: 'Personal Access Token', type: 'secret' as const, required: true }
        ]
      }
    ]
    const manifests = await invoke<any[]>('get_all_manifests').catch(() => [])
    const dynamicTypes = loadManifestCredentialTypes(manifests)
    const existing = new Set(defaultTypes.map(t => t.type))
    credentialTypes.value = [...defaultTypes, ...dynamicTypes.filter(t => !existing.has(t.type))]
  } finally {
    loadingTypes.value = false
  }
}

async function loadAllCredentials() {
  loadingCredentials.value = true
  try {
    const all = await invoke<CredentialMetadata[]>('auth_credentials_list', { credType: null })
    
    // Group by type
    const grouped: Record<string, CredentialMetadata[]> = {}
    for (const cred of all) {
      if (!grouped[cred.type]) {
        grouped[cred.type] = []
      }
      grouped[cred.type].push(cred)
    }
    
    requestAnimationFrame(() => {
      credentialsByType.value = grouped
      loadingCredentials.value = false
    })
  } catch (e) {
    console.warn('Backend credentials list not implemented yet:', e)
    requestAnimationFrame(() => {
      credentialsByType.value = {}
      loadingCredentials.value = false
    })
  }
}

// ═══════════════════════════════════════════════════════════════
// CREDENTIAL FORM METHODS
// ═══════════════════════════════════════════════════════════════

function openCredentialForm(typeId: string) {
  activeForm.value = typeId
  formData.value = { label: '' }
}

function closeCredentialForm() {
  activeForm.value = null
  formData.value = {}
}

async function saveCredential(typeId: string) {
  const credType = credentialTypes.value.find(t => t.type === typeId)
  if (!credType) return
  
  // Validate required fields
  for (const field of credType.fields) {
    if (field.required && !formData.value[field.name]) {
      setFeedback('error', `${field.display_name} is required`)
      return
    }
  }
  
  if (!formData.value.label) {
    setFeedback('error', 'Label is required')
    return
  }
  
  savingCredential.value = true
  try {
    await invoke('auth_credentials_store', {
      credType: typeId,
      label: formData.value.label,
      fields: Object.fromEntries(
        credType.fields.map(f => [f.name, formData.value[f.name] || ''])
      )
    })
    
    requestAnimationFrame(() => {
      setFeedback('success', `${credType.display_name} credential saved`)
      closeCredentialForm()
      savingCredential.value = false
    })
    
    await loadAllCredentials()
    await store.loadAccounts({ silent: true })
    await store.loadStatus({ silent: true })
    
    // If 1Password, load vaults
    if (typeId === '1password' || typeId === 'onepassword') {
      await load1PasswordVaults()
    }
  } catch (e) {
    requestAnimationFrame(() => {
      setFeedback('error', `Failed to save: ${e}`)
      savingCredential.value = false
    })
  }
}

async function deleteCredential(typeId: string, label: string) {
  try {
    await invoke('auth_credentials_delete', { credType: typeId, label })
    requestAnimationFrame(() => {
      setFeedback('success', 'Credential deleted')
    })
    await loadAllCredentials()
    await store.loadAccounts({ silent: true })
    await store.loadStatus({ silent: true })
  } catch (e) {
    requestAnimationFrame(() => {
      setFeedback('error', `Failed to delete: ${e}`)
    })
  }
}

function beginRenameCredential(typeId: string, oldLabel: string) {
  renamingCredential.value = { typeId, oldLabel }
  renameLabel.value = oldLabel
}

function cancelRenameCredential() {
  renamingCredential.value = null
  renameLabel.value = ''
}

async function saveRenameCredential(typeId: string, oldLabel: string) {
  const newLabel = renameLabel.value.trim()
  if (!newLabel) {
    setFeedback('error', 'New label is required')
    return
  }
  if (newLabel === oldLabel) {
    cancelRenameCredential()
    return
  }

  savingRename.value = true
  try {
    await invoke('auth_credentials_rename', {
      credType: typeId,
      oldLabel,
      newLabel,
    })

    setFeedback('success', `Renamed "${oldLabel}" → "${newLabel}"`)
    cancelRenameCredential()
    await loadAllCredentials()
    await store.loadAccounts({ silent: true })
    await store.loadStatus({ silent: true })
  } catch (e) {
    setFeedback('error', `Failed to rename: ${e}`)
  } finally {
    savingRename.value = false
  }
}

// ═══════════════════════════════════════════════════════════════
// 1PASSWORD METHODS
// ═══════════════════════════════════════════════════════════════

async function load1PasswordVaults() {
  loadingVaults.value = true
  try {
    const result = await invoke<{ vaults: Vault[], default_vault: string | null }>('op_vaults_list')
    requestAnimationFrame(() => {
      vaults.value = result.vaults
      defaultVault.value = result.default_vault
      loadingVaults.value = false
    })
  } catch (e) {
    console.warn('Failed to load 1Password vaults:', e)
    requestAnimationFrame(() => {
      loadingVaults.value = false
    })
  }
}

async function setDefault1PasswordVault(vaultId: string) {
  settingDefaultVault.value = true
  try {
    await invoke('op_config_set_default_vault', { vaultId })
    requestAnimationFrame(() => {
      defaultVault.value = vaultId
      setFeedback('success', 'Default vault updated')
      settingDefaultVault.value = false
    })
  } catch (e) {
    requestAnimationFrame(() => {
      setFeedback('error', `Failed to set default vault: ${e}`)
      settingDefaultVault.value = false
    })
  }
}

// ═══════════════════════════════════════════════════════════════
// CUSTOM TYPE METHODS
// ═══════════════════════════════════════════════════════════════

function addCustomFieldRow() {
  customTypeFields.value.push({
    name: '',
    display_name: '',
    type: 'string',
    required: false
  })
}

function removeCustomFieldRow(index: number) {
  customTypeFields.value.splice(index, 1)
}

async function saveCustomType() {
  if (!customTypeName.value || !customTypeDisplayName.value) {
    setFeedback('error', 'Type name and display name are required')
    return
  }
  
  if (customTypeFields.value.length === 0) {
    setFeedback('error', 'Add at least one field')
    return
  }
  
  // Validate fields
  for (const field of customTypeFields.value) {
    if (!field.name || !field.display_name) {
      setFeedback('error', 'All field names and display names are required')
      return
    }
  }
  
  try {
    await invoke('auth_credential_types_custom_create', {
      type: customTypeName.value,
      displayName: customTypeDisplayName.value,
      fields: customTypeFields.value
    })
    
    requestAnimationFrame(() => {
      setFeedback('success', 'Custom credential type created')
      showCustomTypeForm.value = false
      customTypeName.value = ''
      customTypeDisplayName.value = ''
      customTypeFields.value = []
    })
    
    await loadCredentialTypes()
  } catch (e) {
    requestAnimationFrame(() => {
      setFeedback('error', `Failed to create custom type: ${e}`)
    })
  }
}

// ═══════════════════════════════════════════════════════════════
// LEGACY OAUTH METHODS (Google)
// ═══════════════════════════════════════════════════════════════

function getProviderIcon(provider: string): string {
  const icons: Record<string, string> = {
    google: '🔵',
    microsoft: '🔷',
    github: '🐙',
    '1password': '🔑',
    jira: '📋',
    oura: '💍'
  }
  return icons[provider.toLowerCase()] || '🔐'
}

function isTokenExpired(expiry: string): boolean {
  try {
    const expiryDate = new Date(expiry)
    return expiryDate < new Date()
  } catch {
    return true
  }
}

function isTokenExpiringSoon(expiry: string): boolean {
  try {
    const expiryDate = new Date(expiry)
    const fiveMinutes = 5 * 60 * 1000
    return expiryDate < new Date(Date.now() + fiveMinutes)
  } catch {
    return true
  }
}

function formatExpiry(expiry: string): string {
  try {
    const date = new Date(expiry)
    const now = new Date()
    const diff = date.getTime() - now.getTime()
    
    if (diff < 0) return 'Expired'
    
    const hours = Math.floor(diff / (1000 * 60 * 60))
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60))
    
    if (hours > 24) {
      const days = Math.floor(hours / 24)
      return `${days}d ${hours % 24}h`
    }
    if (hours > 0) return `${hours}h ${minutes}m`
    return `${minutes}m`
  } catch {
    return 'Invalid'
  }
}

function getTokenStatusColor(account: AccountInfo): string {
  if (isTokenExpired(account.expiry)) return 'text-naonur-blood'
  if (isTokenExpiringSoon(account.expiry)) return 'text-naonur-rust'
  return 'text-naonur-moss'
}

async function handleRefresh(account: AccountInfo) {
  actionMessage.value = null
  const success = await store.refreshToken(account.provider, account.account)
  
  requestAnimationFrame(() => {
    if (success) {
      actionMessage.value = { type: 'success', text: `Refreshed ${account.account}` }
    } else {
      actionMessage.value = { type: 'error', text: store.error || 'Refresh failed' }
    }
    scheduleActionMessageClear(3000)
  })
}

async function handleRevoke(account: AccountInfo) {
  actionMessage.value = null
  
  // Try revoke first (works for OAuth providers like Google), then delete
  let success = false
  try {
    success = await store.revokeToken(account.provider, account.account)
  } catch (_e) {
    // Revoke not supported for this provider type — just delete
  }
  
  if (!success) {
    // Fall back to credential delete
    try {
      await invoke('auth_credentials_delete', { credType: account.provider, label: account.account })
      success = true
      // Reload accounts
      await store.loadAccounts({ silent: true })
      await store.loadStatus({ silent: true })
    } catch (e) {
      console.error('Delete failed:', e)
    }
  }
  
  requestAnimationFrame(() => {
    if (success) {
      actionMessage.value = { type: 'success', text: `Deleted ${account.account}` }
    } else {
      actionMessage.value = { type: 'error', text: store.error || 'Delete failed' }
    }
    scheduleActionMessageClear(3000)
  })
}

async function handleConnectGoogle() {
  isConnectingOAuth.value = true
  
  requestAnimationFrame(() => {
    actionMessage.value = {
      type: 'success',
      text: 'Opening Google sign-in...',
    }
  })
  
  try {
    const result = await invoke<{ success: boolean; account: string }>('auth_start_google_oauth')
    
    requestAnimationFrame(() => {
      actionMessage.value = {
        type: 'success',
        text: `Connected: ${result.account}`,
      }
      isConnectingOAuth.value = false
    })
    
    await store.loadAccounts()
    await loadGoogleConfigStatus()
    
    requestAnimationFrame(() => {
      scheduleActionMessageClear(5000)
    })
  } catch (e) {
    requestAnimationFrame(() => {
      actionMessage.value = {
        type: 'error',
        text: `OAuth failed: ${e}`,
      }
      isConnectingOAuth.value = false
      scheduleActionMessageClear(5000)
    })
  }
}


function toggleGoogleSection() {
  googleSectionOpen.value = !googleSectionOpen.value
}

function parseAndApplyGoogleJson(raw: string) {
  const parsed = JSON.parse(raw)
  const installed = parsed?.installed ?? parsed?.web ?? parsed
  const extractedId = installed?.client_id
  const extractedSecret = installed?.client_secret
  if (!extractedId || !extractedSecret) throw new Error('Could not find client_id and client_secret in JSON')
  googleClientId.value = extractedId
  googleClientSecret.value = extractedSecret
  setFeedback('success', 'Parsed Google client_secret.json')
}

async function loadGoogleConfigStatus() {
  try {
    const [cfg, st] = await Promise.all([
      invoke<{ client_id: string; client_secret: string } | null>('get_google_oauth_config'),
      invoke<{ status: string; configured: boolean; has_token: boolean; message: string }>('get_google_oauth_status'),
    ])
    if (cfg) {
      googleClientId.value = cfg.client_id || ''
      googleClientSecret.value = cfg.client_secret || ''
    }
    googleStatus.value = st
  } catch (e) {
    console.warn('Google config/status unavailable:', e)
  }
}

async function saveGoogleConfig() {
  if (!googleClientId.value.trim() || !googleClientSecret.value.trim()) {
    setFeedback('error', 'Google Client ID and Client Secret are required')
    return
  }
  savingGoogleConfig.value = true
  try {
    await invoke('save_google_oauth_config', { clientId: googleClientId.value, clientSecret: googleClientSecret.value })
    setFeedback('success', 'Google OAuth credentials saved')
    await loadGoogleConfigStatus()
  } catch (e) {
    setFeedback('error', `Failed to save Google config: ${e}`)
  } finally {
    savingGoogleConfig.value = false
  }
}

async function testGoogleConfig() {
  testingGoogleConfig.value = true
  try {
    const result = await invoke<{ ok: boolean; message: string }>('test_google_oauth_config', {
      clientId: googleClientId.value,
      clientSecret: googleClientSecret.value,
    })
    setFeedback(result.ok ? 'success' : 'error', result.message)
    await loadGoogleConfigStatus()
  } catch (e) {
    setFeedback('error', `Google test failed: ${e}`)
  } finally {
    testingGoogleConfig.value = false
  }
}

async function onGoogleFileInput(event: Event) {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return
  const text = await file.text()
  parseAndApplyGoogleJson(text)
}

async function onGoogleDrop(event: DragEvent) {
  googleDragOver.value = false
  const file = event.dataTransfer?.files?.[0]
  if (!file) return
  const text = await file.text()
  parseAndApplyGoogleJson(text)
}

function loadManifestCredentialTypes(manifests: any[]): CredentialType[] {
  const built: CredentialType[] = []
  const seen = new Set<string>()
  for (const manifest of manifests) {
    const creds = manifest?.requires?.credentials
    if (!Array.isArray(creds)) continue
    for (const c of creds) {
      const rawType = (c?.provider || c?.id || '').toString().toLowerCase()
      if (!rawType) continue
      let typeId = rawType.replace(/[^a-z0-9]+/g, '_')
      if (typeId.includes('google')) typeId = 'google_oauth'
      if (typeId.includes('onepassword')) typeId = '1password'
      if (seen.has(typeId)) continue
      seen.add(typeId)

      const schemaFields = Array.isArray(c?.schema?.fields) ? c.schema.fields : null
      const fields = schemaFields?.map((f: any) => ({
        name: f.name,
        display_name: f.display_name || f.name,
        type: f.type === 'secret' ? 'secret' : 'string',
        required: f.required !== false,
      })) || [
        {
          name: c?.kind === 'oauth2' ? 'oauth_token' : 'token',
          display_name: c?.kind === 'oauth2' ? 'OAuth Token' : 'Token',
          type: 'secret' as const,
          required: true,
        },
      ]

      built.push({
        type: typeId,
        display_name: manifest?.name || typeId,
        fields,
        supports_multiple: true,
      })
    }
  }
  return built
}

// ═══════════════════════════════════════════════════════════════
// UTILITIES
// ═══════════════════════════════════════════════════════════════

function setFeedback(type: 'success' | 'error', text: string) {
  actionMessage.value = { type, text }
  scheduleActionMessageClear(3000)
}
</script>

<template>
  <div class="animate-fade-in">
    <!-- Header -->
    <div class="mb-8 flex items-start justify-between">
      <div>
        <h1 class="font-display text-2xl tracking-wider text-naonur-gold mb-2 flex items-center gap-3">
          <img src="@/assets/icons/auth-services.png" alt="Auth" class="w-8 h-8 object-contain" />
          Auth Services
        </h1>
        <p class="text-naonur-ash font-body">
          Credential management for CLI tools, APIs, and OAuth services.
        </p>
      </div>
      
      <!-- Action Message -->
      <Transition
        enter-active-class="transition-opacity duration-200"
        enter-from-class="opacity-0"
        leave-active-class="transition-opacity duration-200"
        leave-to-class="opacity-0"
      >
        <span 
          v-if="actionMessage" 
          :class="[
            'text-sm px-4 py-2 rounded-lg',
            actionMessage.type === 'success' ? 'text-naonur-moss bg-naonur-moss/10' : 'text-naonur-blood bg-naonur-blood/10'
          ]"
        >
          {{ actionMessage.text }}
        </span>
      </Transition>
    </div>

    <!-- Auth Broker Status -->
    <div class="naonur-card mb-6">
      <div class="flex items-start justify-between mb-4">
        <div>
          <h2 class="font-display text-lg text-naonur-bone mb-1">Auth Broker Status</h2>
          <p class="text-sm text-naonur-ash">Encrypted token storage and refresh daemon</p>
        </div>
        <span :class="['font-mono text-sm font-medium', statusColor]">
          {{ statusText }}
        </span>
      </div>
      
      <div class="grid grid-cols-3 gap-4 pt-4 border-t border-naonur-fog/20">
        <div>
          <p class="text-xs text-naonur-smoke mb-1">Initialized</p>
          <p :class="['text-sm font-medium', store.status?.initialized ? 'text-naonur-moss' : 'text-naonur-blood']">
            {{ store.status?.initialized ? 'Yes' : 'No' }}
          </p>
        </div>
        <div>
          <p class="text-xs text-naonur-smoke mb-1">Master Key</p>
          <p :class="['text-sm font-medium', store.status?.master_key_available ? 'text-naonur-moss' : 'text-naonur-blood']">
            {{ store.status?.master_key_available ? 'Available' : 'Missing' }}
          </p>
        </div>
        <div>
          <p class="text-xs text-naonur-smoke mb-1">Accounts</p>
          <p class="text-sm font-medium text-naonur-bone">
            {{ store.status?.account_count ?? 0 }}
          </p>
        </div>
      </div>
    </div>

    <!-- OAuth Accounts (Legacy - Google) -->
    <div v-if="store.accounts.length > 0" class="naonur-card mb-6">
      <div class="flex items-center justify-between mb-4">
        <h2 class="font-display text-lg text-naonur-bone">OAuth Accounts</h2>
        <span class="text-sm text-naonur-smoke">
          {{ store.accounts.length }} account{{ store.accounts.length === 1 ? '' : 's' }}
        </span>
      </div>

      <div class="space-y-3">
        <div 
          v-for="account in store.accounts" 
          :key="`${account.provider}:${account.account}`"
          class="p-4 rounded-lg bg-naonur-fog/10 border border-naonur-fog/20"
        >
          <div class="flex items-start justify-between">
            <div class="flex items-start gap-3 flex-1">
              <span class="text-2xl">{{ getProviderIcon(account.provider) }}</span>
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <h3 class="font-display text-naonur-bone">{{ account.account }}</h3>
                  <span class="text-xs text-naonur-smoke px-2 py-0.5 rounded-full bg-naonur-void/50">
                    {{ account.provider }}
                  </span>
                </div>
                
                <div class="flex items-center gap-4 text-xs text-naonur-smoke">
                  <span :class="getTokenStatusColor(account)">
                    {{ isTokenExpired(account.expiry) ? '❌ Expired' : '✓ Valid' }}
                  </span>
                  <span>Expires: {{ formatExpiry(account.expiry) }}</span>
                </div>
              </div>
            </div>
            
            <div class="flex gap-2 ml-4">
              <button
                class="btn btn-ghost text-xs"
                :disabled="store.loading"
                @click="handleRefresh(account)"
              >
                🔄 Refresh
              </button>
              <button
                class="btn btn-ghost text-xs text-naonur-blood hover:text-naonur-blood/80"
                :disabled="store.loading"
                @click="handleRevoke(account)"
              >
                🗑️ Delete
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Credential Management -->
    <div class="naonur-card mb-6">
      <div class="flex items-center justify-between mb-4">
        <h2 class="font-display text-lg text-naonur-bone">Credential Management</h2>
        <button
          class="btn btn-secondary text-xs px-3 py-1"
          @click="showCustomTypeForm = true"
        >
          + Add Custom Type
        </button>
      </div>

      <!-- Loading State -->
      <div v-if="loadingTypes" class="text-center py-8 text-naonur-ash animate-pulse">
        Loading credential types...
      </div>

      <!-- Credential Type Sections -->
      <div v-else class="space-y-4">
        <div 
          v-for="credType in credentialTypes" 
          :key="credType.type"
          class="border border-naonur-fog/30 rounded-lg overflow-hidden"
        >
          <!-- Type Header -->
          <div class="px-4 py-3 bg-naonur-fog/10 flex items-center justify-between">
            <div class="flex items-center gap-3">
              <span class="text-2xl">{{ getProviderIcon(credType.type) }}</span>
              <div>
                <h3 class="font-display text-naonur-bone">{{ credType.display_name }}</h3>
                <p class="text-xs text-naonur-smoke">
                  {{ credType.supports_multiple ? 'Multiple accounts supported' : 'Single account' }}
                </p>
              </div>
            </div>
            <button
              v-if="credType.supports_multiple || !(credentialsByType[credType.type]?.length > 0)"
              class="btn btn-ghost text-xs"
              @click="openCredentialForm(credType.type)"
            >
              + Add
            </button>
          </div>

          <!-- Stored Credentials -->
          <div v-if="credentialsByType[credType.type]?.length > 0" class="divide-y divide-naonur-fog/20">
            <div 
              v-for="cred in credentialsByType[credType.type]" 
              :key="cred.label"
              class="px-4 py-3 hover:bg-naonur-fog/5"
            >
              <div class="flex items-center justify-between gap-3">
                <div class="min-w-0">
                  <template v-if="renamingCredential?.typeId === credType.type && renamingCredential?.oldLabel === cred.label">
                    <div class="flex items-center gap-2">
                      <input
                        v-model="renameLabel"
                        type="text"
                        class="input-field text-sm"
                        placeholder="Credential label"
                        :disabled="savingRename"
                        @keyup.enter="saveRenameCredential(credType.type, cred.label)"
                      />
                      <button
                        class="btn btn-ghost text-xs text-naonur-moss"
                        :disabled="savingRename"
                        @click="saveRenameCredential(credType.type, cred.label)"
                      >
                        {{ savingRename ? 'Saving…' : 'Save' }}
                      </button>
                      <button
                        class="btn btn-ghost text-xs"
                        :disabled="savingRename"
                        @click="cancelRenameCredential"
                      >
                        Cancel
                      </button>
                    </div>
                  </template>
                  <template v-else>
                    <p class="text-sm font-medium text-naonur-bone truncate">{{ cred.label }}</p>
                    <p class="text-xs text-naonur-smoke">
                      {{ cred.created_at ? `Added ${new Date(cred.created_at).toLocaleDateString()}` : 'No date' }}
                    </p>
                  </template>
                </div>

                <div v-if="!(renamingCredential?.typeId === credType.type && renamingCredential?.oldLabel === cred.label)" class="flex items-center gap-2">
                  <button
                    class="btn btn-ghost text-xs"
                    @click="beginRenameCredential(credType.type, cred.label)"
                  >
                    ✏️ Rename
                  </button>
                  <button
                    class="btn btn-ghost text-xs text-naonur-blood hover:text-naonur-blood/80"
                    @click="deleteCredential(credType.type, cred.label)"
                  >
                    🗑️ Delete
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Add Form -->
          <Transition
            enter-active-class="transition-all duration-200"
            enter-from-class="opacity-0 max-h-0"
            enter-to-class="opacity-100 max-h-[800px]"
            leave-active-class="transition-all duration-200"
            leave-from-class="opacity-100 max-h-[800px]"
            leave-to-class="opacity-0 max-h-0"
          >
            <div v-if="activeForm === credType.type" class="p-4 bg-naonur-void/50 border-t border-naonur-fog/20">
              <form @submit.prevent="saveCredential(credType.type)">
                <div class="space-y-3">
                  <div>
                    <label class="text-xs text-naonur-smoke block mb-1">Label / Account Name *</label>
                    <input
                      v-model="formData.label"
                      type="text"
                      placeholder="e.g., work, personal, my-account"
                      class="input-field w-full"
                      required
                    />
                  </div>

                  <div v-for="field in credType.fields" :key="field.name">
                    <label class="text-xs text-naonur-smoke block mb-1">
                      {{ field.display_name }} {{ field.required ? '*' : '(optional)' }}
                    </label>
                    <input
                      v-model="formData[field.name]"
                      :type="field.type === 'secret' ? 'password' : 'text'"
                      :placeholder="field.display_name"
                      class="input-field w-full font-mono text-sm"
                      :required="field.required"
                    />
                  </div>

                  <div class="flex gap-2 pt-2">
                    <button type="submit" class="btn btn-primary text-sm flex-1" :disabled="savingCredential">
                      {{ savingCredential ? 'Saving...' : 'Save' }}
                    </button>
                    <button 
                      type="button" 
                      class="btn btn-ghost text-sm"
                      @click="closeCredentialForm"
                      :disabled="savingCredential"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              </form>
            </div>
          </Transition>

          <!-- 1Password Vault Selection -->
          <div 
            v-if="credType.type === '1password' && credentialsByType['1password']?.length > 0"
            class="p-4 bg-naonur-void/30 border-t border-naonur-fog/20"
          >
            <div class="flex items-center justify-between mb-3">
              <h4 class="text-sm font-display text-naonur-bone">1Password Vaults</h4>
              <button
                v-if="vaults.length === 0"
                class="btn btn-ghost text-xs"
                :disabled="loadingVaults"
                @click="load1PasswordVaults"
              >
                {{ loadingVaults ? 'Loading...' : 'Load Vaults' }}
              </button>
            </div>

            <div v-if="loadingVaults" class="text-xs text-naonur-ash animate-pulse">
              Loading vaults...
            </div>

            <div v-else-if="vaults.length > 0" class="space-y-2">
              <div 
                v-for="vault in vaults" 
                :key="vault.id"
                class="flex items-center justify-between px-3 py-2 rounded bg-naonur-fog/10"
              >
                <div class="flex items-center gap-2">
                  <span class="text-sm text-naonur-bone">{{ vault.name }}</span>
                  <span v-if="defaultVault === vault.id" class="text-xs px-2 py-0.5 rounded-full bg-naonur-gold/20 text-naonur-gold">
                    Default
                  </span>
                </div>
                <button
                  v-if="defaultVault !== vault.id"
                  class="btn btn-ghost text-xs"
                  :disabled="settingDefaultVault"
                  @click="setDefault1PasswordVault(vault.id)"
                >
                  Set as Default
                </button>
              </div>
            </div>

            <p v-else class="text-xs text-naonur-smoke">
              No vaults found. Make sure your Service Account token has access to at least one vault.
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- Google Account -->
    <div class="naonur-card">
      <button class="w-full flex items-center justify-between" @click="toggleGoogleSection">
        <h2 class="font-display text-lg text-naonur-bone">Google Account</h2>
        <span class="text-naonur-smoke text-sm">{{ googleSectionOpen ? 'Hide' : 'Show' }}</span>
      </button>

      <Transition
        enter-active-class="transition-all duration-200"
        enter-from-class="opacity-0 max-h-0"
        enter-to-class="opacity-100 max-h-[900px]"
        leave-active-class="transition-all duration-200"
        leave-from-class="opacity-100 max-h-[900px]"
        leave-to-class="opacity-0 max-h-0"
      >
        <div v-if="googleSectionOpen" class="mt-4 space-y-4 overflow-hidden">
          <div class="p-3 rounded-lg border border-naonur-fog/30 bg-naonur-fog/10">
            <div class="flex items-center justify-between gap-3">
              <div>
                <p class="text-sm text-naonur-bone">OAuth Status</p>
                <p class="text-xs text-naonur-smoke">{{ googleStatus?.message || 'Unknown' }}</p>
              </div>
              <button class="btn btn-primary text-xs" :disabled="isConnectingOAuth" @click="handleConnectGoogle">
                {{ isConnectingOAuth ? 'Connecting…' : 'Connect' }}
              </button>
            </div>
          </div>

          <button class="btn btn-ghost text-xs" @click="googleConfigOpen = !googleConfigOpen">
            {{ googleConfigOpen ? 'Hide OAuth client config' : 'Show OAuth client config' }}
          </button>

          <div v-if="googleConfigOpen" class="p-3 rounded-lg border border-naonur-fog/30 bg-naonur-void/40 space-y-3">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <input v-model="googleClientId" class="input-field" placeholder="Google Client ID" />
              <input v-model="googleClientSecret" class="input-field" placeholder="Google Client Secret" type="password" />
            </div>

            <div class="p-3 border border-dashed border-naonur-fog/40 rounded-lg" :class="{ 'bg-naonur-gold/10': googleDragOver }"
              @dragover.prevent="googleDragOver = true" @dragleave.prevent="googleDragOver = false" @drop.prevent="onGoogleDrop">
              <p class="text-xs text-naonur-smoke mb-2">Upload or paste client_secret.json</p>
              <input type="file" accept="application/json" @change="onGoogleFileInput" />
              <textarea v-model="googleJsonInput" class="input-field w-full mt-2 min-h-[84px] font-mono text-xs" placeholder='{"installed":{"client_id":"...","client_secret":"..."}}'/>
              <button class="btn btn-ghost text-xs mt-2" @click="parseAndApplyGoogleJson(googleJsonInput)">Parse JSON</button>
            </div>

            <div class="flex gap-2">
              <button class="btn btn-secondary text-xs" :disabled="savingGoogleConfig" @click="saveGoogleConfig">{{ savingGoogleConfig ? 'Saving…' : 'Save Client Config' }}</button>
              <button class="btn btn-ghost text-xs" :disabled="testingGoogleConfig" @click="testGoogleConfig">{{ testingGoogleConfig ? 'Testing…' : 'Test Config' }}</button>
            </div>
          </div>
        </div>
      </Transition>
    </div>

    <!-- Custom Credential Type Modal -->
    <Transition
      enter-active-class="transition-all duration-200"
      enter-from-class="opacity-0"
      leave-active-class="transition-all duration-200"
      leave-to-class="opacity-0"
    >
      <div 
        v-if="showCustomTypeForm" 
        class="fixed inset-0 bg-naonur-void/80 flex items-center justify-center z-50"
        @click.self="showCustomTypeForm = false"
      >
        <div class="naonur-card max-w-2xl w-full m-4 max-h-[90vh] overflow-auto">
          <h2 class="font-display text-xl text-naonur-gold mb-4">Create Custom Credential Type</h2>
          
          <form @submit.prevent="saveCustomType">
            <div class="space-y-4">
              <div>
                <label class="text-sm text-naonur-bone block mb-1">Type ID (lowercase, no spaces)</label>
                <input
                  v-model="customTypeName"
                  type="text"
                  placeholder="e.g., github, openai, custom-api"
                  class="input-field w-full"
                  required
                />
              </div>

              <div>
                <label class="text-sm text-naonur-bone block mb-1">Display Name</label>
                <input
                  v-model="customTypeDisplayName"
                  type="text"
                  placeholder="e.g., GitHub, OpenAI, Custom API"
                  class="input-field w-full"
                  required
                />
              </div>

              <div>
                <div class="flex items-center justify-between mb-2">
                  <label class="text-sm text-naonur-bone">Fields</label>
                  <button
                    type="button"
                    class="btn btn-ghost text-xs"
                    @click="addCustomFieldRow"
                  >
                    + Add Field
                  </button>
                </div>

                <div class="space-y-2">
                  <div 
                    v-for="(field, index) in customTypeFields" 
                    :key="index"
                    class="grid grid-cols-12 gap-2"
                  >
                    <input
                      v-model="field.name"
                      type="text"
                      placeholder="Field name (e.g., api_key)"
                      class="input-field col-span-3"
                    />
                    <input
                      v-model="field.display_name"
                      type="text"
                      placeholder="Display name"
                      class="input-field col-span-4"
                    />
                    <select v-model="field.type" class="input-field col-span-2">
                      <option value="string">String</option>
                      <option value="secret">Secret</option>
                    </select>
                    <label class="col-span-2 flex items-center gap-2 text-xs text-naonur-ash">
                      <input v-model="field.required" type="checkbox" />
                      Required
                    </label>
                    <button
                      type="button"
                      class="btn btn-ghost text-xs col-span-1"
                      @click="removeCustomFieldRow(index)"
                    >
                      🗑️
                    </button>
                  </div>
                </div>
              </div>

              <div class="flex gap-2 pt-4 border-t border-naonur-fog/20">
                <button type="submit" class="btn btn-primary flex-1">
                  Create Type
                </button>
                <button 
                  type="button" 
                  class="btn btn-ghost"
                  @click="showCustomTypeForm = false"
                >
                  Cancel
                </button>
              </div>
            </div>
          </form>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.btn-primary {
  @apply bg-naonur-gold text-naonur-void hover:bg-naonur-gold/80;
}

.btn-secondary {
  @apply bg-naonur-fog/20 text-naonur-bone hover:bg-naonur-fog/30 border border-naonur-fog/30;
}

.btn-ghost {
  @apply text-naonur-ash hover:text-naonur-bone hover:bg-naonur-fog/20;
}

.input-field {
  @apply bg-naonur-void/50 border border-naonur-fog/30 rounded px-3 py-2 text-sm text-naonur-bone;
  @apply focus:outline-none focus:border-naonur-gold/50 focus:ring-1 focus:ring-naonur-gold/30;
  @apply placeholder:text-naonur-smoke/50;
}

.btn {
  @apply px-4 py-2 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed;
}
</style>
