// ── Shortcut string helpers ──
//
// Shortcut bindings are stored as a `+`-joined token string, e.g.
// "Ctrl+Shift+P" or "Alt+M", matching the format consumed by Tauri's
// global-shortcut plugin. These helpers convert between raw keyboard
// events, the canonical string form, and display tokens.

/**
 * Map a `KeyboardEvent.code` to the displayable token used in a binding
 * string, e.g. `KeyY` → `"Y"`, `Digit1` → `"1"`, `F5` → `"F5"`.
 * Returns `null` for codes that cannot form a binding (e.g. modifier-only
 * or unmapped keys).
 */
export function keyCodeToToken(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3); // KeyA → A
  if (/^Digit[0-9]$/.test(code)) return code.slice(5); // Digit1 → 1
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code; // F1–F24
  const named: Record<string, string> = {
    Space: "Space", Enter: "Enter", Tab: "Tab", Escape: "Escape", Backspace: "Backspace",
    Insert: "Insert", Delete: "Delete", Home: "Home", End: "End",
    PageUp: "PageUp", PageDown: "PageDown",
    ArrowLeft: "Left", ArrowRight: "Right", ArrowUp: "Up", ArrowDown: "Down",
  };
  return named[code] ?? null;
}

/**
 * Split a binding string into display tokens:
 * `"Ctrl+Shift+P"` → `["Ctrl", "Shift", "P"]`.
 */
export function parseShortcutTokens(s: string): string[] {
  return s
    .split("+")
    .map((tok) => tok.trim())
    .filter(Boolean);
}

/** Modifier display names in a stable canonical order. */
const MOD_ORDER = ["Ctrl", "Alt", "Shift", "Cmd"] as const;

/** Token used for the meta (⌘) modifier. */
const META_TOKEN = "Cmd";

/**
 * Returns `true` when the given keyboard event matches the binding string.
 * Matches the main key token and requires the modifier set to match
 * exactly (no extra/missing modifiers).
 */
export function eventMatchesShortcut(e: KeyboardEvent, shortcut: string): boolean {
  const tokens = parseShortcutTokens(shortcut);
  if (tokens.length === 0) return false;

  const modTokens = MOD_ORDER.filter((m) => tokens.includes(m));
  const keyToken = tokens[tokens.length - 1];

  // Main key must match what the event would produce as a token.
  const eventToken = keyCodeToToken(e.code);
  if (!eventToken || eventToken !== keyToken) return false;

  // Exact modifier-set match (prevents Alt+M from firing on Ctrl+Alt+M, etc.).
  if (modTokens.includes("Ctrl") !== e.ctrlKey) return false;
  if (modTokens.includes("Alt") !== e.altKey) return false;
  if (modTokens.includes("Shift") !== e.shiftKey) return false;
  if (modTokens.includes(META_TOKEN) !== e.metaKey) return false;

  return true;
}

/**
 * Build a canonical binding string from a keyboard event, using the same
 * modifier order as display tokens. Returns `null` if the event lacks a
 * usable main key or has no modifier.
 */
export function buildShortcutFromEvent(e: KeyboardEvent): string | null {
  const token = keyCodeToToken(e.code);
  if (!token) return null;
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push(META_TOKEN);
  if (mods.length === 0) return null;
  return [...mods, token].join("+");
}

/**
 * Compare two binding strings for equality, ignoring modifier order and
 * case: `"Ctrl+Alt+M"` === `"Alt+ctrl+m"`. Used for cross-shortcut conflict
 * detection (e.g. wake vs. mode-switch must not collide).
 */
export function shortcutsEqual(a: string, b: string): boolean {
  const norm = (s: string): string =>
    s.split("+").map((tok) => tok.trim().toLowerCase()).filter(Boolean).sort().join("+");
  return norm(a) === norm(b);
}
