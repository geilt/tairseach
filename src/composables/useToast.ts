import { onScopeDispose, ref, readonly } from 'vue'

export interface Toast {
  id: string
  message: string
  variant: 'success' | 'warning' | 'error' | 'info'
  duration: number
}

const toasts = ref<Toast[]>([])
let toastId = 0
const toastTimers = new Map<string, ReturnType<typeof setTimeout>>()

function clearToastTimer(id: string) {
  const timer = toastTimers.get(id)
  if (!timer) return
  clearTimeout(timer)
  toastTimers.delete(id)
}

function addToast(
  message: string,
  variant: Toast['variant'] = 'info',
  duration = 5000
) {
  const id = `toast-${++toastId}`

  toasts.value.push({ id, message, variant, duration })

  // Auto-dismiss
  if (duration > 0) {
    const timer = setTimeout(() => removeToast(id), duration)
    toastTimers.set(id, timer)
  }

  // Limit stack
  if (toasts.value.length > 5) {
    const removed = toasts.value.shift()
    if (removed) clearToastTimer(removed.id)
  }

  return id
}

function removeToast(id: string) {
  clearToastTimer(id)
  const index = toasts.value.findIndex(t => t.id === id)
  if (index !== -1) {
    toasts.value.splice(index, 1)
  }
}

export function useToast() {
  onScopeDispose(() => {
    for (const timer of toastTimers.values()) {
      clearTimeout(timer)
    }
    toastTimers.clear()
  })

  return {
    toasts: readonly(toasts),
    success: (message: string, duration?: number) => addToast(message, 'success', duration),
    warning: (message: string, duration?: number) => addToast(message, 'warning', duration),
    error: (message: string, duration?: number) => addToast(message, 'error', duration),
    info: (message: string, duration?: number) => addToast(message, 'info', duration),
    remove: removeToast,
  }
}
