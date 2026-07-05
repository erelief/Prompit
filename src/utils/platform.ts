// ── Platform detection helpers ──
//
// These mirror the Rust-side cfg!(target_os) checks so the frontend can
// display the correct modifier-key names without a Tauri command round-trip.

export function isMac(): boolean {
  return typeof navigator !== "undefined" && navigator.platform.startsWith("Mac");
}

/** Human-readable name of the Alt / Option modifier key. */
export function altKey(): string {
  return isMac() ? "Option" : "Alt";
}

/** Human-readable name of the Ctrl / Cmd modifier key. */
export function ctrlKey(): string {
  return isMac() ? "Cmd" : "Ctrl";
}