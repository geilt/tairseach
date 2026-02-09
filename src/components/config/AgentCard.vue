<script setup lang="ts">
import { computed, ref } from 'vue'
import { useConfigStore, type AgentConfig, type ModelOption } from '../../stores/config'
import ConfigSelect from './ConfigSelect.vue'
import ConfigInput from './ConfigInput.vue'
import ConfigToggle from './ConfigToggle.vue'

interface Props {
  agent: AgentConfig
}

const props = defineProps<Props>()
defineEmits<{
  remove: [id: string]
}>()

const store = useConfigStore()
const expanded = ref(false)

// Parse the agent's current model
const modelInfo = computed(() => {
  const modelStr = props.agent.model?.primary || ''
  return store.parseModelString(modelStr)
})

// Provider options
const providerOptions = computed(() => {
  return store.allProviders.map(p => ({
    value: p,
    label: p.charAt(0).toUpperCase() + p.slice(1).replace(/-/g, ' '),
  }))
})

// Model options for current provider
const modelOptions = computed(() => {
  const provider = modelInfo.value.provider
  if (!provider) return []
  return store.getModelsForProvider(provider).map((m: ModelOption) => ({
    value: m.id,
    label: m.name,
    description: m.description,
  }))
})

// Update provider
function updateProvider(provider: string) {
  const models = store.getModelsForProvider(provider)
  const firstModel = models[0]?.id || ''
  const newModelStr = store.buildModelString(provider, firstModel)
  store.updateAgent(props.agent.id, {
    model: { ...props.agent.model, primary: newModelStr }
  })
}

// Update model
function updateModel(model: string) {
  const newModelStr = store.buildModelString(modelInfo.value.provider, model)
  store.updateAgent(props.agent.id, {
    model: { ...props.agent.model, primary: newModelStr }
  })
}

// Update workspace
function updateWorkspace(workspace: string | number) {
  store.updateAgent(props.agent.id, { workspace: String(workspace) })
}

// Update default status
function updateDefault(isDefault: boolean) {
  store.updateAgent(props.agent.id, { default: isDefault })
}
</script>

<template>
  <div 
    :class="[
      'p-4 rounded-lg border transition-colors',
      agent.default 
        ? 'border-naonur-gold/50 bg-naonur-gold/5' 
        : 'border-naonur-fog/30 bg-naonur-void/30',
    ]"
  >
    <!-- Header row -->
    <div class="flex items-center justify-between mb-3">
      <div class="flex items-center gap-3">
        <div 
          class="w-10 h-10 rounded-lg bg-naonur-fog/20 flex items-center justify-center text-lg"
        >
          🤖
        </div>
        <div>
          <h4 class="font-display text-naonur-bone flex items-center gap-2">
            {{ agent.id }}
            <span 
              v-if="agent.default" 
              class="px-2 py-0.5 text-xs bg-naonur-gold/20 text-naonur-gold rounded-full"
            >
              default
            </span>
          </h4>
          <p class="text-xs text-naonur-ash font-mono truncate max-w-[200px]">
            {{ agent.model?.primary || 'No model set' }}
          </p>
        </div>
      </div>
      
      <div class="flex items-center gap-2">
        <button
          class="p-2 text-naonur-ash hover:text-naonur-bone transition-colors"
          title="Expand/collapse"
          @click="expanded = !expanded"
        >
          <span :class="['transition-transform inline-block', expanded && 'rotate-180']">▼</span>
        </button>
        <button
          class="p-2 text-naonur-ash hover:text-naonur-blood transition-colors"
          title="Remove agent"
          @click="$emit('remove', agent.id)"
        >
          ✕
        </button>
      </div>
    </div>
    
    <!-- Quick settings row -->
    <div class="grid grid-cols-2 gap-3">
      <ConfigSelect
        :model-value="modelInfo.provider"
        :options="providerOptions"
        label="Provider"
        placeholder="Select provider"
        allow-custom
        @update:model-value="updateProvider"
      />
      <ConfigSelect
        :model-value="modelInfo.model"
        :options="modelOptions"
        label="Model"
        placeholder="Select model"
        :disabled="!modelInfo.provider"
        allow-custom
        @update:model-value="updateModel"
      />
    </div>
    
    <!-- Expanded settings -->
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
          :model-value="agent.workspace || ''"
          label="Workspace"
          placeholder="/path/to/workspace"
          monospace
          @update:model-value="updateWorkspace"
        />
        
        <ConfigToggle
          :model-value="agent.default || false"
          label="Default Agent"
          description="Use this agent when no specific agent is specified"
          @update:model-value="updateDefault"
        />
      </div>
    </Transition>
  </div>
</template>
