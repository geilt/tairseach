# Frontend Views

> **Location:** `src/views/`  
> **Files:** 10  
> **Lines:** 3,823  
> **Tech Stack:** Vue 3 + TypeScript + Pinia

---

## Overview

All main application views. Each view follows a consistent pattern: reactive state management via Pinia stores, composables for side effects (polling, toasts), and TypeScript types for IPC calls.

---

## View Listing

| File | Lines | Purpose |
|------|-------|---------|
| `DashboardView.vue` | ~450 | Main dashboard — server status, quick stats, recent activity |
| `AuthView.vue` | ~620 | OAuth configuration + account management + credential CRUD |
| `PermissionsView.vue` | ~480 | macOS permission status + request UI + system pref links |
| `IntegrationsView.vue` | ~380 | External API integrations (Google, Jira, Oura, etc.) |
| `ProfilesView.vue` | ~210 | User profiles (stub — future multi-user support) |
| `ConfigView.vue` | ~520 | App configuration (providers, models, node settings, exec approvals) |
| `MonitorView.vue` | ~430 | Activity feed + manifest stats + socket health |
| `GoogleSettingsView.vue` | ~340 | Google OAuth setup (client ID/secret, scopes, test) |
| `MCPView.vue` | ~290 | MCP server configuration + tool testing |
| `ActivityView.vue` | ~110 | Detailed activity log viewer |

---

## Shared Patterns

### State Management

All views use Pinia stores:

```typescript
<script setup lang="ts">
import { useAuthStore } from '@/stores/auth'
import { usePermissionsStore } from '@/stores/permissions'

const authStore = useAuthStore()
const permissionsStore = usePermissionsStore()

// Reactive state
const accounts = computed(() => authStore.accounts)
const permissions = computed(() => permissionsStore.permissions)
</script>
```

---

### Composables

```typescript
import { useToast } from '@/composables/useToast'
import { useStatusPoller } from '@/composables/useStatusPoller'

const { showToast } = useToast()
const { status, startPolling, stopPolling } = useStatusPoller()

onMounted(() => startPolling())
onUnmounted(() => stopPolling())
```

---

### Tauri IPC

All IPC calls use typed wrappers from `@/api.ts`:

```typescript
import { invoke } from '@/api'

const accounts = await invoke('auth_accounts', { provider: 'google' })
const perms = await invoke('get_permissions')
```

**Never use:**
```typescript
import { invoke } from '@tauri-apps/api/tauri'  // ❌ NO — use typed wrapper
```

---

## View-Specific Details

### DashboardView

**State:**
- Proxy server status (running/stopped, socket path)
- Account count
- Permission summary (granted/denied counts)
- Recent activity (last 10 events)

**Actions:**
- Start/stop proxy server
- Navigate to sub-views

**Polling:** Uses `useStatusPoller()` for server status updates every 5s

---

### AuthView

**State:**
- OAuth providers
- Accounts per provider
- Credentials (custom types)
- Token expiry status

**Actions:**
- Start OAuth flow (opens browser)
- Revoke tokens
- Add/edit/delete custom credentials
- View token scopes

**Key Feature:** Google OAuth setup embedded (consolidated from GoogleSettingsView)

---

### PermissionsView

**State:**
- All macOS permissions with status
- Critical vs optional flags
- Last checked timestamps

**Actions:**
- Request permission (triggers native prompt)
- Open System Preferences for manual grant
- Refresh status

**Display:** Color-coded status badges (green=granted, red=denied, yellow=not determined)

---

### MonitorView

**State:**
- Real-time activity feed (from `useActivityFeed()`)
- Manifest summary (loaded manifests, tool count)
- Socket health check

**Actions:**
- Filter events by namespace
- Test MCP tool invocation
- View manifest details

**Polling:** Uses `useWorkerPoller()` for activity feed updates (non-blocking)

---

### ConfigView

**Tabs:**
1. **Providers** — AI provider configs (OpenAI, Anthropic, etc.)
2. **Models** — Model selection per provider
3. **Node** — OpenClaw node configuration
4. **Exec Approvals** — Command execution approval settings

**Actions:**
- Save provider API keys
- Select default models
- Configure node URL
- Set exec approval rules

---

## Component Usage

Views import reusable components from `@/components/`:

```vue
<template>
  <TabNav :tabs="tabs" v-model="activeTab" />
  <StatusBadge :status="permission.status" />
  <Toast v-if="toast.visible" :message="toast.message" />
</template>

<script setup>
import TabNav from '@/components/common/TabNav.vue'
import StatusBadge from '@/components/common/StatusBadge.vue'
import Toast from '@/components/common/Toast.vue'
</script>
```

---

## Type Safety

All views use TypeScript types:

```typescript
interface Account {
  provider: string
  account: string
  scopes: string[]
  expiry: string
  last_refreshed?: string
}

interface Permission {
  id: string
  name: string
  description: string
  status: 'granted' | 'denied' | 'not_determined' | 'restricted' | 'unknown'
  critical: boolean
  last_checked?: string
}
```

**Source:** Types match Rust structs exactly (validated via `serde` serialization)

---

## Recent Refactorings

**Branch:** `refactor/vue-dry` (ready for merge)

**Changes:**
- Extracted shared view header pattern
- Consolidated state initialization logic
- Standardized error handling with `useToast()`
- Removed inline polling (replaced with composables)

---

*For state management, see [frontend-infra.md](frontend-infra.md)*  
*For pattern templates, see [patterns/view-pattern.md](../patterns/view-pattern.md)*
