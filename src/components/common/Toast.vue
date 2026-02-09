<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'

export interface ToastProps {
  id: string
  message: string
  variant?: 'success' | 'warning' | 'error' | 'info'
  duration?: number
  dismissible?: boolean
}

const props = withDefaults(defineProps<ToastProps>(), {
  variant: 'info',
  duration: 5000,
  dismissible: true,
})

const emit = defineEmits<{
  dismiss: [id: string]
}>()

const isVisible = ref(false)
const isLeaving = ref(false)
let dismissTimer: ReturnType<typeof setTimeout> | null = null

const variantConfig = computed(() => {
  const configs = {
    success: {
      icon: '✓',
      bgClass: 'bg-naonur-moss',
      borderClass: 'border-naonur-moss',
      iconBg: 'bg-naonur-moss-dim',
    },
    warning: {
      icon: '⚠',
      bgClass: 'bg-naonur-rust',
      borderClass: 'border-naonur-rust',
      iconBg: 'bg-naonur-rust-dim',
    },
    error: {
      icon: '✕',
      bgClass: 'bg-naonur-blood',
      borderClass: 'border-naonur-blood',
      iconBg: 'bg-naonur-blood-dim',
    },
    info: {
      icon: 'ℹ',
      bgClass: 'bg-naonur-water',
      borderClass: 'border-naonur-water',
      iconBg: 'bg-naonur-water-dim',
    },
  }
  return configs[props.variant]
})

function dismiss() {
  isLeaving.value = true
  setTimeout(() => {
    emit('dismiss', props.id)
  }, 200)
}

function startDismissTimer() {
  if (props.duration > 0) {
    dismissTimer = setTimeout(dismiss, props.duration)
  }
}

function pauseDismissTimer() {
  if (dismissTimer) {
    clearTimeout(dismissTimer)
    dismissTimer = null
  }
}

onMounted(() => {
  // Trigger enter animation
  requestAnimationFrame(() => {
    isVisible.value = true
  })
  startDismissTimer()
})

onUnmounted(() => {
  pauseDismissTimer()
})
</script>

<template>
  <div
    :class="[
      'flex items-center gap-3 p-4 rounded-lg shadow-lg',
      'bg-naonur-shadow border',
      variantConfig.borderClass,
      'transform transition-all duration-200',
      isVisible && !isLeaving ? 'translate-x-0 opacity-100' : 'translate-x-4 opacity-0',
    ]"
    @mouseenter="pauseDismissTimer"
    @mouseleave="startDismissTimer"
    role="alert"
  >
    <!-- Icon -->
    <div
      :class="[
        'flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center',
        variantConfig.iconBg,
        'text-naonur-bone text-sm font-bold'
      ]"
    >
      {{ variantConfig.icon }}
    </div>

    <!-- Message -->
    <p class="flex-1 text-naonur-bone font-body text-sm">
      {{ message }}
    </p>

    <!-- Dismiss button -->
    <button
      v-if="dismissible"
      @click="dismiss"
      class="flex-shrink-0 w-6 h-6 rounded flex items-center justify-center
             text-naonur-ash hover:text-naonur-bone hover:bg-naonur-fog
             transition-colors duration-150"
      aria-label="Dismiss"
    >
      ✕
    </button>
  </div>
</template>
