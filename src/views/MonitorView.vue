<script setup lang="ts">
import { onActivated, onMounted } from 'vue'
import { useMonitorStore } from '@/stores/monitor'

const store = useMonitorStore()

onMounted(() => {
  void store.init()
})

onActivated(() => {
  void store.connect({ silent: true })
})
</script>

<template>
  <div class="animate-fade-in">
    <div class="mb-8">
      <h1 class="font-display text-2xl tracking-wider text-naonur-gold mb-2 flex items-center gap-3">
        <img src="@/assets/icons/monitor-header.png" alt="Monitor" class="w-8 h-8 object-contain" />
        Monitor
      </h1>
      <p class="text-naonur-ash font-body">
        Real-time token usage and session monitoring.
      </p>
    </div>

    <div class="naonur-card text-center py-12">
      <img 
        src="@/assets/icons/monitor-header.png" 
        alt="Coming Soon"
        class="w-24 h-24 mx-auto mb-4 object-contain"
      />
      <h2 class="font-display text-xl text-naonur-bone mb-2">Coming Soon</h2>
      <p class="text-naonur-ash">
        Live context usage tracking and cost estimates
      </p>
    </div>
  </div>
</template>
