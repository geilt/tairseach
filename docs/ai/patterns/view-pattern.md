# View Pattern

> **Copy-paste template for adding a new Vue view**

---

## File Location

```
src/views/YourView.vue
```

Add route to `src/router/index.ts`:
```typescript
{
  path: '/your-path',
  name: 'YourView',
  component: () => import('../views/YourView.vue')
}
```

Add navigation link to sidebar (if needed).

---

## Template

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface YourDataType {
  id: string
  name: string
  // ... other fields
}

const loading = ref(false)
const data = ref<YourDataType[]>([])
const error = ref<string | null>(null)

async function fetchData() {
  loading.value = true
  error.value = null
  try {
    const result = await invoke<YourDataType[]>('your_command', {
      // parameters here
    })
    data.value = result
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  void fetchData()
})

// Computed properties for filtering/transforming data
const filteredData = computed(() => {
  // filter logic
  return data.value
})
</script>

<template>
  <section class="animate-fade-in">
    <!-- Header -->
    <div class="mb-6 flex items-center justify-between gap-4">
      <div>
        <h1 class="font-display text-2xl tracking-wider text-naonur-gold">Your View Title</h1>
        <p class="text-sm text-naonur-ash">Description of what this view shows</p>
      </div>

      <!-- Actions (optional) -->
      <div class="flex items-center gap-3">
        <button
          @click="fetchData"
          class="rounded-md bg-naonur-moss px-4 py-2 text-sm text-naonur-shadow hover:bg-naonur-moss/80"
        >
          Refresh
        </button>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="naonur-card p-8 text-center text-naonur-ash">
      Loading...
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="naonur-card border-red-500/50 p-6">
      <p class="text-red-400">{{ error }}</p>
      <button
        @click="fetchData"
        class="mt-4 rounded bg-naonur-fog px-4 py-2 text-sm text-naonur-bone"
      >
        Retry
      </button>
    </div>

    <!-- Empty State -->
    <div v-else-if="filteredData.length === 0" class="naonur-card p-8 text-center text-naonur-smoke">
      No data available.
    </div>

    <!-- Content -->
    <div v-else class="naonur-card">
      <div class="divide-y divide-naonur-fog">
        <div
          v-for="item in filteredData"
          :key="item.id"
          class="p-4 hover:bg-naonur-mist/40"
        >
          <h3 class="font-medium text-naonur-bone">{{ item.name }}</h3>
          <!-- item content -->
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Component-specific styles if needed */
</style>
```

---

## With Composable (Recommended for Complex State)

```vue
<script setup lang="ts">
import { useYourFeature } from '@/composables/useYourFeature'

const { data, loading, error, refresh } = useYourFeature()
</script>

<template>
  <section class="animate-fade-in">
    <div class="mb-6">
      <h1 class="font-display text-2xl tracking-wider text-naonur-gold">Your View</h1>
    </div>

    <div v-if="loading" class="naonur-card p-8 text-center text-naonur-ash">
      Loading...
    </div>

    <div v-else-if="error" class="naonur-card border-red-500/50 p-6">
      <p class="text-red-400">{{ error }}</p>
    </div>

    <div v-else class="naonur-card">
      <!-- content -->
    </div>
  </section>
</template>
```

---

## Naonur Design Tokens

### Colors
```css
/* Background layers */
bg-naonur-void       /* Deepest background */
bg-naonur-shadow     /* Primary background */
bg-naonur-mist       /* Elevated cards */

/* Text */
text-naonur-bone     /* Primary text */
text-naonur-ash      /* Secondary text */
text-naonur-smoke    /* Tertiary/muted text */

/* Accents */
text-naonur-gold     /* Headings, highlights */
bg-naonur-moss       /* Success, primary actions */
border-naonur-fog    /* Subtle borders */

/* States */
bg-red-500/20        /* Error backgrounds */
text-red-400         /* Error text */
```

### Typography
```css
font-display         /* Headers (Irish Grover) */
tracking-wider       /* Letter spacing for headers */
text-sm, text-base   /* Body text sizes */
```

### Components
```css
.naonur-card         /* Standard card container */
.animate-fade-in     /* Page transition */
```

---

## Checklist

- [ ] Create `src/views/YourView.vue`
- [ ] Add route in `src/router/index.ts`
- [ ] Use `<script setup>` with TypeScript
- [ ] Define proper TypeScript interfaces
- [ ] Use Naonur design tokens consistently
- [ ] Handle loading/error/empty states
- [ ] Add `animate-fade-in` to section wrapper
- [ ] Use semantic HTML (section, header, nav, etc.)
- [ ] Consider extracting complex state to a composable
- [ ] Test responsiveness
- [ ] Test with Tauri backend integration

---

## Common Patterns

### Polling Data
```typescript
import { onMounted, onUnmounted } from 'vue'

let timer: number | null = null

function startPolling() {
  timer = window.setInterval(() => {
    void fetchData()
  }, 2000)
}

function stopPolling() {
  if (timer !== null) {
    clearInterval(timer)
    timer = null
  }
}

onMounted(() => {
  void fetchData()
  startPolling()
})

onUnmounted(() => {
  stopPolling()
})
```

### Virtual Scrolling (Large Lists)
See `ActivityView.vue` for virtual scroll implementation with `overscan`, `visibleItems`, `topSpacer`.

### Debounced Input
```typescript
import { ref, watch } from 'vue'

const input = ref('')
const debounced = ref('')
let timer: number | null = null

watch(input, (value) => {
  if (timer) clearTimeout(timer)
  timer = window.setTimeout(() => {
    debounced.value = value
  }, 300)
})
```

---

## See Also

- [modules/frontend-views.md](../modules/frontend-views.md) — All views
- [modules/frontend-infra.md](../modules/frontend-infra.md) — Composables, stores, workers
- [utility-pattern.md](utility-pattern.md) — Add shared TypeScript utilities
