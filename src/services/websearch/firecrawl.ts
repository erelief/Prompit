// Firecrawl Search API integration.
// Docs: https://docs.firecrawl.dev/api-reference/endpoint/search
//
// POST https://api.firecrawl.dev/v2/search
// Auth: Bearer <api-key>
// Request: { query, limit }
// Response: { success, data: { web: [{ title, description, url }] } }

import type { SearchFn, SearchHit, SearchOptions } from "./types";
import { assertOk } from "./types";

const ENDPOINT = "https://api.firecrawl.dev/v2/search";
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
  const body = JSON.stringify({ query, limit: maxResults });

  const response = await fetch(ENDPOINT, {
    method: "POST",
    headers,
    body,
    signal: opts.signal,
  });

  await assertOk(response);

  const data = await response.json();

  const results = data?.data?.web ?? [];
  return (results as any[])
    .map((r) => ({
      title: String(r?.title ?? ""),
      url: String(r?.url ?? ""),
      snippet: String(r?.description ?? ""),
      content: r?.markdown != null ? String(r.markdown) : undefined,
    }))
    .filter((h) => h.title || h.url || h.snippet);
};
