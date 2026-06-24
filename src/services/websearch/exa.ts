// Exa Search API integration.
// Docs: https://exa.ai/docs/reference/search
//
// POST https://api.exa.ai/search
// Auth: x-api-key: <key>
// Body: { query, numResults, contents: { highlights: true } }
// Response: { results: [{ title, url, text, highlights }] }

import type { SearchFn, SearchHit, SearchOptions } from "./types";
import { SearchHttpError } from "./types";

const ENDPOINT = "https://api.exa.ai/search";
const DEFAULT_MAX_RESULTS = 5;

export const search: SearchFn = async (
  query,
  opts: SearchOptions
): Promise<SearchHit[]> => {
  if (!opts.apiKey) {
    throw new SearchHttpError(401, "Exa requires an API key");
  }

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    "x-api-key": opts.apiKey,
  };

  const maxResults = opts.maxResults ?? DEFAULT_MAX_RESULTS;
  const body = JSON.stringify({
    query,
    numResults: maxResults,
    contents: {
      highlights: true,
    },
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
      snippet: String(r?.highlights?.[0] ?? r?.text ?? ""),
      content: r?.text != null ? String(r.text) : undefined,
    }))
    .filter((h) => h.title || h.url || h.snippet);
};
