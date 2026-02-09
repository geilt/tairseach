import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type PermissionStatus = 'granted' | 'denied' | 'not_determined' | 'restricted' | 'unknown';

export interface Permission {
  id: string;
  name: string;
  description: string;
  status: PermissionStatus;
  critical: boolean;
  last_checked?: string;
}

export interface PermissionDefinition {
  id: string;
  name: string;
  description: string;
  critical: boolean;
  icon: string;
  system_pref_pane: string;
}

// Map backend status to display status
function mapStatus(status: string): PermissionStatus {
  const map: Record<string, PermissionStatus> = {
    'granted': 'granted',
    'denied': 'denied',
    'not_determined': 'not_determined',
    'restricted': 'restricted',
  };
  return map[status] || 'unknown';
}

export const usePermissionsStore = defineStore("permissions", () => {
  const permissions = ref<Permission[]>([]);
  const definitions = ref<PermissionDefinition[]>([]);
  // "loading" should only be used for initial / user-triggered loads that need a visible indicator.
  // Background refreshes should not flip this flag (prevents layout thrash in the view).
  const loading = ref(false);
  const error = ref<string | null>(null);

  const criticalPermissions = computed(() => 
    permissions.value.filter(p => p.critical)
  );

  const criticalGranted = computed(() =>
    criticalPermissions.value.filter(p => p.status === 'granted').length
  );

  async function loadDefinitions() {
    try {
      definitions.value = await invoke<PermissionDefinition[]>("get_permission_definitions");
    } catch (e) {
      console.error("Failed to load permission definitions:", e);
    }
  }

  function upsertPermission(next: Permission) {
    const idx = permissions.value.findIndex(p => p.id === next.id);
    if (idx >= 0) {
      const prev = permissions.value[idx];
      permissions.value[idx] = {
        ...prev,
        ...next,
      };
    } else {
      permissions.value.push(next);
    }
  }

  async function loadPermissions(opts: { silent?: boolean } = {}) {
    const silent = opts.silent === true;
    const isInitial = permissions.value.length === 0;

    if (!silent || isInitial) {
      loading.value = true;
    }
    error.value = null;

    try {
      const result = await invoke<Permission[]>("check_all_permissions");
      const mapped = result.map(p => ({
        ...p,
        status: mapStatus(p.status as string),
      }));

      // Replace in one shot to keep ordering stable.
      permissions.value = mapped;
    } catch (e) {
      error.value = String(e);
      console.error("Failed to load permissions:", e);
    } finally {
      if (!silent || isInitial) {
        loading.value = false;
      }
    }
  }

  async function checkPermission(id: string): Promise<Permission | null> {
    try {
      const result = await invoke<Permission>("check_permission", { permissionId: id });
      return {
        ...result,
        status: mapStatus(result.status as string),
      };
    } catch (e) {
      console.error(`Failed to check permission ${id}:`, e);
      return null;
    }
  }

  async function refreshPermission(id: string): Promise<Permission | null> {
    const next = await checkPermission(id);
    if (next) upsertPermission(next);
    return next;
  }

  async function requestPermission(id: string) {
    try {
      const before = permissions.value.find(p => p.id === id)?.status;

      await invoke("request_permission", { permissionId: id });

      // Poll for ~30s (every 2s) to catch the user returning from System Settings.
      // Use focused checks to avoid thrashing the whole UI.
      const startedAt = Date.now();
      const maxMs = 30_000;
      const intervalMs = 2_000;

      while (Date.now() - startedAt < maxMs) {
        // Small delay before first check to give the OS time to update state.
        // eslint-disable-next-line no-await-in-loop
        await new Promise(r => setTimeout(r, intervalMs));
        // eslint-disable-next-line no-await-in-loop
        const next = await refreshPermission(id);
        if (!next) continue;

        if (before && next.status !== before) break;
        // Also stop early if we reached a terminal state.
        if (next.status === 'granted' || next.status === 'denied') break;
      }
    } catch (e) {
      console.error(`Failed to request permission ${id}:`, e);
      error.value = String(e);
    }
  }

  async function openSettings(pane: string) {
    try {
      await invoke("open_permission_settings", { pane });
    } catch (e) {
      console.error("Failed to open settings:", e);
      error.value = String(e);
    }
  }

  function getIcon(id: string): string {
    const def = definitions.value.find(d => d.id === id);
    return def?.icon || '❓';
  }

  // Map permission ID to icon image path using Vite's asset resolution
  function getIconPath(id: string): string | null {
    const iconMap: Record<string, string> = {
      'contacts': 'permission-contacts.png',
      'calendar': 'permission-calendar.png',
      'automation': 'permission-automation.png',
      'full_disk_access': 'permission-disk.png',
      'accessibility': 'permission-accessibility.png',
      'screen_recording': 'permission-screen.png',
      'reminders': 'permission-reminders.png',
      'photos': 'permission-photos.png',
      'camera': 'permission-camera.png',
      'microphone': 'permission-microphone.png',
      'location': 'permission-location.png',
    };
    
    const filename = iconMap[id];
    if (!filename) return null;
    
    // Use Vite's asset resolution
    try {
      return new URL(`../assets/icons/${filename}`, import.meta.url).href;
    } catch {
      return null;
    }
  }

  return {
    permissions,
    definitions,
    loading,
    error,
    criticalPermissions,
    criticalGranted,
    loadDefinitions,
    loadPermissions,
    checkPermission,
    refreshPermission,
    requestPermission,
    openSettings,
    getIcon,
    getIconPath,
  };
});
