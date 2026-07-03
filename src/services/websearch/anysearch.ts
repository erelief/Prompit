// AnySearch API integration. POST https://api.anysearch.com/v1/search
// Auth optional: Bearer key when provided, anonymous (IP-rate-limited) otherwise.
// Routes through the Rust backend proxy (proxyFetch) for CORS bypass and the
// VULN-4 scheme allow-list, consistent with how LLM provider calls are made.

import type { SearchFn, SearchHit, SearchOptions } from "./types";
import { assertOk, proxyFetch } from "./types";

const ENDPOINT = "https://api.anysearch.com/v1/search";
const DEFAULT_MAX_RESULTS = 5;

export const search: SearchFn = async (
  query,
  opts: SearchOptions
): Promise<SearchHit[]> => {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  if (opts.apiKey) {
    headers["Authorization"] = `Bearer ${opts.apiKey}`;
  }

  const maxResults = opts.maxResults ?? DEFAULT_MAX_RESULTS;
  const body = JSON.stringify({ query, max_results: maxResults });

  const response = await proxyFetch(ENDPOINT, {
    method: "POST",
    headers,
    body,
    signal: opts.signal,
  });

  assertOk(response);

  const data = JSON.parse(response.body);

  // Response shape: { data: { results: [{title, url, snippet, content}] } }
  // Tolerate either nested or flat `results` defensively.
  const results = data?.data?.results ?? data?.results ?? [];
  return (results as any[])
    .map((r) => ({
      title: String(r?.title ?? ""),
      url: String(r?.url ?? ""),
      snippet: String(r?.snippet ?? ""),
      content: r?.content != null ? String(r.content) : undefined,
    }))
    .filter((h) => h.title || h.url || h.snippet);
};
