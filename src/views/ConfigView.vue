<script setup lang="ts">
import { onMounted, onActivated, ref, computed } from 'vue'
import { useConfigStore } from '../stores/config'
import ConfigSection from '../components/config/ConfigSection.vue'
import ConfigInput from '../components/config/ConfigInput.vue'
import ConfigSelect from '../components/config/ConfigSelect.vue'
import AgentCard from '../components/config/AgentCard.vue'
import ProviderCard from '../components/config/ProviderCard.vue'

const store = useConfigStore()
const newAgentId = ref('')
const showAddAgent = ref(false)
const saveMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

onMounted(async () => {
  await store.init()
})

onActivated(() => {
  void store.loadConfig({ silent: true })
  void store.loadProviderModels({ silent: true })
})

// Gateway settings
const gatewayPort = computed({
  get: () => store.config.gateway?.port || 18789,
  set: (v) => store.updateConfigValue(['gateway', 'port'], v),
})

const gatewayBind = computed({
  get: () => store.config.gateway?.bind || 'loopback',
  set: (v) => store.updateConfigValue(['gateway', 'bind'], v),
})

const gatewayAuthMode = computed({
  get: () => store.config.gateway?.auth?.mode || 'token',
  set: (v) => store.updateConfigValue(['gateway', 'auth', 'mode'], v),
})

// Agent defaults
const defaultModel = computed({
  get: () => store.config.agents?.defaults?.model?.primary || '',
  set: (v) => store.updateConfigValue(['agents', 'defaults', 'model', 'primary'], v),
})

const defaultWorkspace = computed({
  get: () => store.config.agents?.defaults?.workspace || '',
  set: (v) => store.updateConfigValue(['agents', 'defaults', 'workspace'], v),
})

const maxConcurrent = computed({
  get: () => store.config.agents?.defaults?.maxConcurrent || 4,
  set: (v) => store.updateConfigValue(['agents', 'defaults', 'maxConcurrent'], v),
})

const subagentMaxConcurrent = computed({
  get: () => store.config.agents?.defaults?.subagents?.maxConcurrent || 8,
  set: (v) => store.updateConfigValue(['agents', 'defaults', 'subagents', 'maxConcurrent'], v),
})

// Custom providers
const customProviders = computed(() => {
  const providers = store.config.models?.providers || {}
  return Object.entries(providers).map(([name, config]) => ({ name, config }))
})

// Add new agent
function addNewAgent() {
  if (!newAgentId.value.trim()) return
  store.addAgent({
    id: newAgentId.value.trim(),
    workspace: `/Users/geilt/naonur/${newAgentId.value.trim()}`,
  })
  newAgentId.value = ''
  showAddAgent.value = false
}

// Remove agent
function removeAgent(id: string) {
  if (confirm(`Remove agent "${id}"? This cannot be undone.`)) {
    store.removeAgent(id)
  }
}

// Remove provider
function removeProvider(name: string) {
  if (confirm(`Remove provider "${name}"? This cannot be undone.`)) {
    const providers = { ...store.config.models?.providers }
    delete providers[name]
    store.updateConfigValue(['models', 'providers'], providers)
  }
}

// Save config
async function saveConfig() {
  try {
    await store.saveConfig()
    saveMessage.value = { type: 'success', text: 'Configuration saved successfully!' }
    setTimeout(() => saveMessage.value = null, 3000)
  } catch (e) {
    saveMessage.value = { type: 'error', text: String(e) }
  }
}

// Bind options
const bindOptions = [
  { value: 'loopback', label: 'Loopback Only (localhost)' },
  { value: 'all', label: 'All Interfaces (0.0.0.0)' },
]

// Auth mode options
const authModeOptions = [
  { value: 'token', label: 'Token Authentication' },
  { value: 'none', label: 'No Authentication' },
]
</script>

<template>
  <div class="animate-fade-in">
    <!-- Header -->
    <div class="mb-8 flex items-start justify-between">
      <div>
        <h1 class="font-display text-2xl tracking-wider text-naonur-gold mb-2">
          ⚙️ Configuration
        </h1>
        <p class="text-naonur-ash font-body">
          Edit OpenClaw gateway configuration.
        </p>
        <p v-if="store.configPath" class="text-xs text-naonur-smoke font-mono mt-1">
          {{ store.configPath }}
        </p>
      </div>
      
      <!-- Save/Revert buttons -->
      <div class="flex items-center gap-3">
        <Transition
          enter-active-class="transition-opacity duration-200"
          enter-from-class="opacity-0"
          leave-active-class="transition-opacity duration-200"
          leave-to-class="opacity-0"
        >
          <span 
            v-if="saveMessage" 
            :class="[
              'text-sm',
              saveMessage.type === 'success' ? 'text-naonur-moss' : 'text-naonur-blood'
            ]"
          >
            {{ saveMessage.text }}
          </span>
        </Transition>
        
        <button
          v-if="store.dirty"
          class="btn btn-ghost text-sm"
          @click="store.revertChanges()"
        >
          Discard Changes
        </button>
        
        <button
          :class="[
            'btn text-sm',
            store.dirty ? 'btn-primary' : 'btn-secondary opacity-50',
          ]"
          :disabled="!store.dirty || store.saving"
          @click="saveConfig"
        >
          <span v-if="store.saving">Saving...</span>
          <span v-else>Save Configuration</span>
        </button>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="store.loading" class="naonur-card mb-6 text-center py-8">
      <p class="text-naonur-ash animate-pulse">Loading configuration...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="store.error" class="naonur-card mb-6 border-red-500/50">
      <p class="text-red-400">{{ store.error }}</p>
      <button class="btn btn-secondary mt-4" @click="store.loadConfig()">
        Retry
      </button>
    </div>

    <template v-else>
      <!-- Unsaved changes indicator -->
      <div 
        v-if="store.dirty" 
        class="mb-4 px-4 py-2 bg-naonur-rust/10 border border-naonur-rust/30 rounded-lg text-sm text-naonur-rust flex items-center gap-2"
      >
        <span>●</span>
        <span>You have unsaved changes</span>
      </div>

      <div class="space-y-6">
        <!-- Agents Section -->
        <ConfigSection 
          title="Agents" 
          icon="🤖" 
          description="Configure AI agents and their models"
          :badge="store.agents.length"
        >
          <!-- Agent defaults -->
          <div class="mb-6 p-4 rounded-lg bg-naonur-fog/10 border border-naonur-fog/20">
            <h4 class="text-sm font-display text-naonur-bone mb-4">Default Settings</h4>
            <div class="grid grid-cols-2 gap-4">
              <ConfigInput
                v-model="defaultModel"
                label="Default Model"
                placeholder="anthropic/claude-opus-4-5"
                monospace
              />
              <ConfigInput
                v-model="defaultWorkspace"
                label="Default Workspace"
                placeholder="/path/to/workspace"
                monospace
              />
              <ConfigInput
                v-model="maxConcurrent"
                label="Max Concurrent"
                type="number"
                description="Maximum concurrent agent sessions"
              />
              <ConfigInput
                v-model="subagentMaxConcurrent"
                label="Max Subagents"
                type="number"
                description="Maximum concurrent subagent sessions"
              />
            </div>
          </div>

          <!-- Agent list -->
          <div class="space-y-3">
            <AgentCard 
              v-for="agent in store.agents" 
              :key="agent.id"
              :agent="agent"
              @remove="removeAgent"
            />
          </div>
          
          <!-- Add agent -->
          <div class="mt-4">
            <button
              v-if="!showAddAgent"
              class="btn btn-ghost text-sm w-full"
              @click="showAddAgent = true"
            >
              + Add Agent
            </button>
            <div v-else class="p-4 rounded-lg border border-naonur-fog/30 bg-naonur-void/30">
              <div class="flex gap-3">
                <ConfigInput
                  v-model="newAgentId"
                  placeholder="Agent ID (e.g., myagent)"
                  class="flex-1"
                  @keyup.enter="addNewAgent"
                />
                <button class="btn btn-primary text-sm" @click="addNewAgent">
                  Add
                </button>
                <button class="btn btn-ghost text-sm" @click="showAddAgent = false">
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </ConfigSection>

        <!-- Custom Providers Section -->
        <ConfigSection 
          title="Custom Providers" 
          icon="🔌"
          description="Configure custom model providers (Ollama, local servers)"
          :badge="customProviders.length"
          :default-open="customProviders.length > 0"
        >
          <div v-if="customProviders.length === 0" class="text-center py-6 text-naonur-ash">
            No custom providers configured
          </div>
          <div v-else class="space-y-3">
            <ProviderCard
              v-for="{ name, config } in customProviders"
              :key="name"
              :name="name"
              :config="config"
              @remove="removeProvider"
            />
          </div>
        </ConfigSection>

        <!-- Gateway Section -->
        <ConfigSection 
          title="Gateway" 
          icon="🌐" 
          description="Gateway server settings"
        >
          <div class="grid grid-cols-2 gap-4">
            <ConfigInput
              v-model="gatewayPort"
              label="Port"
              type="number"
              description="Gateway server port"
            />
            <ConfigSelect
              v-model="gatewayBind"
              :options="bindOptions"
              label="Bind Address"
              description="Network interface to bind"
            />
            <ConfigSelect
              v-model="gatewayAuthMode"
              :options="authModeOptions"
              label="Authentication"
              description="Authentication mode"
            />
            <ConfigInput
              :model-value="store.config.gateway?.auth?.token || ''"
              label="Auth Token"
              type="password"
              monospace
              disabled
              description="Token is auto-generated"
            />
          </div>
        </ConfigSection>

        <!-- Channels Section -->
        <ConfigSection 
          title="Channels" 
          icon="📡" 
          description="Messaging channel configuration"
          :default-open="false"
        >
          <div class="space-y-4">
            <!-- Telegram -->
            <div 
              v-if="store.config.channels?.telegram" 
              class="p-4 rounded-lg bg-naonur-fog/10 border border-naonur-fog/20"
            >
              <div class="flex items-center justify-between mb-3">
                <div class="flex items-center gap-2">
                  <span class="text-lg">📱</span>
                  <h4 class="font-display text-naonur-bone">Telegram</h4>
                </div>
                <span 
                  :class="[
                    'px-2 py-0.5 text-xs rounded-full',
                    (store.config.channels.telegram as any)?.enabled 
                      ? 'bg-naonur-moss/20 text-naonur-moss' 
                      : 'bg-naonur-fog/30 text-naonur-ash'
                  ]"
                >
                  {{ (store.config.channels.telegram as any)?.enabled ? 'Enabled' : 'Disabled' }}
                </span>
              </div>
              <p class="text-xs text-naonur-ash">
                {{ Object.keys((store.config.channels.telegram as any)?.accounts || {}).length }} accounts configured
              </p>
            </div>

            <!-- Slack -->
            <div 
              v-if="store.config.channels?.slack" 
              class="p-4 rounded-lg bg-naonur-fog/10 border border-naonur-fog/20"
            >
              <div class="flex items-center justify-between mb-3">
                <div class="flex items-center gap-2">
                  <span class="text-lg">💬</span>
                  <h4 class="font-display text-naonur-bone">Slack</h4>
                </div>
                <span 
                  :class="[
                    'px-2 py-0.5 text-xs rounded-full',
                    (store.config.channels.slack as any)?.enabled 
                      ? 'bg-naonur-moss/20 text-naonur-moss' 
                      : 'bg-naonur-fog/30 text-naonur-ash'
                  ]"
                >
                  {{ (store.config.channels.slack as any)?.enabled ? 'Enabled' : 'Disabled' }}
                </span>
              </div>
              <p class="text-xs text-naonur-ash">
                {{ Object.keys((store.config.channels.slack as any)?.accounts || {}).length }} accounts configured
              </p>
            </div>
          </div>
          
          <p class="text-xs text-naonur-smoke mt-4 text-center">
            Channel configuration is complex. For detailed editing, use the JSON editor below.
          </p>
        </ConfigSection>

        <!-- Tools Section -->
        <ConfigSection 
          title="Tools" 
          icon="🔧" 
          description="Tool and integration settings"
          :default-open="false"
        >
          <div class="space-y-4">
            <!-- Web Search -->
            <div class="p-4 rounded-lg bg-naonur-fog/10 border border-naonur-fog/20">
              <div class="flex items-center gap-2 mb-3">
                <span class="text-lg">🔍</span>
                <h4 class="font-display text-naonur-bone">Web Search</h4>
              </div>
              <div class="grid grid-cols-2 gap-4">
                <ConfigInput
                  :model-value="(store.config.tools?.web as any)?.search?.provider || ''"
                  label="Provider"
                  disabled
                />
                <ConfigInput
                  :model-value="(store.config.tools?.web as any)?.search?.apiKey ? '••••••••' : 'Not set'"
                  label="API Key"
                  type="password"
                  disabled
                />
              </div>
            </div>

            <!-- Agent to Agent -->
            <div class="p-4 rounded-lg bg-naonur-fog/10 border border-naonur-fog/20">
              <div class="flex items-center justify-between mb-3">
                <div class="flex items-center gap-2">
                  <span class="text-lg">🔗</span>
                  <h4 class="font-display text-naonur-bone">Agent-to-Agent</h4>
                </div>
                <span 
                  :class="[
                    'px-2 py-0.5 text-xs rounded-full',
                    (store.config.tools?.agentToAgent as any)?.enabled 
                      ? 'bg-naonur-moss/20 text-naonur-moss' 
                      : 'bg-naonur-fog/30 text-naonur-ash'
                  ]"
                >
                  {{ (store.config.tools?.agentToAgent as any)?.enabled ? 'Enabled' : 'Disabled' }}
                </span>
              </div>
              <p class="text-xs text-naonur-ash">
                Allowed: {{ ((store.config.tools?.agentToAgent as any)?.allow || []).join(', ') }}
              </p>
            </div>
          </div>
        </ConfigSection>

        <!-- Raw JSON Section -->
        <ConfigSection 
          title="Raw JSON" 
          icon="📄" 
          description="View and edit raw configuration (advanced)"
          :default-open="false"
        >
          <div class="relative">
            <pre class="p-4 rounded-lg bg-naonur-void border border-naonur-fog/30 text-xs font-mono text-naonur-smoke overflow-auto max-h-96">{{ JSON.stringify(store.config, null, 2) }}</pre>
          </div>
          <p class="text-xs text-naonur-smoke mt-2 text-center">
            Direct JSON editing coming soon. For now, use the form fields above.
          </p>
        </ConfigSection>
      </div>

      <div class="threshold-line my-8"></div>

      <p class="text-center text-naonur-smoke text-sm font-mono">
        Configuration changes require a gateway restart to take effect.
      </p>
    </template>
  </div>
</template>

<style scoped>
.btn-primary {
  @apply bg-naonur-gold text-naonur-void hover:bg-naonur-gold/80;
}

.btn-ghost {
  @apply text-naonur-ash hover:text-naonur-bone hover:bg-naonur-fog/20;
}

.btn-secondary {
  @apply border border-naonur-fog/50 text-naonur-bone hover:bg-naonur-fog/20;
}

pre {
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
