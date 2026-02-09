const KEY_PREFIX = 'tairseach_cache_'

export interface CachedState<T> {
  data: T
  lastUpdated: string
}

function storageAvailable() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined'
}

export function cacheKey(name: string) {
  return `${KEY_PREFIX}${name}`
}

export function loadStateCache<T>(name: string): CachedState<T> | null {
  if (!storageAvailable()) return null
  try {
    const raw = window.localStorage.getItem(cacheKey(name))
    if (!raw) return null
    return JSON.parse(raw) as CachedState<T>
  } catch (error) {
    console.warn(`[state-cache] failed to load ${name}`, error)
    return null
  }
}

export function saveStateCache<T>(name: string, data: T): CachedState<T> {
  const entry: CachedState<T> = {
    data,
    lastUpdated: new Date().toISOString(),
  }

  if (!storageAvailable()) return entry

  try {
    window.localStorage.setItem(cacheKey(name), JSON.stringify(entry))
  } catch (error) {
    console.warn(`[state-cache] failed to save ${name}`, error)
  }

  return entry
}

export function clearStateCache(name: string) {
  if (!storageAvailable()) return
  try {
    window.localStorage.removeItem(cacheKey(name))
  } catch {
    // no-op
  }
}
