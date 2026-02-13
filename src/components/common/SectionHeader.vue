<script setup lang="ts">
interface Props {
  title: string
  description?: string
  icon?: string
}

defineProps<Props>()
</script>

<template>
  <div class="view-header">
    <div>
      <h1 class="view-title">
        <span v-if="icon" aria-hidden="true">{{ icon }}</span>
        <slot name="icon" />
        <span>{{ title }}</span>
      </h1>
      <p v-if="description" class="view-description">{{ description }}</p>
      <slot name="meta" />
    </div>
    <div class="view-header-actions">
      <slot name="actions" />
    </div>
  </div>
</template>
