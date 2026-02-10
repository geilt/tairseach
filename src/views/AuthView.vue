<script setup lang="ts">
import { computed, onMounted, onActivated, ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import type { AccountInfo } from '@/stores/auth'

const store = useAuthStore()
const actionMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

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
        
        <div class="space-y-3">
          <div 
            v-for="provider in store.providers" 
            :key="provider"
            class="p-4 rounded-lg bg-naonur-fog/10 border border-naonur-fog/20 hover:border-naonur-gold/30 transition-colors cursor-pointer group"
            @click="provider === 'google' ? handleConnectGoogle() : null"
          >
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <span class="text-2xl">{{ getProviderIcon(provider) }}</span>
                <div>
                  <h3 class="font-display text-naonur-bone group-hover:text-naonur-gold transition-colors">
                    {{ provider.charAt(0).toUpperCase() + provider.slice(1) }}
                  </h3>
                  <p class="text-xs text-naonur-smoke">
                    {{ provider === 'google' ? 'Gmail, Drive, Calendar, Contacts' : 'OAuth 2.0 provider' }}
                  </p>
                </div>
              </div>
              <button class="btn btn-primary text-sm">
                Connect
              </button>
            </div>
          </div>
        </div>

        <div class="mt-4 p-3 rounded-lg bg-naonur-rust/10 border border-naonur-rust/30">
          <p class="text-xs text-naonur-rust">
            <strong>Note:</strong> OAuth PKCE flow is not yet implemented in the UI. 
            Use the socket API (<code class="font-mono">auth.store</code>) or CLI to add credentials for now.
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

.btn-ghost {
  @apply text-naonur-ash hover:text-naonur-bone hover:bg-naonur-fog/20;
}
</style>
