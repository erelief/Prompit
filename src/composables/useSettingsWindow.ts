import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import mainWindowConfig from "../shared/main-window.json";
import { useAnimatedResize } from "./useAnimatedResize";

/** Width of the main (FloatingInput-rooted) panel; shared by all main-tier views. */
export const MAIN_WIDTH = mainWindowConfig.width;

/**
 * Shared setup for Settings-like sub-pages.
 * Handles window sizing, growAbove state, and window-config listener.
 * Call in <script setup> of any page that behaves like a Settings sub-page.
 *
 * The window resize on mount is animated: switching from the compact
 * FloatingInput bar (~120px) to a sub-page (~580px) eases in over ~320ms
 * instead of snapping, matching the subtle motion language used elsewhere.
 *
 * Returns { growAbove }.
 */
export function useSettingsWindow(height = 580, width = MAIN_WIDTH) {
  const growAbove = ref(false);
  let unlistenConfig: (() => void) | null = null;
  const { animateResize } = useAnimatedResize();

  onMounted(async () => {
    animateResize(height, width);
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
