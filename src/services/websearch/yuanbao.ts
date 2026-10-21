// Tencent Yuanbao Search (元宝搜索, WSA) API integration.
// Docs: https://cloud.tencent.com/document/product/1806/130615
//
// POST https://api.wsa.cloud.tencent.com/SearchPro
// Auth: Authorization: Bearer <API KEY>
// Body: { Query }
// Response:
//   { Response: { Pages: ["{...json string...}", ...], Query, RequestId } }
//
// Each Pages element is a JSON STRING (needs a second parse) with fields
// { title, url, passage, content?, site, score, date, images?, favicon? }.
// Errors come back as { Response: { Error: { Code, Message } } } — often with
// HTTP 200, so assertOk alone cannot surface them (same shape of problem as
// Doubao's ResponseMetadata.Error).
//
// Requires an API key (no anonymous tier), like Tavily/DeepSeek. `passage`
// (standard summary) maps to SearchHit.snippet; `content` (dynamic summary)
// maps to SearchHit.content when present. The count parameter (Cnt) is
// exclusive to the premium tier, so it is never sent; results are capped
// client-side instead.

import type { SearchFn, SearchHit, SearchOptions } from "./types";
import { assertOk, SearchHttpError, proxyFetch } from "./types";

const ENDPOINT = "https://api.wsa.cloud.tencent.com/SearchPro";
const DEFAULT_MAX_RESULTS = 5;

/** Map a WSA error code to an HTTP-ish status for SearchHttpError. */
function statusForCode(code: string): number {
  if (code.startsWith("UnauthorizedOperation")) return 401;
  if (code.startsWith("InvalidParameter")) return 400;
  if (code.startsWith("RequestLimitExceeded")) return 429;
  return 502;
}

export const search: SearchFn = async (
  query,
  opts: SearchOptions,
): Promise<SearchHit[]> => {
  if (!opts.apiKey) {
    // No anonymous tier; surface as an auth error so the UI prompts for a key.
    throw new SearchHttpError(401, "Yuanbao Search requires an API key");
  }

  const headers: Record<string, string> = {
    "Content-Type": "application/json; charset=UTF-8",
    Authorization: `Bearer ${opts.apiKey}`,
  };

  const body = JSON.stringify({ Query: query });

  const response = await proxyFetch(ENDPOINT, {
    method: "POST",
    headers,
    body,
    signal: opts.signal,
  });

  assertOk(response);

  const data = JSON.parse(response.body);
  const err = data?.Response?.Error;
  if (err && (err.Code || err.Message)) {
    throw new SearchHttpError(
      statusForCode(String(err.Code ?? "")),
      `Yuanbao ${err.Code ?? "error"}: ${err.Message ?? ""}`,
    );
  }

  // Pages is an array of JSON strings — parse each element separately and
  // drop any that fail to parse rather than failing the whole search.
  const pages = Array.isArray(data?.Response?.Pages) ? data.Response.Pages : [];
  const max = opts.maxResults ?? DEFAULT_MAX_RESULTS;
  return (pages as unknown[])
    .map((p) => {
      try {
        return typeof p === "string" ? (JSON.parse(p) as any) : (p as any);
      } catch {
        return null;
      }
    })
    .filter((p): p is any => p != null)
    .map((p) => ({
      title: String(p?.title ?? ""),
      url: String(p?.url ?? ""),
      snippet: String(p?.passage ?? ""),
      content: p?.content != null ? String(p.content) : undefined,
    }))
    .filter((h) => h.title || h.url || h.snippet)
    .slice(0, max);
};
