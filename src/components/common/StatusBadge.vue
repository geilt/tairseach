<script setup lang="ts">
import { computed } from 'vue'

export type StatusVariant = 'granted' | 'denied' | 'pending' | 'unknown'

interface Props {
  status: StatusVariant
  size?: 'sm' | 'md'
  showIcon?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  size: 'md',
  showIcon: true,
})

const config = computed(() => {
  const configs = {
    granted: {
      label: 'Granted',
      icon: '●',
      bgClass: 'bg-naonur-moss/20',
      textClass: 'text-naonur-moss',
      borderClass: 'border-naonur-moss/30',
    },
    denied: {
      label: 'Denied',
      icon: '●',
      bgClass: 'bg-naonur-blood/20',
      textClass: 'text-naonur-blood',
      borderClass: 'border-naonur-blood/30',
    },
    pending: {
      label: 'Pending',
      icon: '○',
      bgClass: 'bg-naonur-rust/20',
      textClass: 'text-naonur-rust',
      borderClass: 'border-naonur-rust/30',
    },
    unknown: {
      label: 'Unknown',
      icon: '◌',
      bgClass: 'bg-naonur-fog/50',
      textClass: 'text-naonur-ash',
      borderClass: 'border-naonur-fog',
    },
  }
  return configs[props.status]
})

const sizeClasses = computed(() => {
  return props.size === 'sm' 
    ? 'px-2 py-0.5 text-xs'
    : 'px-3 py-1 text-sm'
})
</script>

<template>
  <span
    :class="[
      'inline-flex items-center gap-1.5 rounded-full border font-mono',
      config.bgClass,
      config.textClass,
      config.borderClass,
      sizeClasses,
    ]"
  >
    <span v-if="showIcon" class="text-xs">{{ config.icon }}</span>
    <span>{{ config.label }}</span>
  </span>
</template>
