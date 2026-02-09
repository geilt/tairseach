<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  modelValue: string | number
  label?: string
  description?: string
  placeholder?: string
  type?: 'text' | 'number' | 'password'
  disabled?: boolean
  monospace?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  type: 'text',
  disabled: false,
  monospace: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
}>()

const inputValue = computed({
  get: () => props.modelValue,
  set: (value) => {
    if (props.type === 'number') {
      emit('update:modelValue', Number(value))
    } else {
      emit('update:modelValue', value)
    }
  },
})
</script>

<template>
  <div class="space-y-1.5">
    <label v-if="label" class="block text-sm text-naonur-bone font-medium">
      {{ label }}
    </label>
    <input
      v-model="inputValue"
      :type="type"
      :placeholder="placeholder"
      :disabled="disabled"
      :class="[
        'w-full px-3 py-2 text-sm rounded-md border border-naonur-fog/50 bg-naonur-void/50 text-naonur-bone',
        'placeholder:text-naonur-ash/50',
        'focus:outline-none focus:ring-1 focus:ring-naonur-gold/50 focus:border-naonur-gold/50',
        'disabled:opacity-50 disabled:cursor-not-allowed',
        monospace && 'font-mono text-xs',
      ]"
    />
    <p v-if="description" class="text-xs text-naonur-ash">{{ description }}</p>
  </div>
</template>
