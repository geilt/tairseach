<script setup lang="ts">
import { onMounted, onUnmounted, onActivated, computed, ref, watch } from 'vue'
import StatusBadge from '../components/common/StatusBadge.vue'
import { usePermissionsStore } from '../stores/permissions'

const store = usePermissionsStore()

const refreshTimer = ref<number | null>(null)
const flashTimers = new Set<number>()

// Load cached state first, then soft refresh in background.
onMounted(async () => {
  await store.init()

  refreshTimer.value = window.setInterval(() => {
    store.loadPermissions({ silent: true })
  }, 12_000)
})

onActivated(() => {
  void store.loadPermissions({ silent: true })
  void store.loadDefinitions({ silent: true })
})

onUnmounted(() => {
  if (refreshTimer.value != null) {
    clearInterval(refreshTimer.value)
    refreshTimer.value = null
  }

  for (const timer of flashTimers) {
    clearTimeout(timer)
  }
  flashTimers.clear()
})

// Combine permissions with their icons from definitions
const permissionsWithIcons = computed(() => {
  return store.permissions.map(p => ({
    ...p,
    icon: store.getIcon(p.id),
    iconPath: store.getIconPath(p.id),
  }))
})

const criticalCount = computed(() => ({
  granted: store.criticalGranted,
  total: store.criticalPermissions.length,
}))

const isInitialLoading = computed(() => store.loading && store.permissions.length === 0)
const isRefreshing = computed(() => store.loading && store.permissions.length > 0)

// Brief highlight animation when a permission status changes
const flashById = ref<Record<string, boolean>>({})
watch(
  () => store.permissions.map(p => ({ id: p.id, status: p.status })),
  (next, prev) => {
    if (!prev) return
    const prevMap = new Map(prev.map(p => [p.id, p.status]))
    for (const p of next) {
      const before = prevMap.get(p.id)
      if (before && before !== p.status) {
        flashById.value = { ...flashById.value, [p.id]: true }
        const timer = window.setTimeout(() => {
          flashById.value = { ...flashById.value, [p.id]: false }
          flashTimers.delete(timer)
        }, 900)
        flashTimers.add(timer)
      }
    }
  },
  { immediate: true }
)

function requestPermission(id: string) {
  store.requestPermission(id)
}

function openSystemPreferences() {
  store.openSettings('Privacy')
}

// Determine if we should show the status badge or just a Request button
function shouldShowBadge(status: string): boolean {
  return status === 'granted' || status === 'denied'
}

// Map status for StatusBadge component (only used for granted/denied)
function mapStatusForBadge(status: string): 'granted' | 'denied' {
  return status === 'granted' ? 'granted' : 'denied'
}
</script>

<template>
  <div class="animate-fade-in">
    <div class="mb-8">
      <h1 class="font-display text-2xl tracking-wider text-naonur-gold mb-2">
        🔐 Permissions
      </h1>
      <p class="text-naonur-ash font-body">
        Manage macOS permissions required by OpenClaw agents.
      </p>
    </div>

    <!-- Initial Loading State (only when we have nothing to show yet) -->
    <div v-if="isInitialLoading" class="naonur-card mb-6 text-center py-8">
      <p class="text-naonur-ash animate-pulse">Checking permissions...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="store.error" class="naonur-card mb-6 border-red-500/50">
      <p class="text-red-400">{{ store.error }}</p>
      <button class="btn btn-secondary mt-4" @click="store.loadPermissions()">
        Retry
      </button>
    </div>

    <template v-else>
      <!-- Status Summary -->
      <div class="naonur-card mb-6 flex items-center justify-between">
        <div>
          <div class="flex items-center gap-3">
            <p class="text-naonur-ash text-sm">Critical Permissions Status</p>
            <span v-if="isRefreshing" class="text-xs text-naonur-smoke font-mono animate-pulse">
              refreshing…
            </span>
          </div>
          <p class="font-display text-lg text-naonur-bone">
            {{ criticalCount.granted }} of {{ criticalCount.total }} granted
          </p>
        </div>
        <button class="btn btn-secondary" @click="openSystemPreferences">
          Open System Preferences
        </button>
      </div>

      <!-- Permission Cards Grid -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="perm in permissionsWithIcons"
          :key="perm.id"
          :class="[
            'naonur-card perm-card relative grid grid-rows-[auto_1fr_auto] min-h-[180px]',
            perm.critical && 'ring-1 ring-naonur-gold/20',
            isRefreshing && 'opacity-85',
            flashById[perm.id] && 'perm-flash'
          ]"
        >
          <!-- Critical badge -->
          <div 
            v-if="perm.critical" 
            class="absolute top-3 right-3 text-xs text-naonur-gold font-mono"
          >
            CRITICAL
          </div>

          <!-- Top section: Icon + Name -->
          <div class="flex items-start gap-4">
            <div class="w-12 h-12 flex-shrink-0">
              <img 
                v-if="perm.iconPath" 
                :src="perm.iconPath" 
                :alt="perm.name"
                class="w-full h-full object-contain"
              />
              <span v-else class="text-3xl">{{ perm.icon }}</span>
            </div>
            <div class="flex-1 pr-16">
              <h3 class="font-display text-base text-naonur-bone mb-1">
                {{ perm.name }}
              </h3>
            </div>
          </div>
          
          <!-- Middle section: Description (grows to fill space) -->
          <div class="py-2">
            <p class="text-sm text-naonur-smoke line-clamp-2">
              {{ perm.description }}
            </p>
          </div>

          <!-- Bottom section: Status/Button (always at bottom) -->
          <div class="flex items-center justify-between pt-2 border-t border-naonur-fog/50">
            <!-- Show badge only for granted/denied -->
            <template v-if="shouldShowBadge(perm.status)">
              <Transition name="fade" mode="out-in">
                <StatusBadge :key="perm.status" :status="mapStatusForBadge(perm.status)" size="sm" />
              </Transition>
              <button
                v-if="perm.status === 'denied'"
                class="btn btn-ghost text-xs"
                @click="requestPermission(perm.id)"
              >
                Request
              </button>
              <span v-else class="text-xs text-naonur-smoke">✓</span>
            </template>
            
            <!-- For not_determined/restricted/unknown, show Request button -->
            <template v-else>
              <span class="text-xs text-naonur-ash font-mono">Not Set</span>
              <button
                class="btn btn-secondary text-xs"
                @click="requestPermission(perm.id)"
              >
                Request
              </button>
            </template>
          </div>
        </div>
      </div>
    </template>

    <div class="threshold-line my-8"></div>

    <p class="text-center text-naonur-smoke text-sm font-mono">
      Permissions are managed via macOS System Preferences. Click "Request" to open the appropriate settings pane.
    </p>
  </div>
</template>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* Smooth badge/status transitions */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 160ms ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* Subtle card highlight when permission status changes */
.perm-card {
  transition: opacity 200ms ease, box-shadow 240ms ease, transform 240ms ease;
}

.perm-flash {
  box-shadow: 0 0 0 1px rgba(217, 185, 123, 0.25), 0 0 24px rgba(217, 185, 123, 0.10);
  transform: translateY(-1px);
}
</style>
