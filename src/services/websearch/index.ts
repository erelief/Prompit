// Public API: resolve engine, search, format context for the LLM, classify errors.

import { appConfig } from "../../stores/config";
import { ANONYMOUS_FALLBACK, getSearchFn, presetMeta } from "./registry";
import type { SearchHit, ClassifiedSearchError } from "./types";
import { SearchHttpError } from "./types";

// Re-export for convenience
export { SEARCH_PRESETS, presetMeta, ANONYMOUS_FALLBACK, getSearchFn } from "./registry";
export type { SearchHit, SearchFn, ClassifiedSearchError } from "./types";

interface ResolvedEngine {
  preset: string;
  apiKey: string | undefined;
  isAnonymous: boolean;
}

/**
 * Resolve the engine to use. Priority: the active user instance if it's
 * enabled and has a key; otherwise the built-in anonymous fallback. A usable
 * engine always resolves.
 */
export function resolveActiveEngine(): ResolvedEngine {
  const idx = appConfig.web_search_active_index;
  const engines = appConfig.web_engines;
  if (idx >= 0 && idx < engines.length) {
    const engine = engines[idx];
    if (engine.enabled) {
      const meta = presetMeta(engine.preset);
      if (engine.api_key) {
        return { preset: engine.preset, apiKey: engine.api_key, isAnonymous: false };
      }
      if (!meta.keyRequired) {
        return { preset: engine.preset, apiKey: undefined, isAnonymous: true };
      }
    }
  }
  return { preset: ANONYMOUS_FALLBACK.preset, apiKey: undefined, isAnonymous: true };
}

/** Run a search using the resolved engine. Throws on failure (see classifySearchError). */
export async function webSearch(
  query: string,
  signal?: AbortSignal,
  maxResults?: number
): Promise<SearchHit[]> {
  const engine = resolveActiveEngine();
  const fn = getSearchFn(engine.preset);
  return fn(query, { apiKey: engine.apiKey, signal, maxResults });
}

const MAX_HITS = 5;
const MAX_CONTENT_CHARS = 500;
const MAX_TOTAL_CHARS = 4000;

/**
 * Format search hits into a context block for the LLM. Truncates each hit's
 * content to ~500 chars, caps at 5 hits, caps total at ~4000 chars.
 */
export function formatSearchContext(hits: SearchHit[]): string {
  const limited = hits.slice(0, MAX_HITS);
  const lines: string[] = [
    "[Web Search Context]",
    "The following are recent web results for the user's input. Use them to inform your response.",
    "",
  ];
  let total = lines.join("\n").length;
  for (let i = 0; i < limited.length; i++) {
    const h = limited[i];
    const content = h.content
      ? h.content.length > MAX_CONTENT_CHARS
        ? h.content.slice(0, MAX_CONTENT_CHARS) + "…"
        : h.content
      : "";
    const block =
      `${i + 1}. ${h.title}\n` +
      `   ${h.url}\n` +
      `   ${h.snippet}` +
      (content ? `\n   ${content}` : "");
    if (total + block.length > MAX_TOTAL_CHARS) break;
    lines.push(block, "");
    total += block.length + 1;
  }
  return lines.join("\n");
}

/**
 * Classify a search error into a code + raw message from the server.
 */
export function classifySearchError(e: unknown): ClassifiedSearchError {
  if (e instanceof SearchHttpError) {
    return { code: String(e.status), rawMessage: extractErrorMessage(e.message) || undefined };
  }
  if (e instanceof Error && /network|fetch|Failed to fetch/i.test(e.message)) {
    return { code: "NETWORK" };
  }
  return { code: "UNKNOWN" };
}

/** Try to extract a human-readable error string from an HTTP response body. */
function extractErrorMessage(raw: string): string | undefined {
  const text = raw?.trim();
  if (!text) return undefined;
  try {
    const obj = JSON.parse(text);
    return obj.error || obj.message || undefined;
  } catch {
    return text;
  }
}

/**
 * Test a web engine's connection. Used by Settings. Runs a trivial query.
 * Returns { ok, status?, error? } mirroring testProviderConnection's shape.
 */
export async function testWebEngine(
  preset: string,
  apiKey: string
): Promise<{ ok: boolean; status?: number; error?: string }> {
  if (!apiKey) {
    return { ok: false, error: "Missing API key" };
  }
  try {
    const fn = getSearchFn(preset);
    await fn("connection test", { apiKey });
    return { ok: true };
  } catch (e) {
    if (e instanceof SearchHttpError) {
      return { ok: false, status: e.status, error: `HTTP ${e.status}` };
    }
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}
