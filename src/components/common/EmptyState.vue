<script setup lang="ts">
interface Props {
  title?: string
  message: string
  icon?: string
}

withDefaults(defineProps<Props>(), {
  title: '',
  icon: ''
})
</script>

<template>
  <div class="naonur-card state-card text-center py-10">
    <div v-if="icon" class="text-5xl mb-3">{{ icon }}</div>
    <h2 v-if="title" class="font-display text-xl text-naonur-bone mb-2">{{ title }}</h2>
    <p class="text-naonur-ash">{{ message }}</p>
    <slot />
  </div>
</template>
