// Brave Search LLM Context API integration.
// Docs: https://api-dashboard.search.brave.com/api-reference/summarizer/llm_context/
//
// Unlike AnySearch (which returns a list of result hits), Brave returns a
// single pre-extracted, ranked markdown string ("llm_context") already
// formatted for LLM consumption. We wrap it as a single SearchHit so it flows
// through the existing formatSearchContext pipeline unchanged.
//
// - GET  /res/v1/llm-context?q=<query>        for queries < 2000 chars
// - POST /res/v1/llm-context  {"q":...}        for queries >= 2000 chars
// Auth header: X-Subscription-Token: <key>
// Response: { "llm_context": "<markdown string>" }
// No anonymous tier — a key is always required.

import type { SearchFn, SearchHit, SearchOptions } from "./types";
import { SearchHttpError } from "./types";

const ENDPOINT = "https://api.search.brave.com/res/v1/llm-context";
const LONG_QUERY_THRESHOLD = 2000;

export const search: SearchFn = async (
  query,
  opts: SearchOptions
): Promise<SearchHit[]> => {
  if (!opts.apiKey) {
    // Brave has no anonymous tier; surface as an auth error so the UI prompts
    // the user to configure a key.
    throw new SearchHttpError(401, "Brave Search requires an API key");
  }

  const headers: Record<string, string> = {
    Accept: "application/json",
    "X-Subscription-Token": opts.apiKey,
  };

  let response: Response;
  if (query.length < LONG_QUERY_THRESHOLD) {
    // GET with query param for short queries.
    const url = `${ENDPOINT}?q=${encodeURIComponent(query)}`;
    response = await fetch(url, { method: "GET", headers, signal: opts.signal });
  } else {
    // POST with JSON body for long queries.
    headers["Content-Type"] = "application/json";
    response = await fetch(ENDPOINT, {
      method: "POST",
      headers,
      body: JSON.stringify({ q: query }),
      signal: opts.signal,
    });
  }

  if (!response.ok) {
    const errorText = await response.text().catch(() => "");
    throw new SearchHttpError(
      response.status,
      errorText || `HTTP ${response.status}`
    );
  }

  const data = await response.json();
  const context: string = String(data?.llm_context ?? "").trim();
  if (!context) return [];

  // Wrap the markdown context blob as a single synthetic hit so it passes
  // through formatSearchContext. The title acts as a label; content carries
  // the full ranked markdown.
  return [
    {
      title: "Brave LLM Context",
      url: ENDPOINT,
      snippet: context.slice(0, 160),
      content: context,
    },
  ];
};
