<script setup lang="ts">
withDefaults(
  defineProps<{
    message?: string
  }>(),
  {
    message: 'Loading...'
  }
)
</script>

<template>
  <div class="naonur-card state-card text-center py-8">
    <p class="text-naonur-ash animate-pulse">{{ message }}</p>
  </div>
</template>
