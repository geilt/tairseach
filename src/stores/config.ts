import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

// Types for OpenClaw config structure
export interface AgentConfig {
  id: string;
  workspace?: string;
  default?: boolean;
  model?: {
    primary?: string;
  };
  models?: Record<string, { alias?: string }>;
}

export interface ProviderConfig {
  baseUrl?: string;
  apiKey?: string;
  models?: ModelConfig[];
}

export interface ModelConfig {
  id: string;
  name: string;
  api?: string;
  reasoning?: boolean;
  input?: string[];
  cost?: {
    input: number;
    output: number;
    cacheRead?: number;
    cacheWrite?: number;
  };
  contextWindow?: number;
  maxTokens?: number;
}

export interface ModelOption {
  id: string;
  name: string;
  description?: string;
}

export interface OpenClawConfig {
  meta?: {
    lastTouchedVersion?: string;
    lastTouchedAt?: string;
  };
  agents?: {
    defaults?: {
      model?: { primary?: string };
      workspace?: string;
      memorySearch?: Record<string, unknown>;
      compaction?: { mode?: string };
      heartbeat?: Record<string, unknown>;
      maxConcurrent?: number;
      subagents?: { maxConcurrent?: number };
    };
    list?: AgentConfig[];
  };
  models?: {
    mode?: string;
    providers?: Record<string, ProviderConfig>;
  };
  gateway?: {
    port?: number;
    mode?: string;
    bind?: string;
    auth?: {
      mode?: string;
      token?: string;
    };
  };
  channels?: Record<string, unknown>;
  tools?: Record<string, unknown>;
  bindings?: Array<{
    agentId: string;
    match: { channel: string; accountId?: string };
  }>;
  messages?: Record<string, unknown>;
  hooks?: Record<string, unknown>;
  plugins?: Record<string, unknown>;
  [key: string]: unknown;
}

export const useConfigStore = defineStore("config", () => {
  const config = ref<OpenClawConfig>({});
  const originalConfig = ref<string>("");
  const configPath = ref<string>("");
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const providerModels = ref<Record<string, ModelOption[]>>({});

  // Track if config has unsaved changes
  const dirty = computed(() => {
    return JSON.stringify(config.value) !== originalConfig.value;
  });

  // Get list of agents
  const agents = computed(() => config.value.agents?.list || []);

  // Get custom providers
  const customProviders = computed(() => {
    return Object.keys(config.value.models?.providers || {});
  });

  // Get all available providers (built-in + custom)
  const allProviders = computed(() => {
    const builtIn = Object.keys(providerModels.value);
    const custom = customProviders.value;
    return [...new Set([...builtIn, ...custom])];
  });

  async function loadConfig() {
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<{ raw: OpenClawConfig; path: string }>("get_config");
      config.value = result.raw;
      configPath.value = result.path;
      originalConfig.value = JSON.stringify(result.raw);
    } catch (e) {
      error.value = String(e);
      console.error("Failed to load config:", e);
    } finally {
      loading.value = false;
    }
  }

  async function loadProviderModels() {
    try {
      const models = await invoke<Record<string, ModelOption[]>>("get_provider_models");
      providerModels.value = models;
    } catch (e) {
      console.error("Failed to load provider models:", e);
    }
  }

  async function saveConfig() {
    saving.value = true;
    error.value = null;
    try {
      // Update meta before saving
      config.value.meta = {
        ...config.value.meta,
        lastTouchedAt: new Date().toISOString(),
      };
      await invoke("set_config", { config: config.value });
      originalConfig.value = JSON.stringify(config.value);
    } catch (e) {
      error.value = String(e);
      console.error("Failed to save config:", e);
      throw e;
    } finally {
      saving.value = false;
    }
  }

  function revertChanges() {
    if (originalConfig.value) {
      config.value = JSON.parse(originalConfig.value);
    }
  }

  // Agent management
  function updateAgent(id: string, updates: Partial<AgentConfig>) {
    const list = config.value.agents?.list || [];
    const idx = list.findIndex((a) => a.id === id);
    if (idx !== -1) {
      list[idx] = { ...list[idx], ...updates };
    }
  }

  function addAgent(agent: AgentConfig) {
    if (!config.value.agents) {
      config.value.agents = { list: [] };
    }
    if (!config.value.agents.list) {
      config.value.agents.list = [];
    }
    config.value.agents.list.push(agent);
  }

  function removeAgent(id: string) {
    if (config.value.agents?.list) {
      config.value.agents.list = config.value.agents.list.filter((a) => a.id !== id);
    }
  }

  // Get models for a specific provider (combines built-in and custom)
  function getModelsForProvider(provider: string): ModelOption[] {
    const builtIn = providerModels.value[provider] || [];
    const custom = config.value.models?.providers?.[provider]?.models || [];
    const customOptions: ModelOption[] = custom.map((m: ModelConfig) => ({
      id: m.id,
      name: m.name,
      description: m.api || "Custom model",
    }));
    return [...builtIn, ...customOptions];
  }

  // Parse model string like "anthropic/claude-opus-4-5" into provider and model
  function parseModelString(modelStr: string): { provider: string; model: string } {
    const parts = modelStr.split("/");
    if (parts.length === 2) {
      return { provider: parts[0], model: parts[1] };
    }
    return { provider: "", model: modelStr };
  }

  // Build model string from provider and model
  function buildModelString(provider: string, model: string): string {
    return `${provider}/${model}`;
  }

  // Deep update helper
  function updateConfigValue(path: string[], value: unknown) {
    let current: Record<string, unknown> = config.value;
    for (let i = 0; i < path.length - 1; i++) {
      const key = path[i];
      if (!(key in current) || typeof current[key] !== "object") {
        current[key] = {};
      }
      current = current[key] as Record<string, unknown>;
    }
    current[path[path.length - 1]] = value;
  }

  // Get nested value
  function getConfigValue(path: string[]): unknown {
    let current: unknown = config.value;
    for (const key of path) {
      if (current === null || typeof current !== "object") {
        return undefined;
      }
      current = (current as Record<string, unknown>)[key];
    }
    return current;
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
  };
});
