<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import SectionHeader from '@/components/common/SectionHeader.vue'
import LoadingState from '@/components/common/LoadingState.vue'
import ErrorBanner from '@/components/common/ErrorBanner.vue'
import EmptyState from '@/components/common/EmptyState.vue'
import { api } from '@/api/tairseach'
import { useWorkerPoller } from '@/composables/useWorkerPoller'
import type { Manifest, Permission, CredentialMetadata } from '@/api/types'

const router = useRouter()

// State
const loading = ref(true)
const error = ref<string | null>(null)
const manifests = ref<Manifest[]>([])
const credentials = ref<CredentialMetadata[]>([])
const permissions = ref<Permission[]>([])

// Collapsible sections
const expandedSections = ref<Set<string>>(new Set(['native']))
const expandedIntegrations = ref<Set<string>>(new Set())
const expandedTools = ref<Set<string>>(new Set())

// Test state
const testing = ref<string | null>(null)
const testResult = ref<Record<string, unknown>>({})
const testError = ref<Record<string, string>>({})

// MCP install state
const installingToOpenClaw = ref(false)
const openclawInstallResult = ref<{ success: boolean; message: string } | null>(null)

const { proxyStatus, namespaceStatuses, socketAlive } = useWorkerPoller()

// Category emoji mapping
const CATEGORY_EMOJI: Record<string, string> = {
  productivity: '📊',
  communication: '📧',
  security: '🔐',
  health: '💍',
  collaboration: '📋',
}

const INTEGRATION_EMOJI: Record<string, string> = {
  contacts: '👥',
  calendar: '📅',
  reminders: '📝',
  location: '📍',
  screen: '🖥️',
  files: '📁',
  auth: '🔐',
  permissions: '🛡️',
  automation: '🤖',
  server: '⚙️',
  config: '⚙️',
  onepassword: '🔑',
  jira: '📋',
  oura: '💍',
  'google-gmail': '📧',
  'google-calendar-api': '📅',
}

// Helper: Get emoji for manifest
function getEmoji(manifest: Manifest): string {
  return INTEGRATION_EMOJI[manifest.id] || CATEGORY_EMOJI[manifest.category] || '🔗'
}

// Helper: Check if permission is granted
function permissionGranted(permId: string): boolean {
  return permissions.value.some(p => p.id === permId && p.status === 'granted')
}

// Helper: Check if credential exists
function hasCredential(provider?: string, id?: string): boolean {
  if (!provider && !id) return true
  
  const search = (provider || id || '').toLowerCase()
  return credentials.value.some(c => {
    const type = c.type.toLowerCase()
    return type.includes(search) || search.includes(type)
  })
}

// Helper: Get namespace status
function getNamespaceStatus(manifestId: string) {
  const namespace = manifestId.split('.')[0] || manifestId
  return namespaceStatuses.value.find(s => s.namespace === namespace)
}

// Categorize manifests
const nativeIntegrations = computed(() =>
  manifests.value.filter(m => m.category === 'security' || 
    ['contacts', 'calendar', 'reminders', 'location', 'screen', 'files', 'automation', 'server', 'config', 'permissions', 'auth'].includes(m.id))
)

const cloudIntegrations = computed(() =>
  manifests.value.filter(m => !nativeIntegrations.value.find(n => n.id === m.id))
)

// Status helpers
interface IntegrationStatus {
  connected: boolean
  permissionsOk: boolean
  credentialsOk: boolean
  missingPermissions: string[]
  missingCredentials: string[]
}

function getIntegrationStatus(manifest: Manifest): IntegrationStatus {
  const nsStatus = getNamespaceStatus(manifest.id)
  const connected = nsStatus?.connected ?? false
  
  const requiredPerms = manifest.requires?.permissions?.map((p: any) => p.name) ?? []
  const missingPermissions = requiredPerms.filter((p: string) => !permissionGranted(p))
  
  const requiredCreds = manifest.requires?.credentials ?? []
  const missingCredentials = requiredCreds
    .filter((c: any) => !hasCredential(c.provider, c.id))
    .map((c: any) => c.provider || c.id || 'unknown')
  
  return {
    connected,
    permissionsOk: missingPermissions.length === 0,
    credentialsOk: missingCredentials.length === 0,
    missingPermissions,
    missingCredentials,
  }
}

// Toggle helpers
function toggleSection(section: string) {
  if (expandedSections.value.has(section)) {
    expandedSections.value.delete(section)
  } else {
    expandedSections.value.add(section)
  }
}

function toggleIntegration(id: string) {
  if (expandedIntegrations.value.has(id)) {
    expandedIntegrations.value.delete(id)
  } else {
    expandedIntegrations.value.add(id)
  }
}

function toggleTools(id: string) {
  if (expandedTools.value.has(id)) {
    expandedTools.value.delete(id)
  } else {
    expandedTools.value.add(id)
  }
}

// Load data
async function loadAll() {
  loading.value = true
  error.value = null
  
  try {
    const [allManifests, allCreds, allPerms] = await Promise.all([
      api.mcp.manifests().catch(() => []),
      api.auth.credentialsList().catch(() => []),
      api.permissions.all().catch(() => []),
    ])
    
    manifests.value = allManifests
    credentials.value = allCreds
    permissions.value = allPerms
  } catch (e) {
    error.value = `Failed to load integrations: ${e}`
  } finally {
    loading.value = false
  }
}

// Test tool
function getTestTool(manifest: Manifest) {
  if (!manifest.tools?.length) return null
  const readOnly = manifest.tools.find(t => t.annotations?.readOnlyHint)
  return readOnly || manifest.tools[0]
}

async function runTest(manifest: Manifest) {
  const tool = getTestTool(manifest)
  if (!tool) return
  
  testing.value = manifest.id
  delete testResult.value[manifest.id]
  delete testError.value[manifest.id]
  
  try {
    const result = await api.mcp.testTool(tool.name, {})
    testResult.value = { ...testResult.value, [manifest.id]: result }
  } catch (e) {
    testError.value = { ...testError.value, [manifest.id]: String(e) }
  } finally {
    testing.value = null
  }
}

// Navigation
function goToAuth(provider?: string) {
  router.push({ path: '/auth', query: provider ? { credential: provider } : undefined })
}

function goToPermissions() {
  router.push('/permissions')
}

// MCP install
async function installToOpenClaw() {
  installingToOpenClaw.value = true
  openclawInstallResult.value = null
  
  try {
    const result = await api.mcp.installToOpenClaw()
    openclawInstallResult.value = result
  } catch (e) {
    openclawInstallResult.value = { success: false, message: String(e) }
  } finally {
    installingToOpenClaw.value = false
  }
}

// Auto-refresh on namespace changes
watch(namespaceStatuses, () => {
  if (!loading.value && manifests.value.length > 0) {
    void loadAll()
  }
}, { deep: true })

onMounted(loadAll)
</script>

<template>
  <div class="animate-fade-in">
    <SectionHeader
      title="Integrations"
      icon="🔗"
      description="Bridges to the Otherworld — All capabilities in one view."
    />

    <!-- Status Overview -->
    <div class="grid grid-cols-3 gap-4 mb-6">
      <div class="naonur-card">
        <p class="text-xs text-naonur-smoke mb-1">Proxy</p>
        <div class="flex items-center gap-2">
          <span :class="proxyStatus.running ? 'text-naonur-moss' : 'text-naonur-blood'">●</span>
          <p :class="proxyStatus.running ? 'text-naonur-moss' : 'text-naonur-blood'" class="text-sm font-mono">
            {{ proxyStatus.running ? 'Running' : 'Stopped' }}
          </p>
        </div>
      </div>
      <div class="naonur-card">
        <p class="text-xs text-naonur-smoke mb-1">Socket</p>
        <div class="flex items-center gap-2">
          <span :class="socketAlive ? 'text-naonur-moss' : 'text-naonur-blood'">●</span>
          <p :class="socketAlive ? 'text-naonur-moss' : 'text-naonur-blood'" class="text-sm font-mono">
            {{ socketAlive ? 'Alive' : 'Dead' }}
          </p>
        </div>
      </div>
      <div class="naonur-card">
        <p class="text-xs text-naonur-smoke mb-1">Integrations</p>
        <p class="text-naonur-bone text-lg font-display">{{ manifests.length }}</p>
      </div>
    </div>

    <LoadingState v-if="loading" message="Loading integrations..." />
    <ErrorBanner v-else-if="error" :message="error" @retry="loadAll" />

    <div v-else class="space-y-6">
      <!-- Tairseach Native Section -->
      <section class="naonur-card">
        <button
          class="w-full flex items-center justify-between text-left"
          @click="toggleSection('native')"
        >
          <div class="flex items-center gap-3">
            <span class="text-2xl">⚙️</span>
            <div>
              <h2 class="font-display text-lg text-naonur-gold">Tairseach Native</h2>
              <p class="text-xs text-naonur-smoke">Built-in macOS integrations</p>
            </div>
          </div>
          <div class="flex items-center gap-4">
            <span class="text-sm text-naonur-smoke font-mono">{{ nativeIntegrations.length }} integrations</span>
            <span class="text-naonur-ash">{{ expandedSections.has('native') ? '▼' : '▶' }}</span>
          </div>
        </button>

        <Transition
          enter-active-class="transition-all duration-300 ease-out"
          leave-active-class="transition-all duration-300 ease-in"
          enter-from-class="opacity-0 max-h-0"
          enter-to-class="opacity-100 max-h-screen"
          leave-from-class="opacity-100 max-h-screen"
          leave-to-class="opacity-0 max-h-0"
        >
          <div v-if="expandedSections.has('native')" class="mt-4 space-y-3 overflow-hidden">
            <div
              v-for="manifest in nativeIntegrations"
              :key="manifest.id"
              class="border border-naonur-fog/20 rounded-lg overflow-hidden"
            >
              <button
                class="w-full px-4 py-3 flex items-start justify-between bg-naonur-fog/5 hover:bg-naonur-fog/10 transition-colors text-left"
                @click="toggleIntegration(manifest.id)"
              >
                <div class="flex items-start gap-3 flex-1">
                  <span class="text-2xl flex-shrink-0">{{ getEmoji(manifest) }}</span>
                  <div class="flex-1 min-w-0">
                    <h3 class="font-display text-naonur-bone">{{ manifest.name }}</h3>
                    <p class="text-xs text-naonur-smoke mt-0.5">{{ manifest.description }}</p>
                    
                    <!-- Status Icons -->
                    <div class="flex items-center gap-3 mt-2 text-xs">
                      <div :class="getIntegrationStatus(manifest).connected ? 'text-naonur-moss' : 'text-naonur-fog'">
                        <span :title="getIntegrationStatus(manifest).connected ? 'Connected' : 'Disconnected'">
                          {{ getIntegrationStatus(manifest).connected ? '✓' : '○' }} Connection
                        </span>
                      </div>
                      <div :class="getIntegrationStatus(manifest).permissionsOk ? 'text-naonur-moss' : 'text-naonur-blood'">
                        <span :title="getIntegrationStatus(manifest).permissionsOk ? 'Permissions OK' : 'Permissions missing'">
                          {{ getIntegrationStatus(manifest).permissionsOk ? '✓' : '✗' }} Permissions
                        </span>
                      </div>
                      <div class="text-naonur-fog">
                        <span>{{ manifest.tools?.length || 0 }} tools</span>
                      </div>
                    </div>
                  </div>
                </div>
                <span class="text-naonur-ash ml-2">{{ expandedIntegrations.has(manifest.id) ? '▼' : '▶' }}</span>
              </button>

              <!-- Expanded Details -->
              <Transition
                enter-active-class="transition-all duration-200"
                leave-active-class="transition-all duration-200"
                enter-from-class="opacity-0 max-h-0"
                enter-to-class="opacity-100 max-h-96"
                leave-from-class="opacity-100 max-h-96"
                leave-to-class="opacity-0 max-h-0"
              >
                <div v-if="expandedIntegrations.has(manifest.id)" class="px-4 py-3 bg-naonur-void/20 border-t border-naonur-fog/20 overflow-hidden">
                  <!-- Missing Permissions -->
                  <div v-if="getIntegrationStatus(manifest).missingPermissions.length" class="mb-3">
                    <p class="text-xs text-naonur-blood mb-2">Missing permissions:</p>
                    <div class="flex flex-wrap gap-2">
                      <span
                        v-for="perm in getIntegrationStatus(manifest).missingPermissions"
                        :key="perm"
                        class="px-2 py-1 text-xs bg-naonur-blood/10 text-naonur-blood rounded border border-naonur-blood/30"
                      >
                        {{ perm }}
                      </span>
                    </div>
                    <button class="btn btn-ghost text-xs mt-2" @click="goToPermissions">
                      Grant Permissions →
                    </button>
                  </div>

                  <!-- Tools List -->
                  <div>
                    <button class="text-xs text-naonur-gold mb-2" @click="toggleTools(manifest.id)">
                      {{ expandedTools.has(manifest.id) ? '▼' : '▶' }} Tools ({{ manifest.tools?.length || 0 }})
                    </button>
                    
                    <div v-if="expandedTools.has(manifest.id)" class="space-y-2 mt-2">
                      <div
                        v-for="tool in manifest.tools"
                        :key="tool.name"
                        class="bg-naonur-void/30 border border-naonur-fog/20 rounded p-2"
                      >
                        <code class="text-xs text-naonur-gold">{{ tool.name }}</code>
                        <p class="text-xs text-naonur-ash mt-1">{{ tool.description }}</p>
                        <div class="flex gap-2 mt-1">
                          <span v-if="tool.annotations?.readOnlyHint" class="text-xs px-1.5 py-0.5 bg-naonur-moss/10 text-naonur-moss rounded">read-only</span>
                          <span v-if="tool.annotations?.destructiveHint" class="text-xs px-1.5 py-0.5 bg-naonur-blood/10 text-naonur-blood rounded">destructive</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  <!-- Test Button -->
                  <button
                    v-if="getTestTool(manifest)"
                    class="btn btn-ghost text-xs mt-3"
                    :disabled="testing === manifest.id"
                    @click="runTest(manifest)"
                  >
                    {{ testing === manifest.id ? 'Testing...' : 'Test' }}
                  </button>

                  <!-- Test Results -->
                  <pre v-if="testResult[manifest.id]" class="mt-2 text-xs bg-naonur-void border border-naonur-fog/30 p-2 rounded overflow-auto max-h-32">{{ JSON.stringify(testResult[manifest.id], null, 2) }}</pre>
                  <pre v-if="testError[manifest.id]" class="mt-2 text-xs bg-naonur-blood/10 border border-naonur-blood/30 p-2 rounded overflow-auto">{{ testError[manifest.id] }}</pre>
                </div>
              </Transition>
            </div>

            <EmptyState v-if="!nativeIntegrations.length" message="No native integrations found" />
          </div>
        </Transition>
      </section>

      <!-- Cloud Integrations Section -->
      <section class="naonur-card">
        <button
          class="w-full flex items-center justify-between text-left"
          @click="toggleSection('cloud')"
        >
          <div class="flex items-center gap-3">
            <span class="text-2xl">☁️</span>
            <div>
              <h2 class="font-display text-lg text-naonur-gold">Cloud Services</h2>
              <p class="text-xs text-naonur-smoke">External API integrations</p>
            </div>
          </div>
          <div class="flex items-center gap-4">
            <span class="text-sm text-naonur-smoke font-mono">{{ cloudIntegrations.length }} integrations</span>
            <span class="text-naonur-ash">{{ expandedSections.has('cloud') ? '▼' : '▶' }}</span>
          </div>
        </button>

        <Transition
          enter-active-class="transition-all duration-300 ease-out"
          leave-active-class="transition-all duration-300 ease-in"
          enter-from-class="opacity-0 max-h-0"
          enter-to-class="opacity-100 max-h-screen"
          leave-from-class="opacity-100 max-h-screen"
          leave-to-class="opacity-0 max-h-0"
        >
          <div v-if="expandedSections.has('cloud')" class="mt-4 space-y-3 overflow-hidden">
            <div
              v-for="manifest in cloudIntegrations"
              :key="manifest.id"
              class="border border-naonur-fog/20 rounded-lg overflow-hidden"
            >
              <button
                class="w-full px-4 py-3 flex items-start justify-between bg-naonur-fog/5 hover:bg-naonur-fog/10 transition-colors text-left"
                @click="toggleIntegration(manifest.id)"
              >
                <div class="flex items-start gap-3 flex-1">
                  <span class="text-2xl flex-shrink-0">{{ getEmoji(manifest) }}</span>
                  <div class="flex-1 min-w-0">
                    <h3 class="font-display text-naonur-bone">{{ manifest.name }}</h3>
                    <p class="text-xs text-naonur-smoke mt-0.5">{{ manifest.description }}</p>
                    
                    <!-- Status Icons -->
                    <div class="flex items-center gap-3 mt-2 text-xs">
                      <div :class="getIntegrationStatus(manifest).connected ? 'text-naonur-moss' : 'text-naonur-fog'">
                        <span :title="getIntegrationStatus(manifest).connected ? 'Connected' : 'Disconnected'">
                          {{ getIntegrationStatus(manifest).connected ? '✓' : '○' }} Connection
                        </span>
                      </div>
                      <div :class="getIntegrationStatus(manifest).credentialsOk ? 'text-naonur-moss' : 'text-naonur-blood'">
                        <span :title="getIntegrationStatus(manifest).credentialsOk ? 'Credentials OK' : 'Credentials missing'">
                          {{ getIntegrationStatus(manifest).credentialsOk ? '✓' : '✗' }} Credentials
                        </span>
                      </div>
                      <div class="text-naonur-fog">
                        <span>{{ manifest.tools?.length || 0 }} tools</span>
                      </div>
                    </div>
                  </div>
                </div>
                <span class="text-naonur-ash ml-2">{{ expandedIntegrations.has(manifest.id) ? '▼' : '▶' }}</span>
              </button>

              <!-- Expanded Details -->
              <Transition
                enter-active-class="transition-all duration-200"
                leave-active-class="transition-all duration-200"
                enter-from-class="opacity-0 max-h-0"
                enter-to-class="opacity-100 max-h-96"
                leave-from-class="opacity-100 max-h-96"
                leave-to-class="opacity-0 max-h-0"
              >
                <div v-if="expandedIntegrations.has(manifest.id)" class="px-4 py-3 bg-naonur-void/20 border-t border-naonur-fog/20 overflow-hidden">
                  <!-- Missing Credentials -->
                  <div v-if="getIntegrationStatus(manifest).missingCredentials.length" class="mb-3">
                    <p class="text-xs text-naonur-blood mb-2">Missing credentials:</p>
                    <div class="flex flex-wrap gap-2">
                      <span
                        v-for="cred in getIntegrationStatus(manifest).missingCredentials"
                        :key="cred"
                        class="px-2 py-1 text-xs bg-naonur-blood/10 text-naonur-blood rounded border border-naonur-blood/30"
                      >
                        {{ cred }}
                      </span>
                    </div>
                    <button class="btn btn-ghost text-xs mt-2" @click="goToAuth(getIntegrationStatus(manifest).missingCredentials[0])">
                      Configure Credentials →
                    </button>
                  </div>

                  <!-- Tools List -->
                  <div>
                    <button class="text-xs text-naonur-gold mb-2" @click="toggleTools(manifest.id)">
                      {{ expandedTools.has(manifest.id) ? '▼' : '▶' }} Tools ({{ manifest.tools?.length || 0 }})
                    </button>
                    
                    <div v-if="expandedTools.has(manifest.id)" class="space-y-2 mt-2">
                      <div
                        v-for="tool in manifest.tools"
                        :key="tool.name"
                        class="bg-naonur-void/30 border border-naonur-fog/20 rounded p-2"
                      >
                        <code class="text-xs text-naonur-gold">{{ tool.name }}</code>
                        <p class="text-xs text-naonur-ash mt-1">{{ tool.description }}</p>
                        <div class="flex gap-2 mt-1">
                          <span v-if="tool.annotations?.readOnlyHint" class="text-xs px-1.5 py-0.5 bg-naonur-moss/10 text-naonur-moss rounded">read-only</span>
                          <span v-if="tool.annotations?.destructiveHint" class="text-xs px-1.5 py-0.5 bg-naonur-blood/10 text-naonur-blood rounded">destructive</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  <!-- Test Button -->
                  <button
                    v-if="getTestTool(manifest)"
                    class="btn btn-ghost text-xs mt-3"
                    :disabled="testing === manifest.id"
                    @click="runTest(manifest)"
                  >
                    {{ testing === manifest.id ? 'Testing...' : 'Test' }}
                  </button>

                  <!-- Test Results -->
                  <pre v-if="testResult[manifest.id]" class="mt-2 text-xs bg-naonur-void border border-naonur-fog/30 p-2 rounded overflow-auto max-h-32">{{ JSON.stringify(testResult[manifest.id], null, 2) }}</pre>
                  <pre v-if="testError[manifest.id]" class="mt-2 text-xs bg-naonur-blood/10 border border-naonur-blood/30 p-2 rounded overflow-auto">{{ testError[manifest.id] }}</pre>
                </div>
              </Transition>
            </div>

            <EmptyState v-if="!cloudIntegrations.length" message="No cloud integrations configured" />
          </div>
        </Transition>
      </section>

      <!-- MCP / OpenClaw Section -->
      <section class="naonur-card">
        <button
          class="w-full flex items-center justify-between text-left"
          @click="toggleSection('mcp')"
        >
          <div class="flex items-center gap-3">
            <span class="text-2xl">🔌</span>
            <div>
              <h2 class="font-display text-lg text-naonur-gold">OpenClaw Integration</h2>
              <p class="text-xs text-naonur-smoke">MCP server installation and status</p>
            </div>
          </div>
          <span class="text-naonur-ash">{{ expandedSections.has('mcp') ? '▼' : '▶' }}</span>
        </button>

        <Transition
          enter-active-class="transition-all duration-300 ease-out"
          leave-active-class="transition-all duration-300 ease-in"
          enter-from-class="opacity-0 max-h-0"
          enter-to-class="opacity-100 max-h-screen"
          leave-from-class="opacity-100 max-h-screen"
          leave-to-class="opacity-0 max-h-0"
        >
          <div v-if="expandedSections.has('mcp')" class="mt-4 overflow-hidden">
            <p class="text-sm text-naonur-ash mb-4">
              Install Tairseach as an MCP server in OpenClaw to use these tools in agent sessions.
            </p>

            <button
              class="btn btn-primary mb-4"
              :disabled="installingToOpenClaw"
              @click="installToOpenClaw"
            >
              {{ installingToOpenClaw ? '⏳ Installing...' : '📦 Install to OpenClaw' }}
            </button>

            <div v-if="openclawInstallResult" :class="['p-4 rounded-lg border', openclawInstallResult.success ? 'bg-naonur-moss/10 border-naonur-moss/30 text-naonur-moss' : 'bg-naonur-blood/10 border-naonur-blood/30 text-naonur-blood']">
              <p class="text-sm font-medium mb-1">
                {{ openclawInstallResult.success ? '✓ Installation Successful' : '✗ Installation Failed' }}
              </p>
              <p class="text-xs opacity-90">{{ openclawInstallResult.message }}</p>
            </div>
          </div>
        </Transition>
      </section>
    </div>
  </div>
</template>

<style scoped>
/* Ensure smooth height transitions */
.max-h-0 {
  max-height: 0;
}
.max-h-32 {
  max-height: 8rem;
}
.max-h-96 {
  max-height: 24rem;
}
.max-h-screen {
  max-height: 100vh;
}
</style>
