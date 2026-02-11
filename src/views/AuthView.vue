<script setup lang="ts">
import { computed, onMounted, onActivated, ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import type { AccountInfo } from '@/stores/auth'

const store = useAuthStore()
const actionMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)
const showTokenForm = ref(false)
const tokenFormData = ref({
  provider: '',
  account: '',
  tokenType: 'Bearer',
  accessToken: '',
  refreshToken: '',
  scopes: '',
  expiryDays: '365',
})

onMounted(() => {
  void store.init()
})

onActivated(() => {
  void store.loadStatus({ silent: true })
  void store.loadAccounts({ silent: true })
})

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

function getProviderIcon(provider: string): string {
  const icons: Record<string, string> = {
    google: '🔵',
    microsoft: '🔷',
    github: '🐙',
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
  if (success) {
    actionMessage.value = { type: 'success', text: `Refreshed ${account.account}` }
  } else {
    actionMessage.value = { type: 'error', text: store.error || 'Refresh failed' }
  }
  setTimeout(() => actionMessage.value = null, 3000)
}

async function handleRevoke(account: AccountInfo) {
  if (!confirm(`Revoke access for ${account.account}? This will delete the stored credentials.`)) {
    return
  }
  
  actionMessage.value = null
  const success = await store.revokeToken(account.provider, account.account)
  if (success) {
    actionMessage.value = { type: 'success', text: `Revoked ${account.account}` }
  } else {
    actionMessage.value = { type: 'error', text: store.error || 'Revoke failed' }
  }
  setTimeout(() => actionMessage.value = null, 3000)
}

function handleConnectGoogle() {
  actionMessage.value = {
    type: 'error',
    text: 'OAuth PKCE flow not yet implemented. Use the socket API or CLI for now.',
  }
  setTimeout(() => actionMessage.value = null, 5000)
}

function resetTokenForm() {
  tokenFormData.value = {
    provider: '',
    account: '',
    tokenType: 'Bearer',
    accessToken: '',
    refreshToken: '',
    scopes: '',
    expiryDays: '365',
  }
  showTokenForm.value = false
}

async function handleSubmitToken() {
  if (!tokenFormData.value.provider || !tokenFormData.value.account || !tokenFormData.value.accessToken) {
    actionMessage.value = {
      type: 'error',
      text: 'Provider, account, and access token are required',
    }
    setTimeout(() => actionMessage.value = null, 3000)
    return
  }

  const expiryDate = new Date()
  expiryDate.setDate(expiryDate.getDate() + parseInt(tokenFormData.value.expiryDays || '365'))

  const scopes = tokenFormData.value.scopes
    .split(',')
    .map(s => s.trim())
    .filter(Boolean)

  const record = {
    provider: tokenFormData.value.provider,
    account: tokenFormData.value.account,
    client_id: '',
    client_secret: '',
    token_type: tokenFormData.value.tokenType,
    access_token: tokenFormData.value.accessToken,
    refresh_token: tokenFormData.value.refreshToken,
    expiry: expiryDate.toISOString(),
    scopes,
  }

  const success = await store.storeToken(record)
  if (success) {
    actionMessage.value = {
      type: 'success',
      text: `Added ${tokenFormData.value.account}`,
    }
    resetTokenForm()
  } else {
    actionMessage.value = {
      type: 'error',
      text: store.error || 'Failed to store token',
    }
  }
  setTimeout(() => actionMessage.value = null, 3000)
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
          OAuth connections for CLI tools and agents.
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

    <!-- Loading State -->
    <div v-if="store.loading && !store.status" class="naonur-card mb-6 text-center py-8">
      <p class="text-naonur-ash animate-pulse">Loading auth status...</p>
    </div>

    <template v-else>
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

      <!-- Connected Accounts -->
      <div class="naonur-card mb-6">
        <div class="flex items-center justify-between mb-4">
          <h2 class="font-display text-lg text-naonur-bone">Connected Accounts</h2>
          <span class="text-sm text-naonur-smoke">
            {{ store.accounts.length }} account{{ store.accounts.length === 1 ? '' : 's' }}
          </span>
        </div>

        <div v-if="store.accounts.length === 0" class="text-center py-8 text-naonur-ash">
          <p>No accounts connected yet.</p>
          <p class="text-sm text-naonur-smoke mt-2">Use the buttons below to connect a provider.</p>
        </div>

        <div v-else class="space-y-3">
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
                    <span v-if="account.last_refreshed">
                      Last refresh: {{ new Date(account.last_refreshed).toLocaleString() }}
                    </span>
                  </div>
                  
                  <div v-if="account.scopes.length > 0" class="mt-2">
                    <p class="text-xs text-naonur-smoke mb-1">Scopes:</p>
                    <div class="flex flex-wrap gap-1">
                      <span 
                        v-for="scope in account.scopes" 
                        :key="scope"
                        class="text-xs px-2 py-0.5 rounded bg-naonur-void/50 text-naonur-ash font-mono"
                      >
                        {{ scope }}
                      </span>
                    </div>
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
                  🗑️ Revoke
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Connect Provider -->
      <div class="naonur-card">
        <h2 class="font-display text-lg text-naonur-bone mb-4">Connect Provider</h2>
        
        <!-- OAuth Providers -->
        <div class="mb-4">
          <h3 class="text-sm text-naonur-ash mb-2">OAuth 2.0</h3>
          <div class="space-y-2">
            <div 
              v-for="provider in store.providers" 
              :key="provider"
              class="p-3 rounded-lg bg-naonur-fog/10 border border-naonur-fog/20 hover:border-naonur-gold/30 transition-colors cursor-pointer group"
              @click="provider === 'google' ? handleConnectGoogle() : null"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-3">
                  <span class="text-xl">{{ getProviderIcon(provider) }}</span>
                  <div>
                    <h4 class="font-display text-sm text-naonur-bone group-hover:text-naonur-gold transition-colors">
                      {{ provider.charAt(0).toUpperCase() + provider.slice(1) }}
                    </h4>
                    <p class="text-xs text-naonur-smoke">
                      {{ provider === 'google' ? 'Gmail, Drive, Calendar, Contacts' : 'OAuth 2.0 provider' }}
                    </p>
                  </div>
                </div>
                <button class="btn btn-primary text-xs px-3 py-1">
                  Connect
                </button>
              </div>
            </div>
          </div>
          
          <div class="mt-3 p-2 rounded-lg bg-naonur-rust/10 border border-naonur-rust/30">
            <p class="text-xs text-naonur-rust">
              OAuth PKCE flow UI coming soon. Use socket API (<code class="font-mono">auth.store</code>) or CLI for now.
            </p>
          </div>
        </div>

        <!-- Manual Token Entry -->
        <div class="pt-4 border-t border-naonur-fog/20">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm text-naonur-ash">Manual Token / API Key</h3>
            <button 
              v-if="!showTokenForm"
              class="btn btn-secondary text-xs px-3 py-1"
              @click="showTokenForm = true"
            >
              + Add Token
            </button>
          </div>

          <Transition
            enter-active-class="transition-all duration-200"
            enter-from-class="opacity-0 max-h-0"
            enter-to-class="opacity-100 max-h-[600px]"
            leave-active-class="transition-all duration-200"
            leave-from-class="opacity-100 max-h-[600px]"
            leave-to-class="opacity-0 max-h-0"
          >
            <div v-if="showTokenForm" class="p-4 rounded-lg bg-naonur-fog/10 border border-naonur-fog/20">
              <form @submit.prevent="handleSubmitToken">
                <div class="space-y-3">
                  <div class="grid grid-cols-2 gap-3">
                    <div>
                      <label class="text-xs text-naonur-smoke block mb-1">Provider *</label>
                      <input
                        v-model="tokenFormData.provider"
                        type="text"
                        placeholder="e.g., oura, jira, 1password"
                        class="input-field w-full"
                        required
                      />
                    </div>
                    <div>
                      <label class="text-xs text-naonur-smoke block mb-1">Account / Label *</label>
                      <input
                        v-model="tokenFormData.account"
                        type="text"
                        placeholder="e.g., user@example.com"
                        class="input-field w-full"
                        required
                      />
                    </div>
                  </div>

                  <div>
                    <label class="text-xs text-naonur-smoke block mb-1">Access Token / API Key *</label>
                    <input
                      v-model="tokenFormData.accessToken"
                      type="password"
                      placeholder="Paste your token here"
                      class="input-field w-full font-mono text-sm"
                      required
                    />
                  </div>

                  <div>
                    <label class="text-xs text-naonur-smoke block mb-1">Refresh Token (optional)</label>
                    <input
                      v-model="tokenFormData.refreshToken"
                      type="password"
                      placeholder="Leave empty for static tokens"
                      class="input-field w-full font-mono text-sm"
                    />
                  </div>

                  <div class="grid grid-cols-2 gap-3">
                    <div>
                      <label class="text-xs text-naonur-smoke block mb-1">Token Type</label>
                      <select v-model="tokenFormData.tokenType" class="input-field w-full">
                        <option value="Bearer">Bearer</option>
                        <option value="token">token</option>
                      </select>
                    </div>
                    <div>
                      <label class="text-xs text-naonur-smoke block mb-1">Expiry (days)</label>
                      <input
                        v-model="tokenFormData.expiryDays"
                        type="number"
                        min="1"
                        max="3650"
                        class="input-field w-full"
                      />
                    </div>
                  </div>

                  <div>
                    <label class="text-xs text-naonur-smoke block mb-1">Scopes (comma-separated, optional)</label>
                    <input
                      v-model="tokenFormData.scopes"
                      type="text"
                      placeholder="e.g., read, write, admin"
                      class="input-field w-full font-mono text-sm"
                    />
                  </div>

                  <div class="flex gap-2 pt-2">
                    <button type="submit" class="btn btn-primary text-sm flex-1" :disabled="store.loading">
                      {{ store.loading ? 'Saving...' : 'Save Token' }}
                    </button>
                    <button 
                      type="button" 
                      class="btn btn-ghost text-sm"
                      @click="resetTokenForm"
                      :disabled="store.loading"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              </form>
            </div>
          </Transition>

          <p class="text-xs text-naonur-smoke mt-3">
            For services like Oura, Jira, or 1Password that use long-lived API tokens. Static tokens won't auto-refresh.
          </p>
        </div>
      </div>
    </template>
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
