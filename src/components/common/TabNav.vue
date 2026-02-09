<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import logoUrl from '@/assets/icons/logo.png'

interface NavItem {
  id: string
  label: string
  emoji: string
  route: string
}

const route = useRoute()
const router = useRouter()

const navItems: NavItem[] = [
  { id: 'dashboard', label: 'Dashboard', emoji: '🏠', route: '/' },
  { id: 'permissions', label: 'Permissions', emoji: '🔐', route: '/permissions' },
  { id: 'config', label: 'Config', emoji: '⚙️', route: '/config' },
  { id: 'monitor', label: 'Monitor', emoji: '📊', route: '/monitor' },
  { id: 'profiles', label: 'Profiles', emoji: '👤', route: '/profiles' },
  { id: 'auth', label: 'Auth', emoji: '🔑', route: '/auth' },
]

const activeTab = computed(() => {
  const path = route.path
  const item = navItems.find(item => item.route === path)
  return item?.id ?? 'dashboard'
})

function navigateTo(item: NavItem) {
  router.push(item.route)
}
</script>

<template>
  <nav class="flex flex-col h-full bg-naonur-shadow border-r border-naonur-fog">
    <!-- Logo / Brand -->
    <div class="p-6 border-b border-naonur-fog">
      <div class="flex items-center gap-3">
        <img :src="logoUrl" alt="Tairseach" class="w-10 h-10" />
        <div>
          <h1 class="font-display text-lg tracking-wider text-naonur-gold">
            Tairseach
          </h1>
          <p class="text-xs text-naonur-smoke font-body italic">
            The Threshold
          </p>
        </div>
      </div>
    </div>

    <!-- Navigation Items -->
    <div class="flex-1 py-4 px-3 space-y-1">
      <button
        v-for="item in navItems"
        :key="item.id"
        @click="navigateTo(item)"
        :class="[
          'w-full flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200',
          'text-left font-body text-base',
          activeTab === item.id
            ? 'bg-naonur-mist text-naonur-gold border-l-2 border-naonur-gold -ml-px'
            : 'text-naonur-ash hover:bg-naonur-mist hover:text-naonur-bone'
        ]"
      >
        <span class="text-xl w-6 h-6 flex items-center justify-center">
          {{ item.emoji }}
        </span>
        <span class="tracking-wide">{{ item.label }}</span>
      </button>
    </div>

    <!-- Footer -->
    <div class="p-4 border-t border-naonur-fog">
      <div class="flex items-center gap-2 text-naonur-smoke text-sm">
        <span class="w-2 h-2 rounded-full bg-naonur-moss animate-pulse"></span>
        <span class="font-mono text-xs">OpenClaw Connected</span>
      </div>
    </div>
  </nav>
</template>

<style scoped>
/* Gold accent glow on active tab */
button.bg-naonur-mist {
  box-shadow: inset 3px 0 8px -4px rgba(201, 162, 39, 0.3);
}
</style>
