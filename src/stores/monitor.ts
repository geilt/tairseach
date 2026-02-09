import { defineStore } from "pinia";
import { ref } from "vue";

export interface ActivityEvent {
  id: string;
  timestamp: Date;
  type: "tool_call" | "permission_request" | "error" | "info";
  source: string;
  message: string;
  metadata?: Record<string, unknown>;
}

export const useMonitorStore = defineStore("monitor", () => {
  const events = ref<ActivityEvent[]>([]);
  const connected = ref(false);
  const paused = ref(false);

  function addEvent(event: ActivityEvent) {
    events.value.unshift(event);
    // Keep only last 1000 events
    if (events.value.length > 1000) {
      events.value = events.value.slice(0, 1000);
    }
  }

  function clearEvents() {
    events.value = [];
  }

  function togglePause() {
    paused.value = !paused.value;
  }

  async function connect() {
    // TODO: Connect to Tauri event stream
    connected.value = true;
  }

  async function disconnect() {
    // TODO: Disconnect from event stream
    connected.value = false;
  }

  return {
    events,
    connected,
    paused,
    addEvent,
    clearEvents,
    togglePause,
    connect,
    disconnect,
  };
});
