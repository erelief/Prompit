import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import mainWindowConfig from "../shared/main-window.json";

/** Width of the main (FloatingInput-rooted) panel; shared by all main-tier views. */
export const MAIN_WIDTH = mainWindowConfig.width;

/**
 * Shared setup for Settings-like sub-pages.
 * Handles window sizing, growAbove state, and window-config listener.
 * Call in <script setup> of any page that behaves like a Settings sub-page.
 *
 * Returns { growAbove, rootClass }.
 * Apply `:class="rootClass"` on the root element — when growAbove is true,
 * the class `grow-above` is added, which reverses the visual order so
 * the header/tabs anchor to the bottom and content grows upward.
 */
export function useSettingsWindow(height = 580, width = MAIN_WIDTH) {
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
