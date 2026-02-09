<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import StatusBadge from '../components/common/StatusBadge.vue'
import { usePermissionsStore } from '../stores/permissions'

const permissionsStore = usePermissionsStore()

// Activity log - will be populated from permission checks
interface ActivityItem {
  id: number
  agent: string
  action: string
  time: string
  status: 'granted' | 'denied' | 'pending'
  timestamp: Date
}

const recentActivity = ref<ActivityItem[]>([])
let activityIdCounter = 0

// Load permissions on mount
onMounted(async () => {
  await permissionsStore.loadDefinitions()
  await permissionsStore.loadPermissions()
  
  // Generate activity from initial permission check
  generateInitialActivity()
})

// Watch for permission changes and log activity
watch(() => permissionsStore.permissions, (newPerms, oldPerms) => {
  if (oldPerms.length > 0) {
    // Find changed permissions
    for (const newPerm of newPerms) {
      const oldPerm = oldPerms.find(p => p.id === newPerm.id)
      if (oldPerm && oldPerm.status !== newPerm.status) {
        addActivity({
          agent: 'tairseach',
          action: `Permission changed: ${newPerm.name}`,
          status: newPerm.status === 'granted' ? 'granted' : 'denied',
        })
      }
    }
  }
}, { deep: true })

// Computed stats from real data
const stats = computed(() => ({
  permissionsGranted: permissionsStore.criticalGranted,
  permissionsTotal: permissionsStore.criticalPermissions.length,
  totalGranted: permissionsStore.permissions.filter(p => p.status === 'granted').length,
  totalPermissions: permissionsStore.permissions.length,
  activeSessions: 0, // TODO: Connect to monitor store
  tokensToday: '—', // TODO: Connect to monitor store
  connectedServices: 0, // TODO: Connect to auth store
}))

// Generate initial activity from permission states
function generateInitialActivity() {
  // Add a startup event
  addActivity({
    agent: 'tairseach',
    action: 'Permission scan completed',
    status: 'pending',
  })
  
  // Add events for each permission status
  for (const perm of permissionsStore.permissions) {
    const status = perm.status === 'granted' ? 'granted' : 
                   perm.status === 'denied' ? 'denied' : 'pending'
    
    addActivity({
      agent: 'system',
      action: `${perm.name}: ${perm.status}`,
      status,
    })
  }
}

function addActivity(item: { agent: string; action: string; status: 'granted' | 'denied' | 'pending' }) {
  const now = new Date()
  recentActivity.value.unshift({
    id: ++activityIdCounter,
    ...item,
    time: 'just now',
    timestamp: now,
  })
  
  // Keep only last 20 items
  if (recentActivity.value.length > 20) {
    recentActivity.value = recentActivity.value.slice(0, 20)
  }
  
  // Update relative times
  updateRelativeTimes()
}

function updateRelativeTimes() {
  const now = new Date()
  for (const activity of recentActivity.value) {
    const diffMs = now.getTime() - activity.timestamp.getTime()
    const diffSec = Math.floor(diffMs / 1000)
    const diffMin = Math.floor(diffSec / 60)
    const diffHour = Math.floor(diffMin / 60)
    
    if (diffSec < 60) {
      activity.time = 'just now'
    } else if (diffMin < 60) {
      activity.time = `${diffMin} min ago`
    } else if (diffHour < 24) {
      activity.time = `${diffHour} hour${diffHour > 1 ? 's' : ''} ago`
    } else {
      activity.time = activity.timestamp.toLocaleDateString()
    }
  }
}

// Update times every minute
setInterval(updateRelativeTimes, 60000)

// Map activity status to icon path
function getActivityIcon(status: string): string {
  const iconMap: Record<string, string> = {
    'granted': new URL('@/assets/icons/activity-granted.png', import.meta.url).href,
    'denied': new URL('@/assets/icons/activity-denied.png', import.meta.url).href,
    'pending': new URL('@/assets/icons/activity-event.png', import.meta.url).href,
  }
  return iconMap[status] || iconMap['pending']
}
</script>

<template>
  <div class="animate-fade-in">
    <!-- Header -->
    <div class="mb-8">
      <div class="flex items-center gap-4 mb-2">
        <h1 class="font-display text-3xl tracking-wider text-naonur-gold">
          Tairseach
        </h1>
        <span class="text-naonur-smoke">—</span>
        <span class="font-body italic text-naonur-ash text-xl">The Threshold</span>
      </div>
      <p class="text-naonur-ash font-body">
        Welcome to the bridge between worlds. Monitor permissions, manage configurations, and oversee your agents.
      </p>
    </div>

    <!-- Celtic divider -->
    <div class="threshold-line mb-8"></div>

    <!-- Status Cards Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
      <!-- Permissions Card -->
      <router-link to="/permissions" class="naonur-card group hover:ring-1 hover:ring-naonur-gold/30 transition-all">
        <div class="flex items-start justify-between mb-4">
          <div class="text-3xl">🔐</div>
          <StatusBadge 
            :status="stats.permissionsGranted === stats.permissionsTotal ? 'granted' : 'pending'" 
            size="sm"
          />
        </div>
        <h3 class="font-display text-sm tracking-wide text-naonur-ash mb-1">
          Critical Permissions
        </h3>
        <p class="text-2xl font-display text-naonur-bone">
          {{ stats.permissionsGranted }} 
          <span class="text-naonur-smoke text-lg">/ {{ stats.permissionsTotal }}</span>
        </p>
        <p class="text-sm text-naonur-smoke mt-2">
          {{ stats.totalGranted }} of {{ stats.totalPermissions }} total granted
        </p>
      </router-link>

      <!-- Sessions Card -->
      <div class="naonur-card opacity-60">
        <div class="flex items-start justify-between mb-4">
          <div class="text-3xl opacity-50">📊</div>
          <span class="w-2 h-2 rounded-full bg-naonur-smoke"></span>
        </div>
        <h3 class="font-display text-sm tracking-wide text-naonur-ash mb-1">
          Active Sessions
        </h3>
        <p class="text-2xl font-display text-naonur-bone">
          {{ stats.activeSessions }}
        </p>
        <p class="text-sm text-naonur-smoke mt-2">coming soon</p>
      </div>

      <!-- Token Usage Card -->
      <div class="naonur-card opacity-60">
        <div class="flex items-start justify-between mb-4">
          <div class="text-3xl text-naonur-gold/50">✦</div>
        </div>
        <h3 class="font-display text-sm tracking-wide text-naonur-ash mb-1">
          Tokens Today
        </h3>
        <p class="text-2xl font-display text-naonur-bone">
          {{ stats.tokensToday }}
        </p>
        <p class="text-sm text-naonur-smoke mt-2">coming soon</p>
      </div>

      <!-- Connected Services Card -->
      <div class="naonur-card opacity-60">
        <div class="flex items-start justify-between mb-4">
          <div class="text-3xl opacity-50">🔑</div>
          <StatusBadge status="unknown" size="sm" />
        </div>
        <h3 class="font-display text-sm tracking-wide text-naonur-ash mb-1">
          Auth Services
        </h3>
        <p class="text-2xl font-display text-naonur-bone">
          {{ stats.connectedServices }}
        </p>
        <p class="text-sm text-naonur-smoke mt-2">coming soon</p>
      </div>
    </div>

    <!-- Recent Activity -->
    <div class="naonur-card">
      <h2 class="font-display text-lg tracking-wide text-naonur-bone mb-4 flex items-center gap-3">
        <span class="text-naonur-gold">☽</span>
        Recent Activity
      </h2>

      <!-- Loading State -->
      <div v-if="permissionsStore.loading" class="text-center py-8">
        <p class="text-naonur-ash animate-pulse">Loading permissions...</p>
      </div>

      <!-- Empty State -->
      <div v-else-if="recentActivity.length === 0" class="text-center py-8">
        <p class="text-naonur-smoke">No recent activity</p>
      </div>

      <!-- Activity List -->
      <div v-else class="space-y-3">
        <div
          v-for="activity in recentActivity.slice(0, 10)"
          :key="activity.id"
          class="flex items-center gap-4 p-3 rounded-lg bg-naonur-mist/50 hover:bg-naonur-mist transition-colors"
        >
          <img 
            :src="getActivityIcon(activity.status)" 
            :alt="activity.status"
            class="w-8 h-8 object-contain"
          />
          <div class="flex-1 min-w-0">
            <p class="text-naonur-bone font-body truncate">
              {{ activity.action }}
            </p>
            <p class="text-sm text-naonur-smoke font-mono">
              {{ activity.agent }} · {{ activity.time }}
            </p>
          </div>
          <StatusBadge :status="activity.status" size="sm" />
        </div>
      </div>

      <div v-if="recentActivity.length > 10" class="mt-4 pt-4 border-t border-naonur-fog">
        <button class="btn btn-ghost text-sm">
          View All Activity ({{ recentActivity.length }}) →
        </button>
      </div>
    </div>

    <!-- Bottom branding -->
    <div class="mt-12 text-center">
      <div class="threshold-line mb-6"></div>
      <p class="font-display text-sm tracking-[0.3em] text-naonur-smoke uppercase">
        Naonúr Ecosystem
      </p>
      <p class="text-xs text-naonur-smoke/50 mt-2 font-mono">
        The threshold between the digital realm and the system beneath
      </p>
    </div>
  </div>
</template>

<!-- Styles moved to global CSS -->
