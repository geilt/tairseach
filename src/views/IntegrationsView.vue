<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import SectionHeader from '@/components/common/SectionHeader.vue'
import LoadingState from '@/components/common/LoadingState.vue'
import ErrorBanner from '@/components/common/ErrorBanner.vue'
import { api } from '@/api/tairseach'

type JsonObj = Record<string, any>
interface ToolDef {
  name: string
  description?: string
  inputSchema?: JsonObj
  requires?: { permissions?: Array<{ name: string }> }
}
interface ManifestDef {
  id: string
  name: string
  description?: string
  category?: string
  tools?: ToolDef[]
  requires?: {
    permissions?: Array<{ name: string }>
    credentials?: Array<{ id?: string; provider?: string; kind?: string }>
  }
}

interface IntegrationCard {
  id: string
  name: string
  description: string
  emoji: string
  tools: ToolDef[]
  credentialTypes: string[]
  permissions: string[]
}

const router = useRouter()
const loading = ref(true)
const error = ref<string | null>(null)
const manifests = ref<ManifestDef[]>([])
const credentials = ref<Array<{ credential_type: string }>>([])
const permissions = ref<Array<{ id: string; status: string }>>([])
const expanded = ref<string | null>(null)
const testing = ref<string | null>(null)
const testResult = ref<Record<string, unknown>>({})
const testError = ref<Record<string, string>>({})

const EMOJI_BY_ID: Record<string, string> = {
  contacts: '👥',
  calendar: '📅',
  reminders: '📝',
  location: '📍',
  screen: '🖥️',
  files: '📁',
  auth: '🔐',
  permissions: '🛡️',
  automation: '🤖',
  onepassword: '🔑',
  jira: '📋',
  oura: '💍',
  'google-gmail': '📧',
  'google-calendar-api': '📅',
}

const CREDENTIAL_ALIASES: Record<string, string[]> = {
  google: ['google', 'google_oauth', 'google-oauth'],
  onepassword: ['1password', 'onepassword'],
  jira: ['jira'],
  oura: ['oura'],
}

function normalizeType(type: string): string {
  return type.toLowerCase().replace(/[^a-z0-9]+/g, '_')
}

function resolveCredentialType(provider?: string, id?: string) {
  const base = provider || id || ''
  const normalized = normalizeType(base)
  if (normalized.includes('onepassword')) return '1password'
  if (normalized.includes('google')) return 'google_oauth'
  if (normalized.includes('jira')) return 'jira'
  if (normalized.includes('oura')) return 'oura'
  return normalized
}

function extractPermissions(manifest: ManifestDef): string[] {
  const top = manifest.requires?.permissions?.map(p => p.name) ?? []
  const fromTools = (manifest.tools ?? []).flatMap(t => t.requires?.permissions?.map(p => p.name) ?? [])
  return [...new Set([...top, ...fromTools])]
}

const cards = computed<IntegrationCard[]>(() => {
  return manifests.value.map((manifest) => {
    const credTypes = (manifest.requires?.credentials ?? [])
      .map(c => resolveCredentialType(c.provider, c.id))
      .filter(Boolean)

    return {
      id: manifest.id,
      name: manifest.name,
      description: manifest.description || 'No description available.',
      emoji: EMOJI_BY_ID[manifest.id] || '🔗',
      tools: manifest.tools ?? [],
      credentialTypes: [...new Set(credTypes)],
      permissions: extractPermissions(manifest),
    }
  }).sort((a, b) => a.name.localeCompare(b.name))
})

function hasCredential(type: string): boolean {
  const wanted = normalizeType(type)
  const all = credentials.value.map(c => normalizeType(c.credential_type))
  if (all.includes(wanted)) return true

  for (const [root, aliases] of Object.entries(CREDENTIAL_ALIASES)) {
    if (wanted.includes(root) || root.includes(wanted)) {
      if (aliases.some(alias => all.includes(normalizeType(alias)))) return true
    }
  }
  return false
}

function permissionGranted(id: string): boolean {
  return permissions.value.some(p => p.id === id && p.status === 'granted')
}

function credentialStatus(card: IntegrationCard) {
  if (card.credentialTypes.length === 0) return { ok: true, missing: [] as string[] }
  const missing = card.credentialTypes.filter(t => !hasCredential(t))
  return { ok: missing.length === 0, missing }
}

function permissionStatus(card: IntegrationCard) {
  if (card.permissions.length === 0) return { ok: true, missing: [] as string[] }
  const missing = card.permissions.filter(p => !permissionGranted(p))
  return { ok: missing.length === 0, missing }
}

async function loadAll() {
  loading.value = true
  error.value = null
  try {
    const [allManifests, allCreds, allPerms] = await Promise.all([
      api.mcp.manifests(),
      api.auth.listCredentials(),
      api.permissions.all(),
    ])

    manifests.value = allManifests as ManifestDef[]
    credentials.value = allCreds
    permissions.value = allPerms.map((p: any) => ({ id: p.id, status: p.status }))
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

function toggleTools(id: string) {
  expanded.value = expanded.value === id ? null : id
}

function goConfigure(type?: string) {
  router.push({ path: '/auth', query: type ? { credential: type } : undefined })
}

function getTestTool(card: IntegrationCard): ToolDef | null {
  if (!card.tools.length) return null
  const zeroArg = card.tools.find(t => (t.inputSchema?.required?.length ?? 0) === 0)
  return zeroArg || card.tools[0]
}

async function runTest(card: IntegrationCard) {
  const tool = getTestTool(card)
  if (!tool) return

  testing.value = card.id
  delete testResult.value[card.id]
  delete testError.value[card.id]

  try {
    const result = await api.mcp.testTool(tool.name, {})
    testResult.value[card.id] = result
  } catch (e) {
    testError.value[card.id] = String(e)
  } finally {
    testing.value = null
  }
}

onMounted(loadAll)
</script>

<template>
  <div class="animate-fade-in">
    <SectionHeader
      title="Integrations"
      icon="🔗"
      description="Available Bridges"
    />
    <p class="-mt-4 mb-6 text-sm text-naonur-smoke">Bridges to the Otherworld — credentials, permissions, and tools at a glance.</p>

    <LoadingState v-if="loading" message="Gathering manifest bridges..." />
    <ErrorBanner v-else-if="error" :message="error" @retry="loadAll" />

    <div v-else class="space-y-4">
      <div v-for="card in cards" :key="card.id" class="naonur-card">
        <div class="flex items-start justify-between gap-3">
          <div class="flex items-start gap-3 flex-1">
            <span class="text-3xl">{{ card.emoji }}</span>
            <div class="flex-1">
              <h3 class="font-display text-lg text-naonur-bone">{{ card.name }}</h3>
              <p class="text-sm text-naonur-ash">{{ card.description }}</p>

              <div class="mt-3 space-y-1 text-xs">
                <div>
                  <span :class="credentialStatus(card).ok ? 'text-naonur-moss' : 'text-naonur-blood'">
                    {{ credentialStatus(card).ok ? '✅ Credentials' : '❌ Credentials' }}
                  </span>
                  <span class="text-naonur-smoke ml-2" v-if="card.credentialTypes.length">{{ card.credentialTypes.join(', ') }}</span>
                </div>
                <div>
                  <span :class="permissionStatus(card).ok ? 'text-naonur-moss' : 'text-naonur-blood'">
                    {{ permissionStatus(card).ok ? '✅ Permissions' : '❌ Permissions' }}
                  </span>
                  <span class="text-naonur-smoke ml-2" v-if="card.permissions.length">{{ card.permissions.join(', ') }}</span>
                </div>
              </div>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <button class="btn btn-ghost text-xs" @click="toggleTools(card.id)">
              {{ expanded === card.id ? 'Hide Tools' : 'Tools' }}
            </button>
            <button
              class="btn btn-secondary text-xs"
              v-if="credentialStatus(card).missing.length"
              @click="goConfigure(credentialStatus(card).missing[0])"
            >
              Configure
            </button>
            <button
              class="btn btn-ghost text-xs"
              :disabled="testing === card.id"
              @click="runTest(card)"
            >{{ testing === card.id ? 'Testing…' : 'Test' }}</button>
          </div>
        </div>

        <div v-if="expanded === card.id" class="mt-4 border-t border-naonur-fog/20 pt-3 space-y-2">
          <div v-for="tool in card.tools" :key="tool.name" class="rounded-lg border border-naonur-fog/20 p-3 bg-naonur-void/30">
            <code class="text-xs text-naonur-gold">{{ tool.name }}</code>
            <p class="text-sm text-naonur-ash mt-1">{{ tool.description || 'No description' }}</p>
          </div>
          <div v-if="!card.tools.length" class="text-sm text-naonur-smoke">No tools defined.</div>
        </div>

        <pre v-if="testResult[card.id]" class="mt-3 text-xs bg-naonur-void border border-naonur-fog/30 p-2 rounded overflow-auto max-h-44">{{ JSON.stringify(testResult[card.id], null, 2) }}</pre>
        <pre v-if="testError[card.id]" class="mt-3 text-xs bg-naonur-blood/10 border border-naonur-blood/30 p-2 rounded overflow-auto">{{ testError[card.id] }}</pre>
      </div>
    </div>
  </div>
</template>
