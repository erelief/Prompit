import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useRoute } from "vue-router";
import mainWindowConfig from "../shared/main-window.json";
import { useAnimatedResize } from "./useAnimatedResize";
import { useWindowBg } from "./useWindowBg";

/** Width of the main (FloatingInput-rooted) panel; shared by all main-tier views. */
export const MAIN_WIDTH = mainWindowConfig.width;

const prefersReducedMotion = () =>
  typeof window !== "undefined" &&
  window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

/**
 * Cosmetic reveal wipe for the snap resize: a full-window overlay in the
 * view's own background (opaque `--color-bg` for settings-class pages, the
 * translucent gradient for the floating History panel) collapses away on the
 * compositor (`scaleY(1) → 0`, transform only — no relayout/raster per
 * frame). This replaces the old OS-window tween, whose per-frame SetWindowPos
 * outpaced the webview's ~20Hz raster and produced a staircase of unpainted
 * bands. Direction follows growAbove: normal pages reveal top→bottom,
 * grow-above pages bottom→top (their content stacks from the bottom edge).
 */
function playRevealWipe(floating: boolean, growAbove: boolean) {
  if (prefersReducedMotion()) return;
  const overlay = document.createElement("div");
  overlay.className = "vt-wipe-overlay";
  if (floating) {
    overlay.style.background = useWindowBg().value;
  }
  overlay.style.transformOrigin = growAbove ? "top" : "bottom";
  document.body.appendChild(overlay);
  const anim = overlay.animate(
    [{ transform: "scaleY(1)" }, { transform: "scaleY(0)" }],
    { duration: 200, easing: "cubic-bezier(0.25, 1, 0.5, 1)", fill: "forwards" },
  );
  anim.onfinish = () => overlay.remove();
}

/**
 * Shared setup for Settings-like sub-pages.
 * Handles window sizing, growAbove state, and window-config listener.
 * Call in <script setup> of any page that behaves like a Settings sub-page.
 *
 * The window resize on mount is a SNAP (one SetWindowPos to the final size)
 * plus a compositor-driven reveal wipe for motion: the opaque page background
 * fills the new viewport in a single raster, so there is no per-step fill lag
 * (the old animated tween stair-stepped because the OS window moved at 60Hz
 * while the webview could only repaint at ~20Hz).
 *
 * Returns { growAbove }.
 */
export function useSettingsWindow(height = 580, width = MAIN_WIDTH) {
  const growAbove = ref(false);
  let unlistenConfig: (() => void) | null = null;
  const { snapResize } = useAnimatedResize();
  const route = useRoute();

  onMounted(async () => {
    // Only wipe when the size actually changes (settings → settings sub-page
    // navigation keeps 580px and should not replay the reveal). Computed
    // BEFORE the pre-size, which makes the viewport report the target size.
    const sizeChanges = Math.abs(window.innerHeight - height) > 1;

    // Pre-size the webview (NOT the OS window) to the target, then wait one
    // paint: the compositor visual — filled with the DefaultBackgroundColor —
    // reaches full size while the window is still small, so the snap below
    // lands on an already-filled surface instead of a transparent gap.
    if (sizeChanges && !prefersReducedMotion()) {
      invoke("prepare_webview_size", { width, height });
      await nextTick(); // settings DOM committed
      await new Promise((r) =>
        requestAnimationFrame(() => requestAnimationFrame(r)),
      ); // ≥1 frame presented at the new bounds
    }

    snapResize(height, width);
    growAbove.value = await invoke<boolean>("get_grow_above");
    if (sizeChanges) {
      playRevealWipe(route.path === "/history", growAbove.value);
    }
    unlistenConfig = await listen<boolean>("window-config", (e) => {
      growAbove.value = e.payload;
    });
  });

  onUnmounted(() => {
    unlistenConfig?.();
  });

  return { growAbove };
}
