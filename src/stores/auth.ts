import { defineStore } from "pinia";
import { ref, computed } from "vue";

export interface AuthState {
  authenticated: boolean;
  method: "none" | "password" | "biometric" | "hardware_key";
  lastAuth?: Date;
}

export const useAuthStore = defineStore("auth", () => {
  const state = ref<AuthState>({
    authenticated: false,
    method: "none",
  });

  const isAuthenticated = computed(() => state.value.authenticated);

  async function authenticate(method: AuthState["method"], _credential?: string) {
    // TODO: Implement via Tauri command
    state.value.authenticated = true;
    state.value.method = method;
    state.value.lastAuth = new Date();
  }

  async function logout() {
    state.value.authenticated = false;
    state.value.method = "none";
    state.value.lastAuth = undefined;
  }

  async function checkAuth() {
    // TODO: Check auth state via Tauri
  }

  return {
    state,
    isAuthenticated,
    authenticate,
    logout,
    checkAuth,
  };
});
