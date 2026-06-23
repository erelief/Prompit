// Tavily Search API integration.
// Docs: https://docs.tavily.com/documentation/api-reference/endpoint/search
//
// POST https://api.tavily.com/search
// Auth: Authorization: Bearer tvly-<key>
// Body: { query, max_results, include_answer, include_raw_content, search_depth }
// Response: { results: [{ title, url, content, raw_content?, score }] }
//
// Tavily returns a list of result hits (like AnySearch), mapped directly to
// SearchHit[]. No anonymous tier — a key is always required.

import type { SearchFn, SearchHit, SearchOptions } from "./types";
import { SearchHttpError } from "./types";

const ENDPOINT = "https://api.tavily.com/search";
const MAX_RESULTS = 5;

export const search: SearchFn = async (
  query,
  opts: SearchOptions
): Promise<SearchHit[]> => {
  if (!opts.apiKey) {
    // No anonymous tier; surface as an auth error so the UI prompts for a key.
    throw new SearchHttpError(401, "Tavily requires an API key");
  }

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${opts.apiKey}`,
  };

  // Tavily recommends keeping queries under 400 chars; send as-is for now
  // (very long inputs are unusual for the single-shot input workflow).
  const body = JSON.stringify({
    query,
    max_results: MAX_RESULTS,
    include_answer: false,
    include_raw_content: false,
    search_depth: "basic",
  });

  const response = await fetch(ENDPOINT, {
    method: "POST",
    headers,
    body,
    signal: opts.signal,
  });

  if (!response.ok) {
    const errorText = await response.text().catch(() => "");
    throw new SearchHttpError(
      response.status,
      errorText || `HTTP ${response.status}`
    );
  }

  const data = await response.json();
  const results = Array.isArray(data?.results) ? data.results : [];
  return (results as any[])
    .map((r) => ({
      title: String(r?.title ?? ""),
      url: String(r?.url ?? ""),
      snippet: String(r?.content ?? ""),
      content: r?.content != null ? String(r.content) : undefined,
    }))
    .filter((h) => h.title || h.url || h.snippet);
};
