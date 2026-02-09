<script setup lang="ts">
import { ref } from 'vue'
import Toast, { type ToastProps } from './Toast.vue'

// Toast store - can be made global via provide/inject or Pinia
const toasts = ref<ToastProps[]>([])

let toastIdCounter = 0

function addToast(
  message: string,
  variant: ToastProps['variant'] = 'info',
  duration = 5000
) {
  const id = `toast-${++toastIdCounter}`
  toasts.value.push({ id, message, variant, duration })
  
  // Limit visible toasts
  if (toasts.value.length > 5) {
    toasts.value.shift()
  }
}

function removeToast(id: string) {
  const index = toasts.value.findIndex(t => t.id === id)
  if (index !== -1) {
    toasts.value.splice(index, 1)
  }
}

// Expose methods for external use
defineExpose({
  success: (message: string, duration?: number) => addToast(message, 'success', duration),
  warning: (message: string, duration?: number) => addToast(message, 'warning', duration),
  error: (message: string, duration?: number) => addToast(message, 'error', duration),
  info: (message: string, duration?: number) => addToast(message, 'info', duration),
})
</script>

<template>
  <Teleport to="body">
    <div
      class="fixed top-4 right-4 z-50 flex flex-col gap-3 w-96 max-w-[calc(100vw-2rem)]"
      aria-live="polite"
    >
      <TransitionGroup name="toast">
        <Toast
          v-for="toast in toasts"
          :key="toast.id"
          v-bind="toast"
          @dismiss="removeToast"
        />
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.2s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(20px);
}

.toast-move {
  transition: transform 0.2s ease;
}
</style>
