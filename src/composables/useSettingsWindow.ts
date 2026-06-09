import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * Shared setup for Settings-like sub-pages.
 * Handles window sizing, growAbove state, and window-config listener.
 * Call in <script setup> of any page that behaves like a Settings sub-page.
 */
export function useSettingsWindow(height = 580, width = 480) {
  const growAbove = ref(false);
  let unlistenConfig: (() => void) | null = null;

  onMounted(async () => {
    await invoke("resize_and_reposition", { height, width });
    growAbove.value = await invoke<boolean>("get_grow_above");
    unlistenConfig = await listen<boolean>("window-config", (e) => {
      growAbove.value = e.payload;
    });
  });

  onUnmounted(() => {
    unlistenConfig?.();
  });

  return { growAbove };
}
