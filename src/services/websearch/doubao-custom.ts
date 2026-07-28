// Doubao Search (Custom) API integration.
// Docs: https://www.volcengine.com/docs/87772/2272953
//
// POST https://open.feedcoopapi.com/search_api/web_search
// Auth: Authorization: Bearer <API_KEY>
// Body: { Query, SearchType, Count }   (Filter defaults: NeedContent=false, NeedUrl=true)
// Response:
//   { ResponseMetadata: { ..., Error?: {...} },
//     Result: { WebResults: [ { Title, SiteName, Url, Snippet, Summary,
//       Content?, PublishTime, LogoUrl, ... } ] } | null }
//
// Each WebItem carries flat fields, unlike the Global version's text/image
// Snippet array. We prefer Summary (AI-generated, richer) for the snippet.

import type { SearchFn, SearchHit, SearchOptions } from "./types";
import { assertOk, SearchHttpError, proxyFetch } from "./types";
import { throwOnDoubaoError } from "./doubao-shared";

const ENDPOINT = "https://open.feedcoopapi.com/search_api/web_search";
const DEFAULT_COUNT = 5;

export const search: SearchFn = async (
  query,
  opts: SearchOptions
): Promise<SearchHit[]> => {
  if (!opts.apiKey) {
    // No anonymous tier; surface as an auth error so the UI prompts for a key.
    throw new SearchHttpError(401, "Doubao requires an API key");
  }

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${opts.apiKey}`,
  };

  const body = JSON.stringify({
    Query: query,
    SearchType: "web",
    Count: opts.maxResults ?? DEFAULT_COUNT,
  });

  const response = await proxyFetch(ENDPOINT, {
    method: "POST",
    headers,
    body,
    signal: opts.signal,
  });

  assertOk(response);

  const data = JSON.parse(response.body);
  throwOnDoubaoError(data);

  const webResults = Array.isArray(data?.Result?.WebResults) ? data.Result.WebResults : [];
  return (webResults as any[])
    .map((d) => {
      const text = (d?.Summary ?? d?.Snippet ?? "") as string;
      return {
        title: String(d?.Title ?? ""),
        url: String(d?.Url ?? ""),
        snippet: String(text),
        content: d?.Content != null ? String(d.Content) : text || undefined,
      };
    })
    .filter((h) => h.title || h.url || h.snippet);
};
