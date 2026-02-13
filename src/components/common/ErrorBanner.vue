<script setup lang="ts">
withDefaults(
  defineProps<{
    message: string
    retryLabel?: string
  }>(),
  {
    retryLabel: 'Retry'
  }
)

defineEmits<{ retry: [] }>()
</script>

<template>
  <div class="naonur-card state-card border-red-500/50">
    <p class="text-red-400">{{ message }}</p>
    <button class="btn btn-secondary mt-4" @click="$emit('retry')">{{ retryLabel }}</button>
  </div>
</template>
