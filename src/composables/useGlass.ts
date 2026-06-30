// Shared glass-surface background + URL-domain helpers used by the floating
// input and history panel. Both previously inlined byte-identical copies.

import { computed } from "vue";
import { appConfig } from "../stores/config";
import { isDark } from "./useTheme";

/** Glass background gradient that tracks floating_opacity and the theme. */
export function useGlassBg() {
  return computed(() => {
    const o = (appConfig.floating_opacity ?? 90) / 100;
    if (o >= 1) {
      return isDark()
        ? "linear-gradient(135deg, rgb(15,15,20) 0%, rgb(20,20,30) 100%)"
        : "linear-gradient(135deg, rgb(255,255,255) 0%, rgb(245,245,250) 100%)";
    }
    return isDark()
      ? `linear-gradient(135deg, rgba(15,15,20,${o}) 0%, rgba(20,20,30,${o * 0.94}) 100%)`
      : `linear-gradient(135deg, rgba(255,255,255,${o}) 0%, rgba(245,245,250,${o * 0.94}) 100%)`;
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
