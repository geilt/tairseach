<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'

interface ProxyStatus {
  running: boolean
  socket_path?: string | null
}

interface PermissionSummary {
  granted: number
  total: number
}

interface ManifestSummary {
  capabilities_loaded: number
  tools_available: number
  mcp_exposed: number
}

interface ActivityEvent {
  id: string
  timestamp: string
  event_type: string
  source: string
  message: string
  metadata?: Record<string, unknown>
}

const router = useRouter()
const loading = ref(true)
const proxyStatus = ref<ProxyStatus>({ running: false })
const permissionSummary = ref<PermissionSummary>({ granted: 0, total: 11 })
const manifestSummary = ref<ManifestSummary>({ capabilities_loaded: 0, tools_available: 0, mcp_exposed: 0 })
const recentActivity = ref<ActivityEvent[]>([])

const mcpBridgeConnected = computed(() => manifestSummary.value.mcp_exposed > 0)

async function loadDashboard() {
  loading.value = true
  try {
    const [proxy, permissions, manifests, activity] = await Promise.all([
      invoke<ProxyStatus>('get_proxy_status'),
      invoke<Array<{ status: string }>>('check_all_permissions'),
      invoke<ManifestSummary>('get_manifest_summary'),
      invoke<ActivityEvent[]>('get_events', { limit: 10 }),
    ])

    proxyStatus.value = proxy
    permissionSummary.value = {
      granted: permissions.filter((p) => p.status === 'granted').length,
      total: 11,
    }
    manifestSummary.value = manifests
    recentActivity.value = activity.reverse()
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  void loadDashboard()
})
</script>

<template>
  <section class="animate-fade-in">
    <div class="mb-6 flex items-center justify-between">
      <div>
        <h1 class="font-display text-2xl tracking-wider text-naonur-gold">Dashboard</h1>
        <p class="text-sm text-naonur-ash">System health, manifest posture, and recent activity.</p>
      </div>
      <button class="rounded-md border border-naonur-fog px-3 py-2 text-sm hover:bg-naonur-mist" @click="loadDashboard">
        Refresh
      </button>
    </div>

    <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
      <article class="naonur-card dashboard-card">
        <p class="text-xs uppercase tracking-wide text-naonur-smoke">Socket status</p>
        <p class="mt-2 text-xl" :class="proxyStatus.running ? 'text-naonur-moss' : 'text-red-300'">
          {{ proxyStatus.running ? 'Connected' : 'Disconnected' }}
        </p>
      </article>

      <article class="naonur-card dashboard-card">
        <p class="text-xs uppercase tracking-wide text-naonur-smoke">MCP bridge status</p>
        <p class="mt-2 text-xl" :class="mcpBridgeConnected ? 'text-naonur-moss' : 'text-naonur-rust'">
          {{ mcpBridgeConnected ? 'Ready' : 'Not exposed' }}
        </p>
      </article>

      <article class="naonur-card dashboard-card">
        <p class="text-xs uppercase tracking-wide text-naonur-smoke">Permissions</p>
        <p class="mt-2 text-xl text-naonur-bone">{{ permissionSummary.granted }} / {{ permissionSummary.total }} granted</p>
      </article>
    </div>

    <div class="mt-5 grid grid-cols-1 gap-4 lg:grid-cols-2">
      <article class="naonur-card dashboard-card">
        <h2 class="mb-3 text-sm uppercase tracking-wide text-naonur-smoke">Recent activity</h2>
        <div v-if="loading" class="space-y-2">
          <div v-for="n in 4" :key="n" class="h-8 animate-pulse rounded bg-naonur-mist/50" />
        </div>
        <ul v-else class="space-y-2">
          <li v-for="item in recentActivity" :key="item.id" class="rounded-md border border-naonur-fog/50 px-3 py-2 text-sm">
            <p class="truncate text-naonur-bone">{{ item.message }}</p>
            <p class="text-xs text-naonur-smoke">{{ item.source }} · {{ item.timestamp || '—' }}</p>
          </li>
        </ul>
      </article>

      <article class="naonur-card dashboard-card">
        <h2 class="mb-3 text-sm uppercase tracking-wide text-naonur-smoke">Manifest summary</h2>
        <div class="space-y-3 text-naonur-bone">
          <p>Capabilities loaded: <strong>{{ manifestSummary.capabilities_loaded }}</strong></p>
          <p>Tools available: <strong>{{ manifestSummary.tools_available }}</strong></p>
          <p>MCP-exposed: <strong>{{ manifestSummary.mcp_exposed }}</strong></p>
        </div>

        <h3 class="mb-2 mt-6 text-sm uppercase tracking-wide text-naonur-smoke">Quick actions</h3>
        <div class="flex gap-3">
          <button class="rounded-md border border-naonur-fog px-3 py-2 text-sm hover:bg-naonur-mist" @click="router.push('/permissions')">
            Check all permissions
          </button>
          <button class="rounded-md border border-naonur-fog px-3 py-2 text-sm hover:bg-naonur-mist" @click="router.push('/activity')">
            View logs
          </button>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.dashboard-card {
  contain: layout style paint;
}
</style>
