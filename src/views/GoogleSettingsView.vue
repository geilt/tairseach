<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, shallowRef } from 'vue'

interface GoogleStatus {
  status: 'connected' | 'not_configured' | 'token_expired' | string
  configured: boolean
  has_token: boolean
  message: string
}

interface GoogleConfig {
  client_id: string
  client_secret: string
  updated_at: string
}

const clientId = ref('')
const clientSecret = ref('')
const jsonInput = ref('')
const dragOver = ref(false)
const saving = ref(false)
const testing = ref(false)
const status = shallowRef<GoogleStatus | null>(null)
const feedback = ref<{ type: 'success' | 'error'; text: string } | null>(null)

const instructionsOpen = ref(false)
const instructionsStorageKey = 'tairseach-google-setup-open'

const statusTone = computed(() => {
  const v = status.value?.status
  if (v === 'connected') return 'text-naonur-moss bg-naonur-moss/15 border-naonur-moss/25'
  if (v === 'token_expired') return 'text-naonur-rust bg-naonur-rust/15 border-naonur-rust/25'
  return 'text-naonur-ash bg-naonur-fog/20 border-naonur-fog/30'
})

function setFeedback(type: 'success' | 'error', text: string) {
  feedback.value = { type, text }
  window.setTimeout(() => {
    if (feedback.value?.text === text) feedback.value = null
  }, 3200)
}

async function loadCurrent() {
  const [cfg, st] = await Promise.all([
    invoke<GoogleConfig | null>('get_google_oauth_config'),
    invoke<GoogleStatus>('get_google_oauth_status'),
  ])

  if (cfg) {
    clientId.value = cfg.client_id || ''
    clientSecret.value = cfg.client_secret || ''
  }
  status.value = st
}

function parseAndApplyJson(raw: string) {
  const parsed = JSON.parse(raw)
  const installed = parsed?.installed ?? parsed?.web ?? parsed
  const extractedId = installed?.client_id
  const extractedSecret = installed?.client_secret

  if (!extractedId || !extractedSecret) {
    throw new Error('Could not find client_id and client_secret in JSON')
  }

  clientId.value = extractedId
  clientSecret.value = extractedSecret
  setFeedback('success', 'Parsed client_secret.json and filled fields.')
}

async function handleFile(file: File) {
  const text = await file.text()
  parseAndApplyJson(text)
}

function onFileInput(event: Event) {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return
  void handleFile(file).catch((err) => setFeedback('error', String(err)))
}

function onDrop(event: DragEvent) {
  dragOver.value = false
  const file = event.dataTransfer?.files?.[0]
  if (!file) return
  void handleFile(file).catch((err) => setFeedback('error', String(err)))
}

function applyJsonText() {
  try {
    parseAndApplyJson(jsonInput.value)
  } catch (err) {
    setFeedback('error', String(err))
  }
}

async function saveCredentials() {
  if (!clientId.value.trim() || !clientSecret.value.trim()) {
    setFeedback('error', 'Client ID and Client Secret are required.')
    return
  }

  saving.value = true
  try {
    await invoke('save_google_oauth_config', {
      clientId: clientId.value,
      clientSecret: clientSecret.value,
    })
    await loadCurrent()
    setFeedback('success', 'Google OAuth credentials saved.')
  } catch (err) {
    setFeedback('error', String(err))
  } finally {
    saving.value = false
  }
}

async function testConnection() {
  testing.value = true
  try {
    const result = await invoke<{ ok: boolean; message: string; error?: string }>('test_google_oauth_config', {
      clientId: clientId.value,
      clientSecret: clientSecret.value,
    })

    await loadCurrent()
    if (result.ok) {
      setFeedback('success', result.message)
    } else {
      setFeedback('error', result.message)
    }
  } catch (err) {
    setFeedback('error', String(err))
  } finally {
    testing.value = false
  }
}

function toggleInstructions() {
  instructionsOpen.value = !instructionsOpen.value
  localStorage.setItem(instructionsStorageKey, instructionsOpen.value ? '1' : '0')
}

onMounted(async () => {
  instructionsOpen.value = localStorage.getItem(instructionsStorageKey) === '1'
  await loadCurrent()
})
</script>

<template>
  <div class="animate-fade-in space-y-6">
    <div>
      <h1 class="font-display text-2xl tracking-wider text-naonur-gold mb-2">🟢 Google OAuth</h1>
      <p class="text-naonur-ash font-body">Configure Google Workspace credentials for Gmail and Calendar access.</p>
    </div>

    <section class="naonur-card card-contain p-5 space-y-4">
      <div class="flex items-center justify-between gap-3">
        <h2 class="font-display text-naonur-bone">Credential Status</h2>
        <span :class="['px-2.5 py-1 text-xs border rounded-full', statusTone]">
          {{ status?.message || 'Loading...' }}
        </span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <label class="space-y-2">
          <span class="text-xs text-naonur-ash">Client ID</span>
          <input v-model="clientId" class="naonur-input" autocomplete="off" placeholder="1234567890-abc123.apps.googleusercontent.com" />
        </label>
        <label class="space-y-2">
          <span class="text-xs text-naonur-ash">Client Secret</span>
          <input v-model="clientSecret" class="naonur-input" autocomplete="off" type="password" placeholder="GOCSPX-..." />
        </label>
      </div>

      <div class="dropzone" :class="{ 'drag-over': dragOver }" @dragover.prevent="dragOver = true" @dragleave.prevent="dragOver = false" @drop.prevent="onDrop">
        <div class="text-sm text-naonur-ash">Drop <code>client_secret.json</code> here, upload, or paste JSON below.</div>
        <div class="mt-2 flex items-center gap-3">
          <input type="file" accept="application/json" @change="onFileInput" />
        </div>
        <textarea
          v-model="jsonInput"
          class="naonur-input mt-3 min-h-[92px] font-mono text-xs"
          placeholder='Paste JSON: { "installed": { "client_id": "...", "client_secret": "..." } }'
        />
        <button class="btn btn-ghost text-xs mt-2" @click="applyJsonText">Parse JSON</button>
      </div>

      <div class="flex items-center gap-3">
        <button class="btn btn-primary" :disabled="saving" @click="saveCredentials">
          {{ saving ? 'Saving...' : 'Save' }}
        </button>
        <button class="btn btn-secondary" :disabled="testing" @click="testConnection">
          {{ testing ? 'Testing...' : 'Test Connection' }}
        </button>
        <span
          v-if="feedback"
          :class="[
            'text-sm transition-opacity duration-75',
            feedback.type === 'success' ? 'text-naonur-moss' : 'text-naonur-rust'
          ]"
        >
          {{ feedback.text }}
        </span>
      </div>
    </section>

    <section class="naonur-card card-contain p-0 overflow-hidden">
      <button class="w-full px-5 py-4 flex items-center justify-between text-left hover:bg-naonur-fog/10 transition-colors duration-75" @click="toggleInstructions">
        <span class="font-display text-naonur-bone">Need help setting up Google Workspace?</span>
        <span :class="['chevron', { open: instructionsOpen }]">⌄</span>
      </button>

      <div :class="['collapsible-grid', { open: instructionsOpen }]" aria-live="polite">
        <div class="collapsible-inner px-5 pb-5 text-sm text-naonur-ash">
          <ol class="list-decimal list-inside space-y-2">
            <li><a class="naonur-link" href="https://console.cloud.google.com/" target="_blank">Go to Google Cloud Console</a></li>
            <li><a class="naonur-link" href="https://console.cloud.google.com/projectselector2/home/dashboard" target="_blank">Create or select a project</a></li>
            <li><a class="naonur-link" href="https://console.cloud.google.com/apis/library/gmail.googleapis.com" target="_blank">Enable Gmail API</a> and <a class="naonur-link" href="https://console.cloud.google.com/apis/library/calendar-json.googleapis.com" target="_blank">Calendar API</a></li>
            <li><a class="naonur-link" href="https://console.cloud.google.com/apis/credentials/consent" target="_blank">Configure OAuth consent screen</a> (External, add test users)</li>
            <li><a class="naonur-link" href="https://console.cloud.google.com/apis/credentials" target="_blank">Create OAuth 2.0 Client ID</a> as <strong>Desktop application</strong></li>
            <li>Download <code>client_secret.json</code> or copy Client ID + Client Secret</li>
            <li>Paste into Tairseach and click <strong>Save</strong></li>
          </ol>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.card-contain {
  contain: content;
}

.naonur-input {
  @apply w-full rounded-lg border border-naonur-fog/40 bg-naonur-void/50 px-3 py-2 text-naonur-bone focus:outline-none focus:ring-1 focus:ring-naonur-gold;
}

.dropzone {
  border: 1px dashed rgba(145, 145, 145, 0.45);
  border-radius: 0.75rem;
  padding: 0.9rem;
  transition: border-color 45ms linear, background-color 45ms linear;
}

.dropzone.drag-over {
  border-color: rgba(201, 162, 39, 0.8);
  background: rgba(201, 162, 39, 0.06);
}

.chevron {
  transition: transform 45ms linear;
}

.chevron.open {
  transform: rotate(180deg);
}

.collapsible-grid {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 45ms linear;
}

.collapsible-grid.open {
  grid-template-rows: 1fr;
}

.collapsible-inner {
  min-height: 0;
  overflow: hidden;
}

.naonur-link {
  @apply text-naonur-gold hover:underline;
}

.btn-primary {
  @apply bg-naonur-gold text-naonur-void hover:bg-naonur-gold/80;
}

.btn-secondary {
  @apply border border-naonur-fog/50 text-naonur-bone hover:bg-naonur-fog/20;
}

.btn-ghost {
  @apply text-naonur-ash hover:text-naonur-bone hover:bg-naonur-fog/20;
}
</style>
