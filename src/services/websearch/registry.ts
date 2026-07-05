// Preset metadata + routing. The actual request/response logic lives in
// per-preset modules (e.g. anysearch.ts); this registry is metadata-only.

import type { Component } from "vue";
import AnySearch from "../../components/icons/providers/AnySearch.vue";
import Brave from "../../components/icons/providers/Brave.vue";
import Tavily from "../../components/icons/providers/Tavily.vue";
import Exa from "../../components/icons/providers/Exa.vue";
import Firecrawl from "../../components/icons/providers/Firecrawl.vue";
import type { SearchFn } from "./types";
import { search as anysearchSearch } from "./anysearch";
import { search as braveSearch } from "./brave";
import { search as tavilySearch } from "./tavily";
import { search as exaSearch } from "./exa";
import { search as firecrawlSearch } from "./firecrawl";

export interface WebSearchPresetMeta {
  id: string;
  label: string;
  icon: Component;
  /** When false, the provider can be used without an API key (anonymous tier).
   *  The user still opts in by adding it explicitly — there is no built-in
   *  fallback anymore. */
  keyRequired: boolean;
  /** i18n key for an explanatory hint, or empty when none is needed.
   *  Only presets with special key semantics (e.g. anonymous tiers) carry one. */
  keyHelpKey?: string;
  /** URL to the provider's API-key dashboard, shown as a "Get your API key" link. */
  apiUrl?: string;
}

export const WEB_SEARCH_PRESETS: WebSearchPresetMeta[] = [
  {
    id: "anysearch",
    label: "AnySearch",
    icon: AnySearch,
    // Anonymous tier available — a key is optional (higher quota with one).
    keyRequired: false,
    keyHelpKey: "settings.anonymousKeyHint",
    apiUrl: "https://www.anysearch.com/console/api-keys",
  },
  {
    id: "brave",
    label: "Brave Search",
    icon: Brave,
    keyRequired: true,
    apiUrl: "https://brave.com/search/api/",
  },
  {
    id: "exa",
    label: "Exa",
    icon: Exa,
    keyRequired: true,
    apiUrl: "https://dashboard.exa.ai/api-keys",
  },
  {
    id: "firecrawl",
    label: "Firecrawl",
    icon: Firecrawl,
    keyRequired: false,
    keyHelpKey: "settings.anonymousKeyHint",
    apiUrl: "https://www.firecrawl.dev/app",
  },
  {
    id: "tavily",
    label: "Tavily",
    icon: Tavily,
    keyRequired: true,
    apiUrl: "https://app.tavily.com/",
  },
];

/** preset id → search implementation */
const REGISTRY: Record<string, SearchFn> = {
  anysearch: anysearchSearch,
  brave: braveSearch,
  tavily: tavilySearch,
  exa: exaSearch,
  firecrawl: firecrawlSearch,
};

/** Resolve the search implementation for a preset id. */
export function getSearchFn(presetId: string): SearchFn {
  const fn = REGISTRY[presetId];
  if (!fn) {
    throw new Error(`Unknown search preset: ${presetId}`);
  }
  return fn;
}

export function presetMeta(presetId: string): WebSearchPresetMeta {
  const meta = WEB_SEARCH_PRESETS.find((p) => p.id === presetId);
  if (!meta) {
    throw new Error(`Unknown search preset: ${presetId}`);
  }
  return meta;
}
