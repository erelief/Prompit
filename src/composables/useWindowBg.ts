// Shared window background (theme-tracking, opacity-aware) + URL-domain helpers.
//
// Historically this was a "glass" surface with a backdrop-filter blur. The blur
// was dead code: it only does something when the window behind is visible
// through transparency, and in practice the window renders opaque at the
// default opacity. Keeping it created a composited layer that tore during
// animated window resize (the growing region showed a stale/blurred frame).
// What remains is the real, used behaviour: a gradient background whose alpha
// tracks `floating_opacity` so Ctrl+scroll can adjust window transparency.

import { computed } from "vue";
import { appConfig } from "../stores/config";

/** Read a CSS custom property from :root, with a fallback. */
function cssVar(name: string, fallback: string): string {
  if (typeof document === "undefined") return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

/** Parse a hex colour (#RRGGBB) into RGB components. */
function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const clean = hex.replace("#", "");
  if (clean.length !== 6) return null;
  const n = parseInt(clean, 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

/** Darken an RGB triple by a fixed offset for the gradient end-stop. */
function darken(r: number, g: number, b: number, dr = 8, dg = 8, db = 4) {
  return { r: Math.max(0, r - dr), g: Math.max(0, g - dg), b: Math.max(0, b - db) };
}

const LIGHT_BG = "#F8F7F4";

/** Window background gradient that tracks floating_opacity and the theme. */
export function useWindowBg() {
  return computed(() => {
    const o = (appConfig.floating_opacity ?? 90) / 100;
    const bg = cssVar("--color-bg", LIGHT_BG);
    const rgb = hexToRgb(bg) ?? hexToRgb(LIGHT_BG)!;
    const end = darken(rgb.r, rgb.g, rgb.b);
    if (o >= 1) {
      return `linear-gradient(135deg, rgb(${rgb.r},${rgb.g},${rgb.b}) 0%, rgb(${end.r},${end.g},${end.b}) 100%)`;
    }
    return `linear-gradient(135deg, rgba(${rgb.r},${rgb.g},${rgb.b},${o}) 0%, rgba(${end.r},${end.g},${end.b},${o * 0.94}) 100%)`;
  });
}

/** Extract a display hostname from a URL, stripping the leading "www.".
 *  Falls back to the raw string on parse failure. */
export function domainOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

