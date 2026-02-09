<script setup lang="ts">
import { computed, ref } from 'vue'
import { useConfigStore, type ProviderConfig } from '../../stores/config'
import ConfigInput from './ConfigInput.vue'

interface Props {
  name: string
  config: ProviderConfig
}

const props = defineProps<Props>()
defineEmits<{
  remove: [name: string]
}>()

const store = useConfigStore()
const expanded = ref(false)

const modelCount = computed(() => props.config.models?.length || 0)

// Update provider config
function updateBaseUrl(url: string | number) {
  store.updateConfigValue(
    ['models', 'providers', props.name, 'baseUrl'],
    String(url)
  )
}

function updateApiKey(key: string | number) {
  store.updateConfigValue(
    ['models', 'providers', props.name, 'apiKey'],
    String(key)
  )
}
</script>

<template>
  <div class="p-4 rounded-lg border border-naonur-fog/30 bg-naonur-void/30">
    <!-- Header -->
    <div class="flex items-center justify-between mb-3">
      <div class="flex items-center gap-3">
        <div 
          class="w-10 h-10 rounded-lg bg-naonur-fog/20 flex items-center justify-center text-lg"
        >
          🔌
        </div>
        <div>
          <h4 class="font-display text-naonur-bone">{{ name }}</h4>
          <p class="text-xs text-naonur-ash">
            {{ modelCount }} model{{ modelCount !== 1 ? 's' : '' }}
          </p>
        </div>
      </div>
      
      <div class="flex items-center gap-2">
        <button
          class="p-2 text-naonur-ash hover:text-naonur-bone transition-colors"
          @click="expanded = !expanded"
        >
          <span :class="['transition-transform inline-block', expanded && 'rotate-180']">▼</span>
        </button>
        <button
          class="p-2 text-naonur-ash hover:text-naonur-blood transition-colors"
          @click="$emit('remove', name)"
        >
          ✕
        </button>
      </div>
    </div>
    
    <!-- Quick info -->
    <div v-if="config.baseUrl" class="text-xs font-mono text-naonur-ash truncate">
      {{ config.baseUrl }}
    </div>
    
    <!-- Expanded -->
    <Transition
      enter-active-class="transition-all duration-200"
      enter-from-class="opacity-0 max-h-0"
      enter-to-class="opacity-100 max-h-[500px]"
      leave-active-class="transition-all duration-150"
      leave-from-class="opacity-100 max-h-[500px]"
      leave-to-class="opacity-0 max-h-0"
    >
      <div v-if="expanded" class="mt-4 pt-4 border-t border-naonur-fog/30 space-y-4 overflow-hidden">
        <ConfigInput
          :model-value="config.baseUrl || ''"
          label="Base URL"
          placeholder="http://localhost:11434/v1"
          monospace
          @update:model-value="updateBaseUrl"
        />
        
        <ConfigInput
          :model-value="config.apiKey || ''"
          label="API Key"
          type="password"
          placeholder="••••••••"
          monospace
          @update:model-value="updateApiKey"
        />
        
        <!-- Models list -->
        <div v-if="config.models?.length">
          <p class="text-sm text-naonur-bone mb-2">Models</p>
          <div class="space-y-2">
            <div 
              v-for="model in config.models" 
              :key="model.id"
              class="px-3 py-2 rounded bg-naonur-fog/10 text-sm"
            >
              <div class="flex items-center justify-between">
                <span class="font-mono text-naonur-bone">{{ model.id }}</span>
                <span class="text-xs text-naonur-ash">{{ model.contextWindow?.toLocaleString() }} ctx</span>
              </div>
              <p class="text-xs text-naonur-ash">{{ model.name }}</p>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>
