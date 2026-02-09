<script setup lang="ts">
import { ref } from 'vue'

interface Props {
  title: string
  icon?: string
  description?: string
  defaultOpen?: boolean
  badge?: string | number
}

const props = withDefaults(defineProps<Props>(), {
  defaultOpen: true,
})

const isOpen = ref(props.defaultOpen)

function toggle() {
  isOpen.value = !isOpen.value
}
</script>

<template>
  <div class="naonur-card">
    <button 
      type="button"
      class="w-full flex items-center justify-between text-left"
      @click="toggle"
    >
      <div class="flex items-center gap-3">
        <span v-if="icon" class="text-xl">{{ icon }}</span>
        <div>
          <h3 class="font-display text-lg text-naonur-bone flex items-center gap-2">
            {{ title }}
            <span 
              v-if="badge !== undefined" 
              class="px-2 py-0.5 text-xs font-mono bg-naonur-fog/30 rounded-full text-naonur-ash"
            >
              {{ badge }}
            </span>
          </h3>
          <p v-if="description" class="text-sm text-naonur-ash">{{ description }}</p>
        </div>
      </div>
      <span 
        :class="[
          'text-naonur-ash transition-transform duration-200',
          isOpen && 'rotate-180'
        ]"
      >
        ▼
      </span>
    </button>
    
    <Transition
      enter-active-class="transition-all duration-200 ease-out"
      enter-from-class="opacity-0 max-h-0"
      enter-to-class="opacity-100 max-h-[2000px]"
      leave-active-class="transition-all duration-150 ease-in"
      leave-from-class="opacity-100 max-h-[2000px]"
      leave-to-class="opacity-0 max-h-0"
    >
      <div v-show="isOpen" class="mt-4 pt-4 border-t border-naonur-fog/30 overflow-hidden">
        <slot />
      </div>
    </Transition>
  </div>
</template>
