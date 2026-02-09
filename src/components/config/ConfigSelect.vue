<script setup lang="ts">
import { computed } from 'vue'

export interface SelectOption {
  value: string
  label: string
  description?: string
}

interface Props {
  modelValue: string
  options: SelectOption[]
  label?: string
  description?: string
  placeholder?: string
  disabled?: boolean
  allowCustom?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  disabled: false,
  allowCustom: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const selected = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
})

// Check if current value is in options
const isCustomValue = computed(() => {
  return props.modelValue && !props.options.some(o => o.value === props.modelValue)
})
</script>

<template>
  <div class="space-y-1.5">
    <label v-if="label" class="block text-sm text-naonur-bone font-medium">
      {{ label }}
    </label>
    <select
      v-model="selected"
      :disabled="disabled"
      :class="[
        'w-full px-3 py-2 text-sm rounded-md border border-naonur-fog/50 bg-naonur-void/50 text-naonur-bone',
        'focus:outline-none focus:ring-1 focus:ring-naonur-gold/50 focus:border-naonur-gold/50',
        'disabled:opacity-50 disabled:cursor-not-allowed',
        'appearance-none bg-no-repeat bg-right pr-8',
      ]"
      style="background-image: url('data:image/svg+xml;charset=utf-8,%3Csvg xmlns=%27http://www.w3.org/2000/svg%27 fill=%27none%27 viewBox=%270 0 20 20%27%3E%3Cpath stroke=%27%239ca3af%27 stroke-linecap=%27round%27 stroke-linejoin=%27round%27 stroke-width=%271.5%27 d=%27m6 8 4 4 4-4%27/%3E%3C/svg%3E'); background-size: 1.5rem;"
    >
      <option v-if="placeholder" value="" disabled>{{ placeholder }}</option>
      <option 
        v-for="opt in options" 
        :key="opt.value" 
        :value="opt.value"
      >
        {{ opt.label }}
      </option>
      <option v-if="allowCustom && isCustomValue" :value="modelValue">
        {{ modelValue }} (custom)
      </option>
    </select>
    <p v-if="description" class="text-xs text-naonur-ash">{{ description }}</p>
  </div>
</template>
