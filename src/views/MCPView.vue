<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface Tool {
  name: string
  description: string
  inputSchema: Record<string, any>
  outputSchema: Record<string, any>
  annotations?: Record<string, any>
}

interface Manifest {
  id: string
  name: string
  description: string
  category: string
  version: string
  tools: Tool[]
}

interface ProxyStatus {
  running: boolean
  socket_path?: string
}

const loading = ref(true)
const error = ref<string | null>(null)
const manifests = ref<Manifest[]>([])
const proxyStatus = ref<ProxyStatus>({ running: false })
const selectedTool = ref<{ manifest: string; tool: Tool } | null>(null)
const testParams = ref('{}')
const testResult = ref<any>(null)
const testError = ref<string | null>(null)
const testLoading = ref(false)
const expandedTools = ref<Set<string>>(new Set())
const socketAlive = ref(false)

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
  try {
    loading.value = true
    error.value = null
    manifests.value = await invoke<Manifest[]>('get_all_manifests')
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function checkProxyStatus() {
  try {
    proxyStatus.value = await invoke<ProxyStatus>('get_proxy_status')
  } catch (e) {
    console.error('Failed to get proxy status:', e)
  }
}

async function checkSocketStatus() {
  try {
    const result = await invoke<any>('check_socket_alive')
    socketAlive.value = result.alive
  } catch (e) {
    socketAlive.value = false
  }
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
  
  try {
    testLoading.value = true
    testError.value = null
    testResult.value = null
    
    const params = JSON.parse(testParams.value)
    
    const result = await invoke<any>('test_mcp_tool', {
      toolName: selectedTool.value.tool.name,
      params
    })
    
    testResult.value = result
  } catch (e) {
    testError.value = String(e)
  } finally {
    testLoading.value = false
  }
}

function copySkillConfig() {
  navigator.clipboard.writeText(skillConfig.value)
    .then(() => alert('Skill config copied to clipboard!'))
    .catch(e => alert('Failed to copy: ' + e))
}

onMounted(() => {
  loadManifests()
  checkProxyStatus()
  checkSocketStatus()
  
  // Poll status every 5 seconds
  setInterval(() => {
    checkProxyStatus()
    checkSocketStatus()
  }, 5000)
})
</script>

<template>
  <div class="animate-fade-in">
    <!-- Header -->
    <div class="mb-8">
      <h1 class="font-display text-2xl tracking-wider text-naonur-gold mb-2">
        🔌 MCP Tools
      </h1>
      <p class="text-naonur-ash font-body">
        Model Context Protocol tool manifests and testing interface.
      </p>
    </div>

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
    <div v-if="loading" class="naonur-card text-center py-8">
      <p class="text-naonur-ash animate-pulse">Loading manifests...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="naonur-card border-red-500/50">
      <p class="text-red-400 mb-4">{{ error }}</p>
      <button class="btn btn-secondary" @click="loadManifests">
        Retry
      </button>
    </div>

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
                <!-- Manifest header -->
                <div class="px-4 py-3 bg-naonur-fog/10">
                  <div class="flex items-start justify-between">
                    <div>
                      <h4 class="font-display text-naonur-bone">{{ manifest.name }}</h4>
                      <p class="text-xs text-naonur-smoke mt-1">{{ manifest.description }}</p>
                      <p class="text-xs text-naonur-fog font-mono mt-1">
                        {{ manifest.id }} • v{{ manifest.version }}
                      </p>
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
        
        <div v-if="!selectedTool" class="text-center py-8 text-naonur-smoke">
          Select a tool above to test it
        </div>
        
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
          To use Tairseach tools in OpenClaw, create a skill file at 
          <code class="text-naonur-gold">~/.openclaw/skills/tairseach/SKILL.md</code>
        </p>

        <div class="flex items-center gap-3 mb-4">
          <button class="btn btn-primary" @click="copySkillConfig">
            📋 Copy Skill Config
          </button>
          <span class="text-xs text-naonur-smoke">
            (Paste into OpenClaw skill configuration)
          </span>
        </div>

        <div>
          <div class="text-sm text-naonur-smoke mb-2">Preview:</div>
          <pre class="text-xs bg-naonur-void border border-naonur-fog/30 p-3 rounded-lg overflow-auto">{{ skillConfig }}</pre>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.btn-primary {
  @apply bg-naonur-gold text-naonur-void hover:bg-naonur-gold/80 px-4 py-2 rounded-lg transition-colors;
}

.btn-ghost {
  @apply text-naonur-ash hover:text-naonur-bone hover:bg-naonur-fog/20 px-3 py-1.5 rounded-lg transition-colors;
}

.btn-secondary {
  @apply border border-naonur-fog/50 text-naonur-bone hover:bg-naonur-fog/20 px-4 py-2 rounded-lg transition-colors;
}
</style>
