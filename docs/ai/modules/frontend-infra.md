# Frontend Infrastructure

> **Location:** `src/` (stores, composables, workers, components, router)  
> **Lines:** ~2,200  
> **Purpose:** Vue 3 state management, reactive utilities, background workers

---

## Overview

Infrastructure for the Vue frontend: Pinia stores for global state, composables for reusable logic, web workers for background polling, and shared components.

---

## Structure

```
src/
├── stores/              # Pinia state stores
│   ├── auth.ts         # OAuth accounts + credentials
│   ├── permissions.ts  # macOS permission status
│   ├── config.ts       # App configuration
│   ├── monitor.ts      # Activity feed + manifests
│   └── profiles.ts     # User profiles (stub)
├── composables/         # Vue composables
│   ├── useToast.ts     # Global toast notifications
│   ├── useWorkerPoller.ts    # Web Worker polling wrapper
│   ├── useStatusPoller.ts    # Proxy status polling
│   ├── useActivityFeed.ts    # Real-time activity aggregation
│   └── useStateCache.ts      # LocalStorage caching
├── workers/             # Web Workers
│   ├── unified-poller.worker.ts   # Unified background poller
│   └── status-poller.worker.ts    # Legacy status poller
├── components/          # Shared Vue components
│   ├── common/          # Generic UI components
│   └── config/          # Config-specific components
├── router/              # Vue Router
│   └── index.ts
├── api.ts               # Typed Tauri IPC wrappers
└── main.ts              # Vue app entry point
```

---

## Pinia Stores

### auth.ts

```typescript
export const useAuthStore = defineStore('auth', {
  state: () => ({
    providers: [] as string[],
    accounts: [] as Account[],
    credentials: [] as Credential[],
    loading: false,
    error: null as string | null,
  }),
  
  actions: {
    async loadProviders() {
      this.providers = await invoke('auth_providers')
    },
    
    async loadAccounts(provider?: string) {
      this.accounts = await invoke('auth_accounts', { provider })
    },
    
    async startOAuth(provider: string, account: string, scopes: string[]) {
      const url = await invoke('auth_start_google_oauth', {
        account,
        scopes,
        clientId: this.googleClientId,
        clientSecret: this.googleClientSecret,
      })
      window.open(url, '_blank')
    },
    
    async revokeToken(provider: string, account: string) {
      await invoke('auth_revoke_token', { provider, account })
      await this.loadAccounts(provider)
    },
  },
})
```

---

### permissions.ts

```typescript
export const usePermissionsStore = defineStore('permissions', {
  state: () => ({
    permissions: [] as Permission[],
    definitions: [] as PermissionDefinition[],
  }),
  
  actions: {
    async loadPermissions() {
      this.permissions = await invoke('get_permissions')
    },
    
    async requestPermission(id: string) {
      const result = await invoke('request_permission', { id })
      await this.loadPermissions()
      return result
    },
    
    async openSettings(id: string) {
      await invoke('open_permission_settings', { id })
    },
  },
  
  getters: {
    criticalPermissions: (state) => state.permissions.filter(p => p.critical),
    grantedCount: (state) => state.permissions.filter(p => p.status === 'granted').length,
  },
})
```

---

## Composables

### useToast.ts

Global toast notification system:

```typescript
export function useToast() {
  const toasts = ref<Toast[]>([])
  
  function showToast(message: string, type: 'success' | 'error' | 'info' = 'info', duration = 3000) {
    const id = Date.now()
    toasts.value.push({ id, message, type, visible: true })
    
    setTimeout(() => {
      toasts.value = toasts.value.filter(t => t.id !== id)
    }, duration)
  }
  
  return { toasts, showToast }
}
```

**Usage:**
```typescript
const { showToast } = useToast()

try {
  await doSomething()
  showToast('Success!', 'success')
} catch (e) {
  showToast(e.message, 'error')
}
```

---

### useWorkerPoller.ts

Web Worker-based polling (prevents UI thread blocking):

```typescript
export function useWorkerPoller(
  workerPath: string,
  pollingInterval: number = 5000
) {
  const data = ref(null)
  const worker = ref<Worker | null>(null)
  
  function start() {
    worker.value = new Worker(new URL(workerPath, import.meta.url))
    worker.value.postMessage({ action: 'start', interval: pollingInterval })
    
    worker.value.onmessage = (e) => {
      data.value = e.data
    }
  }
  
  function stop() {
    worker.value?.postMessage({ action: 'stop' })
    worker.value?.terminate()
  }
  
  onUnmounted(() => stop())
  
  return { data, start, stop }
}
```

---

### useActivityFeed.ts

Real-time activity event aggregation:

```typescript
export function useActivityFeed() {
  const events = ref<ActivityEvent[]>([])
  const { data, start, stop } = useWorkerPoller('/workers/unified-poller.worker.ts', 2000)
  
  watch(data, (newData) => {
    if (newData?.events) {
      events.value = [...newData.events, ...events.value].slice(0, 100)
    }
  })
  
  return { events, start, stop }
}
```

---

## Web Workers

### unified-poller.worker.ts

Background poller for activity events:

```typescript
let intervalId: number | null = null

self.onmessage = async (e) => {
  if (e.data.action === 'start') {
    intervalId = setInterval(async () => {
      const events = await invoke('get_events', { limit: 20 })
      self.postMessage({ events })
    }, e.data.interval)
  } else if (e.data.action === 'stop') {
    if (intervalId) clearInterval(intervalId)
  }
}
```

**Why Web Workers?** Prevents `setInterval()` from blocking the main thread during heavy rendering.

---

## Typed API Layer

### api.ts

Type-safe Tauri IPC wrappers:

```typescript
import { invoke as tauriInvoke } from '@tauri-apps/api/tauri'

export async function invoke<T = any>(
  command: string,
  args?: Record<string, any>
): Promise<T> {
  try {
    return await tauriInvoke(command, args)
  } catch (error) {
    console.error(`IPC Error [${command}]:`, error)
    throw error
  }
}

// Typed helpers
export const authApi = {
  providers: () => invoke<string[]>('auth_providers'),
  accounts: (provider?: string) => invoke<Account[]>('auth_accounts', { provider }),
  startOAuth: (params: OAuthParams) => invoke<string>('auth_start_google_oauth', params),
}

export const permissionsApi = {
  getAll: () => invoke<Permission[]>('get_permissions'),
  request: (id: string) => invoke<Permission>('request_permission', { id }),
  openSettings: (id: string) => invoke<void>('open_permission_settings', { id }),
}
```

---

## Shared Components

### StatusBadge.vue

```vue
<template>
  <span :class="badgeClass">{{ statusText }}</span>
</template>

<script setup lang="ts">
const props = defineProps<{ status: string }>()

const badgeClass = computed(() => ({
  'bg-green-500': props.status === 'granted',
  'bg-red-500': props.status === 'denied',
  'bg-yellow-500': props.status === 'not_determined',
  'bg-gray-500': props.status === 'unknown',
}))

const statusText = computed(() => props.status.replace('_', ' ').toUpperCase())
</script>
```

---

### TabNav.vue

```vue
<template>
  <div class="tabs">
    <button
      v-for="tab in tabs"
      :key="tab.id"
      :class="{ active: modelValue === tab.id }"
      @click="$emit('update:modelValue', tab.id)"
    >
      {{ tab.label }}
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  tabs: Array<{ id: string; label: string }>
  modelValue: string
}>()

defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()
</script>
```

---

## Router Configuration

```typescript
// router/index.ts
const routes = [
  { path: '/', component: DashboardView },
  { path: '/auth', component: AuthView },
  { path: '/permissions', component: PermissionsView },
  { path: '/integrations', component: IntegrationsView },
  { path: '/config', component: ConfigView },
  { path: '/monitor', component: MonitorView },
  { path: '/profiles', component: ProfilesView },
  { path: '/mcp', component: MCPView },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})
```

---

## Anti-Patterns

❌ **Inline `setInterval()` in components:**
```vue
<script setup>
onMounted(() => {
  setInterval(() => {
    // Polling logic
  }, 5000)  // ❌ Blocks main thread
})
</script>
```

✅ **Use composables + workers:**
```vue
<script setup>
import { useStatusPoller } from '@/composables/useStatusPoller'

const { status, start, stop } = useStatusPoller()

onMounted(() => start())
onUnmounted(() => stop())
</script>
```

---

*For view-specific details, see [frontend-views.md](frontend-views.md)*
