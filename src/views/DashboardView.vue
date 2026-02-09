<script setup lang="ts">
import { computed, onActivated, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'
import { loadStateCache, saveStateCache } from '@/composables/useStateCache'

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

interface DashboardState {
  proxyStatus: ProxyStatus
  permissionSummary: PermissionSummary
  manifestSummary: ManifestSummary
  recentActivity: ActivityEvent[]
}

const router = useRouter()
const refreshing = ref(false)
const lastUpdated = ref<string | null>(null)

const proxyStatus = ref<ProxyStatus>({ running: false })
const permissionSummary = ref<PermissionSummary>({ granted: 0, total: 11 })
const manifestSummary = ref<ManifestSummary>({ capabilities_loaded: 0, tools_available: 0, mcp_exposed: 0 })
const recentActivity = ref<ActivityEvent[]>([])

const animGranted = ref(0)
const animCaps = ref(0)
const animTools = ref(0)
const animMcp = ref(0)

const mcpBridgeConnected = computed(() => manifestSummary.value.mcp_exposed > 0)

function hydrateCache() {
  const cached = loadStateCache<DashboardState>('dashboard')
  if (!cached) return
  proxyStatus.value = cached.data.proxyStatus
  permissionSummary.value = cached.data.permissionSummary
  manifestSummary.value = cached.data.manifestSummary
  recentActivity.value = cached.data.recentActivity
  lastUpdated.value = cached.lastUpdated
}

function persistCache() {
  const entry = saveStateCache<DashboardState>('dashboard', {
    proxyStatus: proxyStatus.value,
    permissionSummary: permissionSummary.value,
    manifestSummary: manifestSummary.value,
    recentActivity: recentActivity.value,
  })
  lastUpdated.value = entry.lastUpdated
}

function tween(from: number, to: number, setter: (n: number) => void, duration = 220) {
  if (from === to) return setter(to)
  const start = performance.now()
  const step = (t: number) => {
    const p = Math.min(1, (t - start) / duration)
    const eased = 1 - Math.pow(1 - p, 3)
    setter(Math.round(from + (to - from) * eased))
    if (p < 1) requestAnimationFrame(step)
  }
  requestAnimationFrame(step)
}

watch(
  () => permissionSummary.value.granted,
  (to, from = 0) => tween(from, to, (n) => (animGranted.value = n)),
  { immediate: true },
)
watch(() => manifestSummary.value.capabilities_loaded, (to, from = 0) => tween(from, to, (n) => (animCaps.value = n)), { immediate: true })
watch(() => manifestSummary.value.tools_available, (to, from = 0) => tween(from, to, (n) => (animTools.value = n)), { immediate: true })
watch(() => manifestSummary.value.mcp_exposed, (to, from = 0) => tween(from, to, (n) => (animMcp.value = n)), { immediate: true })

function mergeActivity(next: ActivityEvent[]) {
  const byId = new Map(recentActivity.value.map((item) => [item.id, item]))
  recentActivity.value = next.map((item) => {
    const existing = byId.get(item.id)
    return existing ? { ...existing, ...item } : item
  })
}

async function loadDashboard(silent = false) {
  if (!silent) refreshing.value = true
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
    mergeActivity(activity.slice().reverse())
    persistCache()
  } finally {
    if (!silent) refreshing.value = false
  }
}

function refreshDashboard() {
  void loadDashboard(false)
}

onMounted(() => {
  hydrateCache()
  void loadDashboard(true)
})

onActivated(() => {
  void loadDashboard(true)
})
</script>

<template>
  <section class="animate-fade-in">
    <div class="mb-6 flex items-center justify-between">
      <div>
        <h1 class="font-display text-2xl tracking-wider text-naonur-gold">Dashboard</h1>
        <p class="text-sm text-naonur-ash">System health, manifest posture, and recent activity.</p>
      </div>
      <button class="rounded-md border border-naonur-fog px-3 py-2 text-sm hover:bg-naonur-mist" @click="refreshDashboard">
        {{ refreshing ? 'Refreshing…' : 'Refresh' }}
      </button>
    </div>

    <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
      <article class="naonur-card dashboard-card min-h-[96px]">
        <p class="text-xs uppercase tracking-wide text-naonur-smoke">Socket status</p>
        <Transition name="fade" mode="out-in">
          <p class="mt-2 text-xl" :class="proxyStatus.running ? 'text-naonur-moss' : 'text-red-300'" :key="String(proxyStatus.running)">
            {{ proxyStatus.running ? 'Connected' : 'Disconnected' }}
          </p>
        </Transition>
      </article>

      <article class="naonur-card dashboard-card min-h-[96px]">
        <p class="text-xs uppercase tracking-wide text-naonur-smoke">MCP bridge status</p>
        <Transition name="fade" mode="out-in">
          <p class="mt-2 text-xl" :class="mcpBridgeConnected ? 'text-naonur-moss' : 'text-naonur-rust'" :key="String(mcpBridgeConnected)">
            {{ mcpBridgeConnected ? 'Ready' : 'Not exposed' }}
          </p>
        </Transition>
      </article>

      <article class="naonur-card dashboard-card min-h-[96px]">
        <p class="text-xs uppercase tracking-wide text-naonur-smoke">Permissions</p>
        <p class="mt-2 text-xl text-naonur-bone tabular-nums">{{ animGranted }} / {{ permissionSummary.total }} granted</p>
      </article>
    </div>

    <div class="mt-5 grid grid-cols-1 gap-4 lg:grid-cols-2">
      <article class="naonur-card dashboard-card min-h-[420px]">
        <h2 class="mb-3 text-sm uppercase tracking-wide text-naonur-smoke">Recent activity</h2>
        <ul class="space-y-2 min-h-[320px]">
          <li v-for="item in recentActivity" :key="item.id" class="rounded-md border border-naonur-fog/50 px-3 py-2 text-sm">
            <p class="truncate text-naonur-bone">{{ item.message }}</p>
            <p class="text-xs text-naonur-smoke">{{ item.source }} · {{ item.timestamp || '—' }}</p>
          </li>
          <li v-if="recentActivity.length === 0" class="rounded-md border border-naonur-fog/50 px-3 py-2 text-sm text-naonur-ash">No activity yet.</li>
        </ul>
      </article>

      <article class="naonur-card dashboard-card min-h-[420px]">
        <h2 class="mb-3 text-sm uppercase tracking-wide text-naonur-smoke">Manifest summary</h2>
        <div class="space-y-3 text-naonur-bone tabular-nums min-h-[110px]">
          <p>Capabilities loaded: <strong>{{ animCaps }}</strong></p>
          <p>Tools available: <strong>{{ animTools }}</strong></p>
          <p>MCP-exposed: <strong>{{ animMcp }}</strong></p>
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

.fade-enter-active,
.fade-leave-active {
  transition: opacity 140ms ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
