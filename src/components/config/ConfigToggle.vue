<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  modelValue: boolean
  label?: string
  description?: string
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  disabled: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const isOn = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
})
</script>

<template>
  <div class="flex items-center justify-between gap-4">
    <div class="flex-1">
      <span v-if="label" class="text-sm text-naonur-bone font-medium">{{ label }}</span>
      <p v-if="description" class="text-xs text-naonur-ash mt-0.5">{{ description }}</p>
    </div>
    <button
      type="button"
      role="switch"
      :aria-checked="isOn"
      :disabled="disabled"
      :class="[
        'relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-naonur-gold/50',
        isOn ? 'bg-naonur-moss' : 'bg-naonur-fog',
        disabled && 'opacity-50 cursor-not-allowed',
      ]"
      @click="isOn = !isOn"
    >
      <span
        :class="[
          'inline-block h-4 w-4 transform rounded-full bg-white transition-transform shadow',
          isOn ? 'translate-x-6' : 'translate-x-1',
        ]"
      />
    </button>
  </div>
</template>
