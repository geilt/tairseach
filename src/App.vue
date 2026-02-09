<script setup lang="ts">
import { ref } from 'vue'
import TabNav from './components/common/TabNav.vue'
import ToastContainer from './components/common/ToastContainer.vue'

const toastContainer = ref<InstanceType<typeof ToastContainer> | null>(null)

// Make toast accessible globally via provide/inject
// (For now, components can use toastContainer.value?.success() etc.)
</script>

<template>
  <div class="flex h-screen bg-naonur-void text-naonur-bone overflow-hidden">
    <!-- Sidebar Navigation -->
    <aside class="w-64 flex-shrink-0">
      <TabNav />
    </aside>

    <!-- Main Content Area -->
    <main class="flex-1 overflow-auto bg-feather">
      <!-- Top bar / breadcrumb area -->
      <header class="h-14 px-6 flex items-center border-b border-naonur-fog/50 titlebar">
        <div class="flex-1">
          <!-- Placeholder for breadcrumbs or title -->
        </div>
        <div class="flex items-center gap-4">
          <!-- Status indicator -->
          <div class="flex items-center gap-2 text-sm text-naonur-ash">
            <span class="w-2 h-2 rounded-full bg-naonur-moss"></span>
            <span class="font-mono text-xs">v0.1.0</span>
          </div>
        </div>
      </header>

      <!-- Router View -->
      <div class="p-6">
        <RouterView v-slot="{ Component }">
          <Transition name="page" mode="out-in">
            <component :is="Component" />
          </Transition>
        </RouterView>
      </div>
    </main>

    <!-- Toast Notifications -->
    <ToastContainer ref="toastContainer" />
  </div>
</template>

<style>
/* Page transition */
.page-enter-active,
.page-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.page-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
