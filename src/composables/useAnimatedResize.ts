import { invoke } from "@tauri-apps/api/core";
import mainWindowConfig from "../shared/main-window.json";

// NOTE: imported from the JSON directly (NOT from useSettingsWindow) to avoid a
// circular import: useSettingsWindow → useAnimatedResize → useSettingsWindow.
// Reading the live `const` mid-cycle would hit the TDZ and crash the app on boot.
/** Width of the main panel; kept in sync with useSettingsWindow's MAIN_WIDTH. */
const MAIN_WIDTH = mainWindowConfig.width;

/**
 * Animated window resize: eases the OS window into its new size instead of the
 * raw Win32 `SetWindowPos` / NSWindow `setFrame` snap.
 *
 * Two duration tiers:
 *  - View transitions (FloatingInput ↔ Settings/History): ~160ms. Short enough
 *    to read as a crisp ease rather than a noticeable animation, long enough to
 *    avoid the one-frame compositor tear that a bare SetWindowPos produces.
 *  - Micro (small continuous resizes, e.g. typing): ~120ms.
 *
 * Result-appearance growth is handled by `snapResize`, NOT animated: the result
 * text arrives atomically and the content reflow races the window resize, so a
 * tween there produces a visible layering glitch (window edge grows before the
 * webview repaints). Snapping eliminates the mismatch.
 *
 * State is held at MODULE scope because the whole app shares a single OS
 * window — when a view unmounts and another mounts, the new view must retarget
 * from wherever the window *actually is*, not from a per-instance zero.
 */

/** Subtle easing: fast start, gentle settle. */
function easeOutQuart(t: number): number {
  return 1 - Math.pow(1 - t, 4);
}

/** Duration for view transitions (120→580px class changes). */
const DEFAULT_DURATION = 160; // ms
/** Short variant: tiny continuous resizes (typing). */
const MICRO_DURATION = 120; // ms
/** Threshold below which we use the micro duration. */
const MICRO_DELTA = 120; // px

// ── Module-level shared state (one OS window → one set of values) ──
let currentH = 0;
let currentW = MAIN_WIDTH;
let rafId: number | null = null;
let state: {
  targetH: number;
  targetW: number;
  startH: number;
  startW: number;
  startTime: number;
  duration: number;
  cancelled: boolean;
} | null = null;

const prefersReducedMotion = () =>
  typeof window !== "undefined" &&
  window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

function frame(now: number) {
  if (!state || state.cancelled) {
    state = null;
    rafId = null;
    return;
  }

  const elapsed = now - state.startTime;
  const t = elapsed >= state.duration ? 1 : elapsed / state.duration;
  const eased = easeOutQuart(t);

  const nextH = state.startH + (state.targetH - state.startH) * eased;
  const widthChanges = state.targetW !== state.startW;
  const nextW = widthChanges
    ? state.startW + (state.targetW - state.startW) * eased
    : state.targetW;

  invoke("resize_and_reposition", {
    height: nextH,
    width: nextW,
  });
  currentH = nextH;
  if (widthChanges) currentW = nextW;

  if (t < 1) {
    rafId = requestAnimationFrame(frame);
  } else {
    // Guarantee we land exactly on target (avoid float drift on the last frame).
    invoke("resize_and_reposition", { height: state.targetH, width: state.targetW });
    currentH = state.targetH;
    currentW = state.targetW;
    state = null;
    rafId = null;
  }
}

/**
 * Drives a smooth, GPU/OS-cheap resize by sending interpolated height/width to
 * the `resize_and_reposition` Tauri command once per animation frame.
 *
 * Design notes:
 *  - The backend native call is cheap (a single Win32 `SetWindowPos` / NSWindow
 *    `setFrame`), so ~10 calls across 160ms is well within budget.
 *  - New targets cancel and *retarget* any in-flight animation: the new tween
 *    starts from wherever the window currently is, so rapid content changes
 *    still feel responsive — no overshoot.
 *  - We skip the animation entirely for `prefers-reduced-motion` users.
 *
 * Returns two functions mirroring the `resize_and_reposition` IPC signature.
 */
export function useAnimatedResize() {
  /**
   * Animate the window to `height` × `width` (logical, CSS px).
   * Pass `width: undefined` to keep the current width (matches the backend's
   * `Option<f64>` signature, which defaults to 500 when absent).
   *
   * Use for view transitions and small continuous resizes. Do NOT use for
   * result-appearance growth — call `snapResize` for that (see class doc).
   */
  function animateResize(height: number, width?: number) {
    const targetW = width ?? currentW;

    // Respect reduced-motion: jump immediately, no tween.
    if (prefersReducedMotion()) {
      return snapResize(height, width);
    }

    // If a tween is in flight, cancel it and start fresh from current values.
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      if (state) state.cancelled = true;
      rafId = null;
    }

    // Resolve the START size from the live webview geometry, not from a stale
    // cache. Backend commands (StartupReminder / Onboarding / tray show) resize
    // the OS window directly, bypassing this module, so currentH/W can be out of
    // sync. window.innerHeight/Width is synchronous, zero-IPC, and in the same
    // logical-CSS-px coordinate system the backend height/width uses.
    // Fallback to the cache only mid-tween (retarget), where the webview is
    // already mid-transition and reading it would catch a stale frame.
    const inFlightTween = state !== null;
    const startH = inFlightTween ? (currentH || height) : (window.innerHeight || currentH || height);
    const startW = inFlightTween ? (currentW || targetW) : (window.innerWidth || currentW || targetW);

    // No-op if we're already at the target (avoids a needless IPC burst —
    // important since ResizeObserver can fire frequently while typing).
    const dh = Math.abs(height - startH);
    const dw = Math.abs(targetW - startW);
    if (dh < 0.5 && dw < 0.5) {
      currentH = height;
      currentW = targetW;
      return;
    }

    const duration = Math.max(dh, dw) <= MICRO_DELTA ? MICRO_DURATION : DEFAULT_DURATION;

    state = {
      targetH: height,
      targetW,
      startH,
      startW,
      startTime: performance.now(),
      duration,
      cancelled: false,
    };
    rafId = requestAnimationFrame(frame);
  }

  /**
   * Jump to `height` × `width` instantly, bypassing the animation. Use for:
   *  - Result-appearance growth (content arrives atomically; a tween here tears).
   *  - Force-resize on mount/wake (cold-start geometry, no animation wanted).
   */
  function snapResize(height: number, width?: number) {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      if (state) state.cancelled = true;
      rafId = null;
      state = null;
    }
    const targetW = width ?? currentW;
    currentH = height;
    currentW = targetW;
    invoke("resize_and_reposition", { height, width });
  }

  return { animateResize, snapResize };
}
