// Doubao Search (Global) API integration.
// Docs: https://www.volcengine.com/docs/87772/2548026
//
// POST https://open.feedcoopapi.com/search_api/global_search
// Auth: Authorization: Bearer <API_KEY>
// Body: { Query, DocCount, MaxSnippetLength }
// Response:
//   { ResponseMetadata: { ..., Error?: {...} },
//     Result: { Documents: [ { Rank, Url, Title,
//       Snippet: [ { Type: "text"|"image", Text?, Image? } ], ... } ] } | null }
//
// Doubao (火山引擎/字节跳动) Global 版 only supports pay-as-you-go and
// requires an API key (no anonymous tier), like Tavily/Exa. Each document
// carries a text/image Snippet array; we join the text snippets into one
// SearchHit.snippet.

import type { SearchFn, SearchHit, SearchOptions } from "./types";
import { assertOk, SearchHttpError, proxyFetch } from "./types";
import { throwOnDoubaoError } from "./doubao-shared";

const ENDPOINT = "https://open.feedcoopapi.com/search_api/global_search";
const DEFAULT_DOC_COUNT = 5;

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
    DocCount: opts.maxResults ?? DEFAULT_DOC_COUNT,
    MaxSnippetLength: 3000,
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

  const documents = Array.isArray(data?.Result?.Documents) ? data.Result.Documents : [];
  return (documents as any[])
    .map((d) => {
      const snippets = Array.isArray(d?.Snippet) ? d.Snippet : [];
      const text = snippets
        .filter((s: any) => s?.Type === "text" && typeof s.Text === "string")
        .map((s: any) => s.Text as string)
        .join("\n");
      return {
        title: String(d?.Title ?? ""),
        url: String(d?.Url ?? ""),
        snippet: text,
        content: text || undefined,
      };
    })
    .filter((h) => h.title || h.url || h.snippet);
};
