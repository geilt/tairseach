import { defineStore } from 'pinia'
import { computed, ref, shallowRef } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { loadStateCache, saveStateCache } from '@/composables/useStateCache'

export interface AgentConfig {
  id: string
  workspace?: string
  default?: boolean
  model?: {
    primary?: string
  }
  models?: Record<string, { alias?: string }>
}

export interface ProviderConfig {
  baseUrl?: string
  apiKey?: string
  models?: ModelConfig[]
}

export interface ModelConfig {
  id: string
  name: string
  api?: string
  reasoning?: boolean
  input?: string[]
  cost?: {
    input: number
    output: number
    cacheRead?: number
    cacheWrite?: number
  }
  contextWindow?: number
  maxTokens?: number
}

export interface ModelOption {
  id: string
  name: string
  description?: string
}

export interface OpenClawConfig {
  meta?: { lastTouchedVersion?: string; lastTouchedAt?: string }
  agents?: {
    defaults?: {
      model?: { primary?: string }
      workspace?: string
      memorySearch?: Record<string, unknown>
      compaction?: { mode?: string }
      heartbeat?: Record<string, unknown>
      maxConcurrent?: number
      subagents?: { maxConcurrent?: number }
    }
    list?: AgentConfig[]
  }
  models?: { mode?: string; providers?: Record<string, ProviderConfig> }
  gateway?: { port?: number; mode?: string; bind?: string; auth?: { mode?: string; token?: string } }
  channels?: Record<string, unknown>
  tools?: Record<string, unknown>
  bindings?: Array<{ agentId: string; match: { channel: string; accountId?: string } }>
  messages?: Record<string, unknown>
  hooks?: Record<string, unknown>
  plugins?: Record<string, unknown>
  [key: string]: unknown
}

interface ConfigCacheData {
  config: OpenClawConfig
  configPath: string
  originalConfig: string
  providerModels: Record<string, ModelOption[]>
}

export const useConfigStore = defineStore('config', () => {
  const config = shallowRef<OpenClawConfig>({})
  const originalConfig = ref('')
  const configPath = ref('')
  const loading = ref(false)
  const saving = ref(false)
  const error = ref<string | null>(null)
  const providerModels = shallowRef<Record<string, ModelOption[]>>({})
  const hydrated = ref(false)
  const lastUpdated = ref<string | null>(null)

  const dirty = computed(() => JSON.stringify(config.value) !== originalConfig.value)
  const agents = computed(() => config.value.agents?.list || [])
  const customProviders = computed(() => Object.keys(config.value.models?.providers || {}))
  const allProviders = computed(() => [...new Set([...Object.keys(providerModels.value), ...customProviders.value])])

  function persistCache() {
    const entry = saveStateCache<ConfigCacheData>('config', {
      config: config.value,
      configPath: configPath.value,
      originalConfig: originalConfig.value,
      providerModels: providerModels.value,
    })
    lastUpdated.value = entry.lastUpdated
  }

  function hydrateFromCache() {
    const cached = loadStateCache<ConfigCacheData>('config')
    if (!cached) return false
    config.value = cached.data.config ?? {}
    configPath.value = cached.data.configPath ?? ''
    originalConfig.value = cached.data.originalConfig ?? JSON.stringify(cached.data.config ?? {})
    providerModels.value = cached.data.providerModels ?? {}
    lastUpdated.value = cached.lastUpdated
    return true
  }

  async function init() {
    if (hydrated.value) return
    hydrateFromCache()
    hydrated.value = true
    void loadConfig({ silent: true })
    void loadProviderModels({ silent: true })
  }

  async function loadConfig(opts: { silent?: boolean } = {}) {
    const silent = opts.silent === true
    if (!silent) loading.value = true
    error.value = null
    try {
      const result = await invoke<{ raw: OpenClawConfig; path: string }>('get_config')
      config.value = result.raw
      configPath.value = result.path
      originalConfig.value = JSON.stringify(result.raw)
      persistCache()
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load config:', e)
    } finally {
      if (!silent) loading.value = false
    }
  }

  async function loadProviderModels(_opts: { silent?: boolean } = {}) {
    try {
      providerModels.value = await invoke<Record<string, ModelOption[]>>('get_provider_models')
      persistCache()
    } catch (e) {
      console.error('Failed to load provider models:', e)
    }
  }

  async function saveConfig() {
    saving.value = true
    error.value = null
    try {
      config.value = {
        ...config.value,
        meta: { ...config.value.meta, lastTouchedAt: new Date().toISOString() },
      }
      await invoke('set_config', { config: config.value })
      originalConfig.value = JSON.stringify(config.value)
      persistCache()
    } catch (e) {
      error.value = String(e)
      console.error('Failed to save config:', e)
      throw e
    } finally {
      saving.value = false
    }
  }

  function revertChanges() {
    if (originalConfig.value) config.value = JSON.parse(originalConfig.value)
  }

  function updateAgent(id: string, updates: Partial<AgentConfig>) {
    const list = [...(config.value.agents?.list || [])]
    const idx = list.findIndex((a) => a.id === id)
    if (idx !== -1) {
      list[idx] = { ...list[idx], ...updates }
      config.value = {
        ...config.value,
        agents: { ...(config.value.agents || {}), list },
      }
      persistCache()
    }
  }

  function addAgent(agent: AgentConfig) {
    const list = [...(config.value.agents?.list || []), agent]
    config.value = { ...config.value, agents: { ...(config.value.agents || {}), list } }
    persistCache()
  }

  function removeAgent(id: string) {
    const list = (config.value.agents?.list || []).filter((a) => a.id !== id)
    config.value = { ...config.value, agents: { ...(config.value.agents || {}), list } }
    persistCache()
  }

  function getModelsForProvider(provider: string): ModelOption[] {
    const builtIn = providerModels.value[provider] || []
    const custom = config.value.models?.providers?.[provider]?.models || []
    const customOptions: ModelOption[] = custom.map((m: ModelConfig) => ({
      id: m.id,
      name: m.name,
      description: m.api || 'Custom model',
    }))
    return [...builtIn, ...customOptions]
  }

  function parseModelString(modelStr: string): { provider: string; model: string } {
    const parts = modelStr.split('/')
    return parts.length === 2 ? { provider: parts[0], model: parts[1] } : { provider: '', model: modelStr }
  }

  function buildModelString(provider: string, model: string): string {
    return `${provider}/${model}`
  }

  function updateConfigValue(path: string[], value: unknown) {
    const next = JSON.parse(JSON.stringify(config.value || {})) as Record<string, unknown>
    let current: Record<string, unknown> = next
    for (let i = 0; i < path.length - 1; i++) {
      const key = path[i]
      if (!(key in current) || typeof current[key] !== 'object' || current[key] === null) current[key] = {}
      current = current[key] as Record<string, unknown>
    }
    current[path[path.length - 1]] = value
    config.value = next as OpenClawConfig
    persistCache()
  }

  function getConfigValue(path: string[]): unknown {
    let current: unknown = config.value
    for (const key of path) {
      if (current === null || typeof current !== 'object') return undefined
      current = (current as Record<string, unknown>)[key]
    }
    return current
  }

  return {
    config,
    configPath,
    loading,
    saving,
    error,
    dirty,
    agents,
    customProviders,
    allProviders,
    providerModels,
    hydrated,
    lastUpdated,
    init,
    loadConfig,
    loadProviderModels,
    saveConfig,
    revertChanges,
    updateAgent,
    addAgent,
    removeAgent,
    getModelsForProvider,
    parseModelString,
    buildModelString,
    updateConfigValue,
    getConfigValue,
  }
})
