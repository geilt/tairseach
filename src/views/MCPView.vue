<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useWorkerPoller, type NamespaceStatus } from '@/composables/useWorkerPoller'
import { api } from '@/api/tairseach'
import SectionHeader from '@/components/common/SectionHeader.vue'
import LoadingState from '@/components/common/LoadingState.vue'
import ErrorBanner from '@/components/common/ErrorBanner.vue'
import EmptyState from '@/components/common/EmptyState.vue'

interface Tool {
  name: string
  description: string
  inputSchema: Record<string, unknown>
  outputSchema?: Record<string, unknown>
  annotations?: Record<string, unknown>
}

interface Manifest {
  id: string
  name: string
  description: string
  category: string
  version: string
  tools: Tool[]
}

const loading = ref(true)
const error = ref<string | null>(null)
const manifests = ref<Manifest[]>([])
const selectedTool = ref<{ manifest: string; tool: Tool } | null>(null)
const testParams = ref('{}')
const testResult = ref<unknown>(null)
const testError = ref<string | null>(null)
const testLoading = ref(false)
const expandedTools = ref<Set<string>>(new Set())

// Use worker-based status poller
const { proxyStatus, namespaceStatuses, socketAlive } = useWorkerPoller()

// OpenClaw integration state
const installingToOpenClaw = ref(false)
const openclawInstallResult = ref<{ success: boolean; message: string } | null>(null)

const allTools = computed(() => {
  return manifests.value.flatMap(manifest => 
    manifest.tools.map(tool => ({
      manifestId: manifest.id,
      manifestName: manifest.name,
      category: manifest.category,
      ...tool
    }))
  )
})

const groupedByCategory = computed(() => {
  const groups: Record<string, Array<Manifest>> = {}
  for (const manifest of manifests.value) {
    const cat = manifest.category || 'other'
    if (!groups[cat]) {
      groups[cat] = []
    }
    groups[cat].push(manifest)
  }
  return groups
})

const skillConfig = computed(() => {
  return JSON.stringify({
    id: "tairseach",
    name: "Tairseach MCP Tools",
    description: "Access to Tairseach MCP capabilities",
    instructions: "~/.openclaw/skills/tairseach/SKILL.md"
  }, null, 2)
})

async function loadManifests() {
  loading.value = true
  error.value = null
  
  try {
    const result = await api.mcp.manifests()
    
    requestAnimationFrame(() => {
      manifests.value = result
      loading.value = false
    })
  } catch (e) {
    requestAnimationFrame(() => {
      error.value = String(e)
      loading.value = false
    })
  }
}

// Status polling is now handled by the useWorkerPoller composable (Web Worker + rAF)

function getNamespaceStatus(manifestId: string): NamespaceStatus | undefined {
  const namespace = manifestId.split('.')[0] || 'default'
  return namespaceStatuses.value.find(s => s.namespace === namespace)
}

function toggleTool(toolKey: string) {
  if (expandedTools.value.has(toolKey)) {
    expandedTools.value.delete(toolKey)
  } else {
    expandedTools.value.add(toolKey)
  }
}

function selectToolForTest(manifestId: string, tool: Tool) {
  selectedTool.value = { manifest: manifestId, tool }
  testParams.value = JSON.stringify({}, null, 2)
  testResult.value = null
  testError.value = null
}

async function testTool() {
  if (!selectedTool.value) return
  
  testLoading.value = true
  testError.value = null
  testResult.value = null
  
  try {
    const params = JSON.parse(testParams.value)
    
    const result = await api.mcp.testTool(selectedTool.value.tool.name, params)
    
    requestAnimationFrame(() => {
      testResult.value = result
      testLoading.value = false
    })
  } catch (e) {
    requestAnimationFrame(() => {
      testError.value = String(e)
      testLoading.value = false
    })
  }
}

async function installToOpenClaw() {
  installingToOpenClaw.value = true
  openclawInstallResult.value = null
  
  try {
    const result = await api.mcp.installToOpenClaw()
    
    requestAnimationFrame(() => {
      openclawInstallResult.value = result
      installingToOpenClaw.value = false
    })
  } catch (e) {
    requestAnimationFrame(() => {
      openclawInstallResult.value = {
        success: false,
        message: `Installation failed: ${e}`
      }
      installingToOpenClaw.value = false
    })
  }
}

function copySkillConfig() {
  navigator.clipboard.writeText(skillConfig.value)
    .then(() => alert('Skill config copied to clipboard!'))
    .catch(e => alert('Failed to copy: ' + e))
}

onMounted(() => {
  loadManifests()
})
</script>

<template>
  <div class="animate-fade-in">
    <!-- Header -->
    <SectionHeader
      title="MCP Tools"
      icon="🔌"
      description="Model Context Protocol tool manifests and testing interface."
    />

    <!-- Status Cards -->
    <div class="grid grid-cols-3 gap-4 mb-6">
      <!-- Proxy Status -->
      <div class="naonur-card">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-display text-naonur-bone">Proxy Server</h3>
          <span 
            :class="[
              'px-2 py-0.5 text-xs rounded-full border font-mono inline-flex items-center gap-1.5',
              proxyStatus.running 
                ? 'bg-naonur-moss/20 text-naonur-moss border-naonur-moss/30' 
                : 'bg-naonur-fog/50 text-naonur-ash border-naonur-fog'
            ]"
          >
            <span>●</span>
            <span>{{ proxyStatus.running ? 'Running' : 'Stopped' }}</span>
          </span>
        </div>
        <p v-if="proxyStatus.socket_path" class="text-xs text-naonur-smoke font-mono break-all">
          {{ proxyStatus.socket_path }}
        </p>
      </div>

      <!-- Socket Status -->
      <div class="naonur-card">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-display text-naonur-bone">Socket</h3>
          <span 
            :class="[
              'px-2 py-0.5 text-xs rounded-full border font-mono inline-flex items-center gap-1.5',
              socketAlive 
                ? 'bg-naonur-moss/20 text-naonur-moss border-naonur-moss/30' 
                : 'bg-naonur-fog/50 text-naonur-ash border-naonur-fog'
            ]"
          >
            <span>●</span>
            <span>{{ socketAlive ? 'Alive' : 'Dead' }}</span>
          </span>
        </div>
        <p class="text-xs text-naonur-smoke">
          {{ socketAlive ? 'Responding' : 'Not responding' }}
        </p>
      </div>

      <!-- Tools Count -->
      <div class="naonur-card">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-display text-naonur-bone">Available Tools</h3>
          <span class="text-naonur-gold font-display text-lg">{{ allTools.length }}</span>
        </div>
        <p class="text-xs text-naonur-smoke">
          {{ manifests.length }} manifests loaded
        </p>
      </div>
    </div>

    <!-- Loading State -->
    <LoadingState v-if="loading" message="Loading manifests..." />

    <!-- Error State -->
    <ErrorBanner v-else-if="error" :message="error" @retry="loadManifests" />

    <template v-else>
      <!-- Tool Browser -->
      <div class="naonur-card mb-6">
        <h2 class="font-display text-lg text-naonur-gold mb-4 flex items-center gap-2">
          🛠️ Tool Browser
          <span class="text-sm text-naonur-smoke font-body">({{ allTools.length }} tools)</span>
        </h2>

        <div class="space-y-4">
          <div v-for="(categoryManifests, category) in groupedByCategory" :key="category">
            <h3 class="text-sm font-display text-naonur-bone mb-2 uppercase tracking-wider opacity-70">
              {{ category }}
            </h3>
            
            <div class="space-y-2 ml-4">
              <div 
                v-for="manifest in categoryManifests" 
                :key="manifest.id"
                class="border border-naonur-fog/30 rounded-lg overflow-hidden"
              >
                <!-- Manifest header with connection status -->
                <div class="px-4 py-3 bg-naonur-fog/10">
                  <div class="flex items-start justify-between">
                    <div class="flex items-start gap-3 flex-1">
                      <!-- Connection Status Indicator -->
                      <div class="flex-shrink-0 pt-1">
                        <div 
                          :class="[
                            'w-2 h-2 rounded-full',
                            getNamespaceStatus(manifest.id)?.connected 
                              ? 'bg-naonur-moss shadow-[0_0_8px_rgba(74,124,89,0.6)]' 
                              : 'bg-naonur-blood shadow-[0_0_8px_rgba(139,0,0,0.4)]'
                          ]"
                          :title="getNamespaceStatus(manifest.id)?.connected ? 'Connected' : 'Disconnected'"
                        />
                      </div>
                      
                      <div class="flex-1">
                        <h4 class="font-display text-naonur-bone">{{ manifest.name }}</h4>
                        <p class="text-xs text-naonur-smoke mt-1">{{ manifest.description }}</p>
                        <p class="text-xs text-naonur-fog font-mono mt-1">
                          {{ manifest.id }} • v{{ manifest.version }}
                        </p>
                      </div>
                    </div>
                    <span class="px-2 py-1 text-xs rounded-full bg-naonur-gold/20 text-naonur-gold">
                      {{ manifest.tools.length }} {{ manifest.tools.length === 1 ? 'tool' : 'tools' }}
                    </span>
                  </div>
                </div>

                <!-- Tools list -->
                <div class="divide-y divide-naonur-fog/20">
                  <div 
                    v-for="tool in manifest.tools" 
                    :key="tool.name"
                    class="px-4 py-3 hover:bg-naonur-fog/5 transition-colors"
                  >
                    <div class="flex items-start justify-between gap-4">
                      <div class="flex-1 min-w-0">
                        <div class="flex items-center gap-2 mb-1">
                          <code class="text-sm text-naonur-gold font-mono">{{ tool.name }}</code>
                          <span 
                            v-if="tool.annotations?.readOnlyHint"
                            class="px-1.5 py-0.5 text-xs rounded bg-naonur-moss/20 text-naonur-moss"
                          >
                            read-only
                          </span>
                          <span 
                            v-if="tool.annotations?.destructiveHint"
                            class="px-1.5 py-0.5 text-xs rounded bg-naonur-blood/20 text-naonur-blood"
                          >
                            destructive
                          </span>
                        </div>
                        <p class="text-sm text-naonur-ash">{{ tool.description }}</p>
                        
                        <!-- Expandable schema -->
                        <button
                          class="text-xs text-naonur-fog hover:text-naonur-bone mt-2"
                          @click="toggleTool(manifest.id + ':' + tool.name)"
                        >
                          {{ expandedTools.has(manifest.id + ':' + tool.name) ? '▼' : '▶' }} View Schema
                        </button>
                        
                        <div 
                          v-if="expandedTools.has(manifest.id + ':' + tool.name)"
                          class="mt-3 space-y-2"
                        >
                          <div>
                            <div class="text-xs text-naonur-smoke mb-1">Input Schema:</div>
                            <pre class="text-xs bg-naonur-void/50 p-2 rounded border border-naonur-fog/30 overflow-auto">{{ JSON.stringify(tool.inputSchema, null, 2) }}</pre>
                          </div>
                          <div>
                            <div class="text-xs text-naonur-smoke mb-1">Output Schema:</div>
                            <pre class="text-xs bg-naonur-void/50 p-2 rounded border border-naonur-fog/30 overflow-auto">{{ JSON.stringify(tool.outputSchema, null, 2) }}</pre>
                          </div>
                        </div>
                      </div>
                      
                      <button
                        class="btn btn-ghost text-xs flex-shrink-0"
                        @click="selectToolForTest(manifest.id, tool)"
                      >
                        Test
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Tool Tester -->
      <div class="naonur-card mb-6">
        <h2 class="font-display text-lg text-naonur-gold mb-4">🧪 Tool Tester</h2>
        
        <EmptyState v-if="!selectedTool" message="Select a tool above to test it" />
        
        <div v-else class="space-y-4">
          <div>
            <div class="text-sm text-naonur-bone mb-2">Selected Tool:</div>
            <code class="text-naonur-gold">{{ selectedTool.tool.name }}</code>
            <span class="text-naonur-smoke text-sm ml-2">({{ selectedTool.manifest }})</span>
          </div>

          <div>
            <label class="block text-sm text-naonur-bone mb-2">Parameters (JSON):</label>
            <textarea
              v-model="testParams"
              class="w-full h-32 px-3 py-2 bg-naonur-void border border-naonur-fog/50 rounded-lg text-naonur-bone font-mono text-sm focus:border-naonur-gold focus:outline-none focus:ring-1 focus:ring-naonur-gold"
              placeholder='{ "param": "value" }'
            />
          </div>

          <button
            class="btn btn-primary"
            :disabled="testLoading"
            @click="testTool"
          >
            {{ testLoading ? 'Testing...' : 'Execute Tool' }}
          </button>

          <div v-if="testResult" class="mt-4">
            <div class="text-sm text-naonur-moss mb-2">✓ Result:</div>
            <pre class="text-xs bg-naonur-void border border-naonur-fog/30 p-3 rounded-lg overflow-auto max-h-96">{{ JSON.stringify(testResult, null, 2) }}</pre>
          </div>

          <div v-if="testError" class="mt-4">
            <div class="text-sm text-naonur-blood mb-2">✗ Error:</div>
            <pre class="text-xs bg-naonur-blood/10 border border-naonur-blood/30 p-3 rounded-lg overflow-auto">{{ testError }}</pre>
          </div>
        </div>
      </div>

      <!-- OpenClaw Integration -->
      <div class="naonur-card">
        <h2 class="font-display text-lg text-naonur-gold mb-4">🦅 OpenClaw Integration</h2>
        
        <p class="text-sm text-naonur-ash mb-4">
          Install Tairseach MCP server to your OpenClaw configuration to use these tools in agent sessions.
        </p>

        <!-- Install Button -->
        <div class="mb-4">
          <button 
            class="btn btn-primary"
            :disabled="installingToOpenClaw"
            @click="installToOpenClaw"
          >
            <span v-if="installingToOpenClaw" class="flex items-center gap-2">
              <span class="animate-spin">⏳</span>
              Installing...
            </span>
            <span v-else>📦 Install to OpenClaw</span>
          </button>
        </div>

        <!-- Installation Result -->
        <Transition
          enter-active-class="transition-all duration-200"
          enter-from-class="opacity-0 max-h-0"
          enter-to-class="opacity-100 max-h-32"
          leave-active-class="transition-all duration-200"
          leave-from-class="opacity-100 max-h-32"
          leave-to-class="opacity-0 max-h-0"
        >
          <div v-if="openclawInstallResult" class="overflow-hidden">
            <div 
              :class="[
                'p-4 rounded-lg border mb-4',
                openclawInstallResult.success 
                  ? 'bg-naonur-moss/10 border-naonur-moss/30 text-naonur-moss' 
                  : 'bg-naonur-blood/10 border-naonur-blood/30 text-naonur-blood'
              ]"
            >
              <p class="text-sm font-medium mb-1">
                {{ openclawInstallResult.success ? '✓ Installation Successful' : '✗ Installation Failed' }}
              </p>
              <p class="text-xs opacity-90">{{ openclawInstallResult.message }}</p>
            </div>

            <!-- Restart OpenClaw Button (only on success) -->
            <div v-if="openclawInstallResult.success" class="mb-4">
              <p class="text-xs text-naonur-smoke mb-2">
                Restart OpenClaw for the changes to take effect:
              </p>
              <button class="btn btn-secondary text-sm">
                🔄 Restart OpenClaw
              </button>
            </div>
          </div>
        </Transition>

        <!-- Skill Config Preview -->
        <div class="pt-4 border-t border-naonur-fog/20">
          <div class="flex items-center gap-3 mb-3">
            <h3 class="text-sm font-display text-naonur-bone">Skill Configuration</h3>
            <button class="btn btn-ghost text-xs" @click="copySkillConfig">
              📋 Copy
            </button>
          </div>
          
          <div>
            <div class="text-sm text-naonur-smoke mb-2">Preview:</div>
            <pre class="text-xs bg-naonur-void border border-naonur-fog/30 p-3 rounded-lg overflow-auto">{{ skillConfig }}</pre>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
</style>
