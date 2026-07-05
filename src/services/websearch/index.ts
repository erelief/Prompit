// Public API: resolve provider, search, format context for the LLM, classify errors.

import { appConfig } from "../../stores/config";
import { getSearchFn, presetMeta } from "./registry";
import type { SearchHit, ClassifiedSearchError } from "./types";
import { SearchHttpError } from "./types";

// Re-export for convenience (getSearchFn stays internal — no consumer outside
// this module imports it).
export { WEB_SEARCH_PRESETS, presetMeta } from "./registry";
export type { SearchHit, SearchFn, ClassifiedSearchError } from "./types";

interface ResolvedProvider {
  preset: string;
  apiKey: string | undefined;
}

/**
 * Resolve the active web search provider. Returns the enabled, active provider
 * when one is usable (has a key, or is a keyless preset); otherwise null. There
 * is no built-in fallback — callers must handle null (e.g. block the search and
 * prompt the user to add a provider).
 */
export function resolveActiveProvider(): ResolvedProvider | null {
  const idx = appConfig.web_search_active_index;
  const providers = appConfig.web_search_providers;
  if (idx >= 0 && idx < providers.length) {
    const provider = providers[idx];
    if (provider.enabled) {
      const meta = presetMeta(provider.preset);
      // Keyed provider → needs a key; keyless preset (e.g. Firecrawl/AnySearch
      // anonymous tier) → usable without one.
      if (provider.api_key || !meta.keyRequired) {
        return { preset: provider.preset, apiKey: provider.api_key || undefined };
      }
    }
  }
  return null;
}

/** Run a search using the resolved provider. Throws on failure (see classifySearchError). */
export async function webSearch(
  query: string,
  signal?: AbortSignal,
  maxResults?: number
): Promise<SearchHit[]> {
  const provider = resolveActiveProvider();
  if (!provider) {
    // No usable provider configured. Surface as an HTTP-style error so the
    // existing SearchFailureError → UI path renders a clear failure.
    throw new SearchHttpError(0, "No web search provider configured");
  }
  const fn = getSearchFn(provider.preset);
  return fn(query, { apiKey: provider.apiKey, signal, maxResults });
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
 * Test a web search provider's connection. Used by Settings/Onboarding. Runs a
 * trivial query. Returns { ok, status?, error? } mirroring testProviderConnection's
 * shape. Keyless presets (Firecrawl/AnySearch anonymous tier) can be tested
 * without an API key.
 */
export async function testWebSearchProvider(
  preset: string,
  apiKey: string
): Promise<{ ok: boolean; status?: number; error?: string }> {
  if (presetMeta(preset).keyRequired && !apiKey) {
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
