<script setup lang="ts">
import { onActivated, onMounted } from 'vue'
import { useProfilesStore } from '@/stores/profiles'
import SectionHeader from '@/components/common/SectionHeader.vue'
import EmptyState from '@/components/common/EmptyState.vue'

const store = useProfilesStore()

onMounted(() => {
  void store.init()
})

onActivated(() => {
  void store.loadProfiles({ silent: true })
})
</script>

<template>
  <div class="animate-fade-in">
    <SectionHeader title="Agent Profiles" description="Manage visual identities for your agents.">
      <template #icon>
        <img src="@/assets/icons/profile-inner.png" alt="Profiles" class="w-8 h-8 object-contain" />
      </template>
    </SectionHeader>

    <EmptyState title="Coming Soon" message="Avatar uploads, colors, and display names">
      <img src="@/assets/icons/profile-inner.png" alt="Coming Soon" class="w-24 h-24 mx-auto mb-4 object-contain" />
    </EmptyState>
  </div>
</template>
