<script setup lang="ts">
import { onActivated, onMounted } from 'vue'
import { useMonitorStore } from '@/stores/monitor'
import SectionHeader from '@/components/common/SectionHeader.vue'
import EmptyState from '@/components/common/EmptyState.vue'

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
    <SectionHeader title="Monitor" description="Real-time token usage and session monitoring.">
      <template #icon>
        <img src="@/assets/icons/monitor-header.png" alt="Monitor" class="w-8 h-8 object-contain" />
      </template>
    </SectionHeader>

    <EmptyState title="Coming Soon" message="Live context usage tracking and cost estimates">
      <img src="@/assets/icons/monitor-header.png" alt="Coming Soon" class="w-24 h-24 mx-auto mb-4 object-contain" />
    </EmptyState>
  </div>
</template>
