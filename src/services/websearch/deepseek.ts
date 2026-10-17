// DeepSeek Search API integration.
// Docs: https://api-docs.deepseek.com/guides/search
//
// POST https://api.deepseek.com/v1/tools/search
// Auth: Authorization: Bearer <DeepSeek API Key>
// Body: { query, count }
// Response: { results: [{ rerank_score, title, url, snippet, summary,
//           content?, site_name, site_icon, date, authority }] }
//
// Requires a DeepSeek API key (no anonymous tier), like Tavily/Exa. Results
// are already reranked; `summary` carries a richer digest than `snippet`, so
// it maps to SearchHit.content for the LLM context block.

import type { SearchFn, SearchHit, SearchOptions } from "./types";
import { assertOk, SearchHttpError, proxyFetch } from "./types";

const ENDPOINT = "https://api.deepseek.com/v1/tools/search";
const DEFAULT_COUNT = 5;
const MAX_COUNT = 50;

export const search: SearchFn = async (
  query,
  opts: SearchOptions,
): Promise<SearchHit[]> => {
  if (!opts.apiKey) {
    // No anonymous tier; surface as an auth error so the UI prompts for a key.
    throw new SearchHttpError(401, "DeepSeek requires an API key");
  }

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${opts.apiKey}`,
  };

  const body = JSON.stringify({
    query,
    count: Math.min(Math.max(opts.maxResults ?? DEFAULT_COUNT, 1), MAX_COUNT),
  });

  const response = await proxyFetch(ENDPOINT, {
    method: "POST",
    headers,
    body,
    signal: opts.signal,
  });

  assertOk(response);

  const data = JSON.parse(response.body);
  const results = Array.isArray(data?.results) ? data.results : [];
  return (results as any[])
    .map((r) => ({
      title: String(r?.title ?? ""),
      url: String(r?.url ?? ""),
      snippet: String(r?.snippet ?? ""),
      content: r?.summary != null ? String(r.summary) : undefined,
    }))
    .filter((h) => h.title || h.url || h.snippet);
};
