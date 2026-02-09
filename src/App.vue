<script setup lang="ts">
import { ref } from 'vue'
import TabNav from './components/common/TabNav.vue'
import ToastContainer from './components/common/ToastContainer.vue'

const toastContainer = ref<InstanceType<typeof ToastContainer> | null>(null)
</script>

<template>
  <div class="flex h-screen bg-naonur-void text-naonur-bone overflow-hidden">
    <aside class="w-64 flex-shrink-0">
      <TabNav />
    </aside>

    <main class="flex-1 overflow-auto bg-feather">
      <header class="h-14 px-6 flex items-center border-b border-naonur-fog/50 titlebar">
        <div class="flex-1" />
        <div class="flex items-center gap-4">
          <div class="flex items-center gap-2 text-sm text-naonur-ash">
            <span class="w-2 h-2 rounded-full bg-naonur-moss"></span>
            <span class="font-mono text-xs">v0.1.0</span>
          </div>
        </div>
      </header>

      <div class="p-6">
        <RouterView v-slot="{ Component, route }">
          <Transition name="page" mode="default">
            <KeepAlive :max="5">
              <component :is="Component" :key="route.name" />
            </KeepAlive>
          </Transition>
        </RouterView>
      </div>
    </main>

    <ToastContainer ref="toastContainer" />
  </div>
</template>

<style>
.page-enter-active,
.page-leave-active {
  transition: opacity 0.08s linear;
}

.page-enter-from,
.page-leave-to {
  opacity: 0;
}
</style>
