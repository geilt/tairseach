<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { useActivityFeed, type ActivityEntry } from '@/composables/useActivityFeed'
import SectionHeader from '@/components/common/SectionHeader.vue'
import LoadingState from '@/components/common/LoadingState.vue'
import EmptyState from '@/components/common/EmptyState.vue'

const { entries, namespaces, loading, lastUpdated } = useActivityFeed(4000)

const namespaceInput = ref('all')
const namespaceFilter = ref('all')
let debounceTimer: number | null = null

watch(namespaceInput, (value) => {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = window.setTimeout(() => {
    namespaceFilter.value = value
  }, 150)
})

const filteredEntries = computed(() => {
  if (namespaceFilter.value === 'all') return entries.value
  return entries.value.filter((e) => e.namespace === namespaceFilter.value)
})

const container = ref<HTMLElement | null>(null)
const isHovering = ref(false)
const rowHeight = 46
const overscan = 8
const scrollTop = ref(0)

function onScroll() {
  if (!container.value) return
  scrollTop.value = container.value.scrollTop
}

const visibleCount = computed(() => {
  const h = container.value?.clientHeight ?? 520
  return Math.ceil(h / rowHeight) + overscan
})

const startIndex = computed(() => Math.max(0, Math.floor(scrollTop.value / rowHeight) - overscan))
const endIndex = computed(() => Math.min(filteredEntries.value.length, startIndex.value + visibleCount.value))
const visibleItems = computed(() => filteredEntries.value.slice(startIndex.value, endIndex.value))
const topSpacer = computed(() => startIndex.value * rowHeight)
const totalHeight = computed(() => filteredEntries.value.length * rowHeight)

watch(
  () => [filteredEntries.value.length, lastUpdated.value],
  () => {
    if (!container.value || isHovering.value) return
    const el = container.value
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 120
    if (nearBottom || el.scrollTop === 0) {
      requestAnimationFrame(() => {
        if (container.value) container.value.scrollTop = container.value.scrollHeight
      })
    }
  },
)


onUnmounted(() => {
  if (debounceTimer) {
    clearTimeout(debounceTimer)
    debounceTimer = null
  }
})

function rowClass(item: ActivityEntry) {
  switch (item.operationType) {
    case 'write':
      return 'border-l-2 border-blue-400/60'
    case 'destructive':
      return 'border-l-2 border-amber-400/70'
    case 'error':
      return 'border-l-2 border-red-400/70'
    default:
      return 'border-l-2 border-naonur-fog'
  }
}

function fmtTime(ts: string) {
  if (!ts) return '—'
  const d = new Date(ts)
  if (Number.isNaN(d.getTime())) return ts
  return d.toLocaleTimeString()
}
</script>

<template>
  <section class="animate-fade-in">
    <SectionHeader
      title="Activity Feed"
      description="Real-time operations from ~/.tairseach/logs/proxy.log"
      class="mb-6"
    >
      <template #actions>
        <select
          v-model="namespaceInput"
          class="rounded-md border border-naonur-fog bg-naonur-shadow px-3 py-2 text-sm text-naonur-bone"
        >
          <option value="all">All namespaces</option>
          <option v-for="ns in namespaces" :key="ns" :value="ns">{{ ns }}</option>
        </select>
      </template>
    </SectionHeader>

    <div class="naonur-card activity-card p-0">
      <div class="grid grid-cols-12 gap-2 border-b border-naonur-fog/70 px-4 py-3 text-xs uppercase tracking-wide text-naonur-smoke">
        <div class="col-span-2">Time</div>
        <div class="col-span-2">Client</div>
        <div class="col-span-2">Namespace</div>
        <div class="col-span-4">Tool</div>
        <div class="col-span-2">Status</div>
      </div>

      <div
        ref="container"
        class="h-[520px] overflow-auto"
        @scroll="onScroll"
        @mouseenter="isHovering = true"
        @mouseleave="isHovering = false"
      >
        <div v-if="loading && filteredEntries.length === 0" class="p-3">
          <LoadingState message="Loading activity..." />
        </div>

        <div v-else-if="filteredEntries.length === 0" class="p-3">
          <EmptyState message="No matching activity." />
        </div>

        <div v-else :style="{ height: `${totalHeight}px`, position: 'relative' }">
          <div :style="{ transform: `translateY(${topSpacer}px)` }">
            <div
              v-for="item in visibleItems"
              :key="item.id"
              class="grid h-[46px] grid-cols-12 items-center gap-2 px-4 text-sm hover:bg-naonur-mist/40"
              :class="rowClass(item)"
            >
              <div class="col-span-2 font-mono text-xs text-naonur-ash">{{ fmtTime(item.timestamp) }}</div>
              <div class="col-span-2 truncate text-naonur-bone">{{ item.client }}</div>
              <div class="col-span-2 truncate text-naonur-ash">{{ item.namespace }}</div>
              <div class="col-span-4 truncate text-naonur-bone">{{ item.tool }}</div>
              <div class="col-span-2">
                <span
                  class="rounded px-2 py-1 text-xs"
                  :class="item.status === 'error' ? 'bg-red-500/20 text-red-300' : item.status === 'success' ? 'bg-naonur-moss/20 text-naonur-moss' : 'bg-naonur-fog/30 text-naonur-smoke'"
                >
                  {{ item.status }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.activity-card {
  contain: layout style paint;
}
</style>
