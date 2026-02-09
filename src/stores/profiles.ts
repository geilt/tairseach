import { defineStore } from "pinia";
import { ref } from "vue";

export interface Profile {
  id: string;
  name: string;
  type: "agent" | "tool" | "mcp_server";
  config: Record<string, unknown>;
  enabled: boolean;
}

export const useProfilesStore = defineStore("profiles", () => {
  const profiles = ref<Profile[]>([]);
  const activeProfile = ref<string | null>(null);
  const loading = ref(false);

  async function loadProfiles() {
    loading.value = true;
    // TODO: Load from Tauri backend
    loading.value = false;
  }

  async function createProfile(_profile: Omit<Profile, "id">) {
    // TODO: Create via Tauri command
  }

  async function updateProfile(_id: string, _updates: Partial<Profile>) {
    // TODO: Update via Tauri command
  }

  async function deleteProfile(_id: string) {
    // TODO: Delete via Tauri command
  }

  async function setActiveProfile(id: string) {
    activeProfile.value = id;
    // TODO: Persist via Tauri command
  }

  return {
    profiles,
    activeProfile,
    loading,
    loadProfiles,
    createProfile,
    updateProfile,
    deleteProfile,
    setActiveProfile,
  };
});
