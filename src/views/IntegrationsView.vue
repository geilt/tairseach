<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { api } from '@/api/tairseach'
import SectionHeader from '@/components/common/SectionHeader.vue'
import LoadingState from '@/components/common/LoadingState.vue'
import ErrorBanner from '@/components/common/ErrorBanner.vue'


interface StoredCredential {
  credential_type: string
  created_at: string
}

interface Tool {
  name: string
  description: string
  inputSchema: Record<string, unknown>
}

interface Integration {
  id: string
  name: string
  emoji: string
  description: string
  credentialType: string
  hasCredentials: boolean
  testTool?: string
  testParams?: Record<string, unknown>
  tools: Tool[]
}

const router = useRouter()
const loading = ref(true)
const error = ref<string | null>(null)
const integrations = ref<Integration[]>([])
const expandedIntegration = ref<string | null>(null)
const expandedTool = ref<string | null>(null)
const testingIntegration = ref<string | null>(null)
const testResults = ref<Record<string, unknown>>({})
const testErrors = ref<Record<string, string>>({})

// Tool tester state
const selectedToolForTest = ref<{ integration: string; tool: Tool } | null>(null)
const toolTestParams = ref('{}')
const toolTestResult = ref<unknown>(null)
const toolTestError = ref<string | null>(null)
const toolTestLoading = ref(false)

/**
 * Integration definitions with test tool mappings
 */
const INTEGRATION_CONFIGS = [
  {
    id: 'google-calendar',
    name: 'Google Calendar',
    emoji: '📅',
    credentialType: 'google_oauth',
    testTool: 'gcalendar.listCalendars',
    testParams: {},
    description: 'Manage calendars and events'
  },
  {
    id: 'gmail',
    name: 'Gmail',
    emoji: '📧',
    credentialType: 'google_oauth',
    testTool: 'gmail.labels',
    testParams: {},
    description: 'Email management and search'
  },
  {
    id: 'google-contacts',
    name: 'Google Contacts',
    emoji: '👥',
    credentialType: 'google_oauth',
    testTool: 'contacts.list',
    testParams: { limit: 5 },
    description: 'Contact management'
  },
  {
    id: '1password',
    name: '1Password',
    emoji: '🔐',
    credentialType: '1password',
    testTool: 'op.vaults.list',
    testParams: {},
    description: 'Password and secrets management'
  },
  {
    id: 'oura',
    name: 'Oura Ring',
    emoji: '💍',
    credentialType: 'oura',
    testTool: 'oura.sleep',
    testParams: {},
    description: 'Sleep and health tracking'
  },
  {
    id: 'jira',
    name: 'Jira',
    emoji: '📋',
    credentialType: 'jira',
    testTool: 'jira.projects',
    testParams: {},
    description: 'Project and issue tracking'
  }
]

/**
 * Load stored credentials
 */
async function loadStoredCredentials(): Promise<StoredCredential[]> {
  try {
    return await api.auth.listCredentials()
  } catch (e) {
    console.error('Failed to load stored credentials:', e)
    return []
  }
}

/**
 * Load tools from MCP manifests
 */
async function loadTools(): Promise<Map<string, Tool[]>> {
  try {
    const manifests = await api.mcp.manifests()
    const toolsByNamespace = new Map<string, Tool[]>()
    
    for (const manifest of manifests) {
      const namespace = manifest.id.split('.')[0]
      const tools = manifest.tools || []
      
      if (!toolsByNamespace.has(namespace)) {
        toolsByNamespace.set(namespace, [])
      }
      
      toolsByNamespace.get(namespace)?.push(...tools)
    }
    
    return toolsByNamespace
  } catch (e) {
    console.error('Failed to load tools:', e)
    return new Map()
  }
}

/**
 * Initialize integrations view
 */
async function loadIntegrations() {
  loading.value = true
  error.value = null

  try {
    const [storedCreds, toolsByNamespace] = await Promise.all([
      loadStoredCredentials(),
      loadTools()
    ])

    const credTypeSet = new Set(storedCreds.map(c => c.credential_type))

    const integrationData = INTEGRATION_CONFIGS.map(config => {
      const hasCredentials = credTypeSet.has(config.credentialType)
      const namespace = config.id.split('-')[0]
      const tools = toolsByNamespace.get(namespace) || []

      return {
        ...config,
        hasCredentials,
        tools
      }
    })
    
    requestAnimationFrame(() => {
      integrations.value = integrationData
      loading.value = false
    })
  } catch (e) {
    requestAnimationFrame(() => {
      error.value = String(e)
      loading.value = false
    })
  }
}

/**
 * Test an integration's diagnostic tool
 */
async function testIntegration(integration: Integration) {
  if (!integration.testTool) {
    requestAnimationFrame(() => {
      testErrors.value[integration.id] = 'No test tool configured'
    })
    return
  }

  testingIntegration.value = integration.id
  delete testResults.value[integration.id]
  delete testErrors.value[integration.id]

  try {
    const result = await api.mcp.testTool(integration.testTool, integration.testParams || {})

    requestAnimationFrame(() => {
      testResults.value[integration.id] = result
      testingIntegration.value = null
    })
  } catch (e) {
    requestAnimationFrame(() => {
      testErrors.value[integration.id] = String(e)
      testingIntegration.value = null
    })
  }
}

/**
 * Navigate to auth configuration
 */
function goToAuth() {
  router.push('/auth')
}

/**
 * Toggle integration expansion
 */
function toggleIntegration(id: string) {
  expandedIntegration.value = expandedIntegration.value === id ? null : id
}

/**
 * Toggle tool expansion
 */
function toggleTool(key: string) {
  expandedTool.value = expandedTool.value === key ? null : key
}

/**
 * Select a tool for testing
 */
function selectToolForTest(integration: Integration, tool: Tool) {
  selectedToolForTest.value = { integration: integration.id, tool }
  toolTestParams.value = JSON.stringify({}, null, 2)
  toolTestResult.value = null
  toolTestError.value = null
}

/**
 * Test a specific tool
 */
async function testTool() {
  if (!selectedToolForTest.value) return

  toolTestLoading.value = true
  toolTestError.value = null
  toolTestResult.value = null

  try {
    const params = JSON.parse(toolTestParams.value)

    const result = await api.mcp.testTool(selectedToolForTest.value.tool.name, params)

    requestAnimationFrame(() => {
      toolTestResult.value = result
      toolTestLoading.value = false
    })
  } catch (e) {
    requestAnimationFrame(() => {
      toolTestError.value = String(e)
      toolTestLoading.value = false
    })
  }
}

/**
 * Clear tool test state
 */
function clearToolTest() {
  selectedToolForTest.value = null
  toolTestParams.value = '{}'
  toolTestResult.value = null
  toolTestError.value = null
}

const groupedIntegrations = computed(() => {
  const connected = integrations.value.filter(i => i.hasCredentials)
  const available = integrations.value.filter(i => !i.hasCredentials)
  return { connected, available }
})

onMounted(() => {
  loadIntegrations()
})
</script>

<template>
  <div class="animate-fade-in">
    <!-- Header -->
    <SectionHeader
      title="Integrations"
      icon="🔗"
      description="Connected services and available tools"
    />

    <!-- Loading State -->
    <LoadingState v-if="loading" message="Loading integrations..." />

    <!-- Error State -->
    <ErrorBanner v-else-if="error" :message="error" @retry="loadIntegrations" />

    <template v-else>
      <!-- Summary Stats -->
      <div class="grid grid-cols-3 gap-4 mb-6">
        <div class="naonur-card">
          <div class="text-sm text-naonur-smoke mb-1">Total Integrations</div>
          <div class="text-2xl font-display text-naonur-gold">
            {{ integrations.length }}
          </div>
        </div>
        <div class="naonur-card">
          <div class="text-sm text-naonur-smoke mb-1">Connected</div>
          <div class="text-2xl font-display text-naonur-moss">
            {{ groupedIntegrations.connected.length }}
          </div>
        </div>
        <div class="naonur-card">
          <div class="text-sm text-naonur-smoke mb-1">Total Tools</div>
          <div class="text-2xl font-display text-naonur-bone">
            {{ integrations.reduce((sum, i) => sum + i.tools.length, 0) }}
          </div>
        </div>
      </div>

      <!-- Connected Integrations -->
      <div v-if="groupedIntegrations.connected.length > 0" class="mb-6">
        <h2 class="font-display text-lg text-naonur-bone mb-4 flex items-center gap-2">
          ✓ Connected
        </h2>
        <div class="grid grid-cols-1 gap-4">
          <div
            v-for="integration in groupedIntegrations.connected"
            :key="integration.id"
            class="naonur-card"
          >
            <!-- Integration Header -->
            <div class="flex items-start justify-between mb-4">
              <div class="flex items-start gap-3 flex-1">
                <span class="text-3xl">{{ integration.emoji }}</span>
                <div class="flex-1">
                  <div class="flex items-center gap-2 mb-1">
                    <h3 class="font-display text-lg text-naonur-bone">
                      {{ integration.name }}
                    </h3>
                    <span class="w-2 h-2 rounded-full bg-naonur-moss shadow-[0_0_8px_rgba(74,124,89,0.6)]" />
                  </div>
                  <p class="text-sm text-naonur-ash">{{ integration.description }}</p>
                  <p class="text-xs text-naonur-smoke font-mono mt-1">
                    {{ integration.tools.length }} {{ integration.tools.length === 1 ? 'tool' : 'tools' }} available
                  </p>
                </div>
              </div>
              <div class="flex gap-2">
                <button
                  v-if="integration.testTool"
                  class="btn btn-ghost text-sm"
                  :disabled="testingIntegration === integration.id"
                  @click="testIntegration(integration)"
                >
                  {{ testingIntegration === integration.id ? '⏳ Testing...' : '🧪 Test' }}
                </button>
                <button class="btn btn-ghost text-sm" @click="goToAuth">
                  ⚙️ Configure
                </button>
                <button
                  class="btn btn-ghost text-sm"
                  @click="toggleIntegration(integration.id)"
                >
                  {{ expandedIntegration === integration.id ? '▼' : '▶' }} Tools
                </button>
              </div>
            </div>

            <!-- Test Result -->
            <div v-if="testResults[integration.id]" class="mb-4">
              <div class="text-sm text-naonur-moss mb-2">✓ Test Result:</div>
              <pre class="text-xs bg-naonur-void border border-naonur-fog/30 p-3 rounded-lg overflow-auto max-h-48">{{ JSON.stringify(testResults[integration.id], null, 2) }}</pre>
            </div>

            <!-- Test Error -->
            <div v-if="testErrors[integration.id]" class="mb-4">
              <div class="text-sm text-naonur-blood mb-2">✗ Test Error:</div>
              <pre class="text-xs bg-naonur-blood/10 border border-naonur-blood/30 p-3 rounded-lg overflow-auto">{{ testErrors[integration.id] }}</pre>
            </div>

            <!-- Tools List (Expandable) -->
            <div v-if="expandedIntegration === integration.id" class="border-t border-naonur-fog/20 pt-4">
              <div class="space-y-2">
                <div
                  v-for="tool in integration.tools"
                  :key="tool.name"
                  class="border border-naonur-fog/20 rounded-lg p-3 hover:bg-naonur-fog/5 transition-colors"
                >
                  <div class="flex items-start justify-between gap-3">
                    <div class="flex-1 min-w-0">
                      <code class="text-sm text-naonur-gold font-mono">{{ tool.name }}</code>
                      <p class="text-sm text-naonur-ash mt-1">{{ tool.description }}</p>
                      
                      <!-- Expandable Schema -->
                      <button
                        class="text-xs text-naonur-fog hover:text-naonur-bone mt-2"
                        @click="toggleTool(integration.id + ':' + tool.name)"
                      >
                        {{ expandedTool === (integration.id + ':' + tool.name) ? '▼' : '▶' }} View Schema
                      </button>
                      
                      <div v-if="expandedTool === (integration.id + ':' + tool.name)" class="mt-3">
                        <div class="text-xs text-naonur-smoke mb-1">Input Schema:</div>
                        <pre class="text-xs bg-naonur-void/50 p-2 rounded border border-naonur-fog/30 overflow-auto max-h-32">{{ JSON.stringify(tool.inputSchema, null, 2) }}</pre>
                      </div>
                    </div>
                    
                    <button
                      class="btn btn-ghost text-xs flex-shrink-0"
                      @click="selectToolForTest(integration, tool)"
                    >
                      Try It
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Available Integrations -->
      <div v-if="groupedIntegrations.available.length > 0">
        <h2 class="font-display text-lg text-naonur-bone mb-4 flex items-center gap-2">
          ○ Available
        </h2>
        <div class="grid grid-cols-2 gap-4">
          <div
            v-for="integration in groupedIntegrations.available"
            :key="integration.id"
            class="naonur-card opacity-60 hover:opacity-100 transition-opacity"
          >
            <div class="flex items-start gap-3 mb-3">
              <span class="text-2xl">{{ integration.emoji }}</span>
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <h3 class="font-display text-naonur-bone">{{ integration.name }}</h3>
                  <span class="w-2 h-2 rounded-full bg-naonur-fog" />
                </div>
                <p class="text-sm text-naonur-smoke">{{ integration.description }}</p>
              </div>
            </div>
            <button class="btn btn-secondary text-sm w-full" @click="goToAuth">
              🔑 Connect
            </button>
          </div>
        </div>
      </div>

      <!-- Tool Tester Modal -->
      <div
        v-if="selectedToolForTest"
        class="fixed inset-0 bg-naonur-void/80 backdrop-blur-sm flex items-center justify-center z-50 p-6"
        @click.self="clearToolTest"
      >
        <div class="naonur-card max-w-2xl w-full max-h-[80vh] overflow-auto">
          <div class="flex items-start justify-between mb-4">
            <div>
              <h3 class="font-display text-lg text-naonur-gold mb-1">Tool Tester</h3>
              <code class="text-sm text-naonur-bone">{{ selectedToolForTest.tool.name }}</code>
            </div>
            <button class="text-naonur-ash hover:text-naonur-bone" @click="clearToolTest">
              ✕
            </button>
          </div>

          <div class="space-y-4">
            <div>
              <label class="block text-sm text-naonur-bone mb-2">Parameters (JSON):</label>
              <textarea
                v-model="toolTestParams"
                class="w-full h-32 px-3 py-2 bg-naonur-void border border-naonur-fog/50 rounded-lg text-naonur-bone font-mono text-sm focus:border-naonur-gold focus:outline-none focus:ring-1 focus:ring-naonur-gold"
                placeholder='{ "param": "value" }'
              />
            </div>

            <button
              class="btn btn-primary w-full"
              :disabled="toolTestLoading"
              @click="testTool"
            >
              {{ toolTestLoading ? 'Executing...' : 'Execute Tool' }}
            </button>

            <div v-if="toolTestResult" class="mt-4">
              <div class="text-sm text-naonur-moss mb-2">✓ Result:</div>
              <pre class="text-xs bg-naonur-void border border-naonur-fog/30 p-3 rounded-lg overflow-auto max-h-64">{{ JSON.stringify(toolTestResult, null, 2) }}</pre>
            </div>

            <div v-if="toolTestError" class="mt-4">
              <div class="text-sm text-naonur-blood mb-2">✗ Error:</div>
              <pre class="text-xs bg-naonur-blood/10 border border-naonur-blood/30 p-3 rounded-lg overflow-auto">{{ toolTestError }}</pre>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>

/* Animate fade in */
@keyframes fade-in {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.animate-fade-in {
  animation: fade-in 0.3s ease-out;
}
</style>
