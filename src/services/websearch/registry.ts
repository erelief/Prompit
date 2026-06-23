// Preset metadata + routing. The actual request/response logic lives in
// per-preset modules (e.g. anysearch.ts); this registry is metadata-only.

import type { Component } from "vue";
import AnySearch from "../../components/icons/providers/AnySearch.vue";
import Brave from "../../components/icons/providers/Brave.vue";
import type { SearchFn } from "./types";
import { search as anysearchSearch } from "./anysearch";
import { search as braveSearch } from "./brave";

export interface SearchPresetMeta {
  id: string;
  label: string;
  icon: Component;
  supportsAnonymous: boolean;
  keyRequired: boolean;
  keyHelpKey: string; // i18n key explaining the key requirement
}

export const SEARCH_PRESETS: SearchPresetMeta[] = [
  {
    id: "anysearch",
    label: "AnySearch",
    icon: AnySearch,
    supportsAnonymous: true,
    keyRequired: true,
    keyHelpKey: "settings.webSearchKeyHint",
  },
  {
    id: "brave",
    label: "Brave Search",
    icon: Brave,
    supportsAnonymous: false,
    keyRequired: true,
    keyHelpKey: "settings.braveKeyHint",
  },
];

/** preset id → search implementation */
const REGISTRY: Record<string, SearchFn> = {
  anysearch: anysearchSearch,
  brave: braveSearch,
};

/** Built-in anonymous fallback. Always available so a usable engine exists. */
export const ANONYMOUS_FALLBACK = {
  preset: "anysearch",
  apiKey: undefined,
} as const;

/** Resolve the search implementation for a preset id. */
export function getSearchFn(presetId: string): SearchFn {
  const fn = REGISTRY[presetId];
  if (!fn) {
    throw new Error(`Unknown search preset: ${presetId}`);
  }
  return fn;
}

export function presetMeta(presetId: string): SearchPresetMeta {
  const meta = SEARCH_PRESETS.find((p) => p.id === presetId);
  if (!meta) {
    throw new Error(`Unknown search preset: ${presetId}`);
  }
  return meta;
}
