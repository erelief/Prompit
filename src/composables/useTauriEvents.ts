import { onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export function useShortcutTriggered(callback: () => void) {
  let unlisten: UnlistenFn | null = null;

  onMounted(async () => {
    unlisten = await listen("shortcut-triggered", () => {
      callback();
    });
  });

  onUnmounted(() => {
    unlisten?.();
  });
}
