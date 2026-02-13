import { onScopeDispose } from 'vue'

interface RegisterOptions {
  id: string
  intervalMs: number
  onTick: () => void | Promise<void>
}

type TickMessage = {
  type: 'tick'
  id: string
}

export function useUnifiedPoller() {
  const worker = new Worker(new URL('../workers/unified-poller.worker.ts', import.meta.url), {
    type: 'module',
  })

  const handlers = new Map<string, () => void | Promise<void>>()

  worker.onmessage = (event: MessageEvent<TickMessage>) => {
    const msg = event.data
    if (msg.type !== 'tick') return

    const handler = handlers.get(msg.id)
    if (!handler) return

    void handler()
  }

  function register({ id, intervalMs, onTick }: RegisterOptions) {
    handlers.set(id, onTick)
    worker.postMessage({ type: 'register', id, intervalMs })
  }

  function unregister(id: string) {
    handlers.delete(id)
    worker.postMessage({ type: 'unregister', id })
  }

  function dispose() {
    for (const id of handlers.keys()) {
      worker.postMessage({ type: 'unregister', id })
    }
    handlers.clear()
    worker.terminate()
  }

  onScopeDispose(dispose)

  return {
    register,
    unregister,
    dispose,
  }
}
