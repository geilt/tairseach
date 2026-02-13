type RegisterMessage = {
  type: 'register'
  id: string
  intervalMs: number
}

type UnregisterMessage = {
  type: 'unregister'
  id: string
}

type IncomingMessage = RegisterMessage | UnregisterMessage

type TickMessage = {
  type: 'tick'
  id: string
}

const timers = new Map<string, ReturnType<typeof setInterval>>()

function register(id: string, intervalMs: number) {
  const existing = timers.get(id)
  if (existing) {
    clearInterval(existing)
  }

  const handle = setInterval(() => {
    const msg: TickMessage = { type: 'tick', id }
    self.postMessage(msg)
  }, intervalMs)

  timers.set(id, handle)
}

function unregister(id: string) {
  const handle = timers.get(id)
  if (!handle) return
  clearInterval(handle)
  timers.delete(id)
}

self.onmessage = (event: MessageEvent<IncomingMessage>) => {
  const msg = event.data

  if (msg.type === 'register') {
    register(msg.id, msg.intervalMs)
    return
  }

  unregister(msg.id)
}
