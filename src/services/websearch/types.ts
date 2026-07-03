// Shared types for the web-search service. Each preset module exports a
// SearchFn matching the contract here; the registry routes by preset id.

export interface SearchHit {
  title: string;
  url: string;
  snippet: string;
  content?: string;
}

export interface SearchOptions {
  apiKey?: string;
  /** Max results to request. Falls back to each preset's default when unset. */
  maxResults?: number;
  signal?: AbortSignal;
}

/** Every preset module exports a function matching this signature. */
export type SearchFn = (query: string, opts: SearchOptions) => Promise<SearchHit[]>;

/** Result of classifying a search error for user display. */
export interface ClassifiedSearchError {
  code: string; // HTTP status as string, or "NETWORK" / "UNKNOWN"
  rawMessage?: string; // Raw error text from the server
}

import { SearchHttpError } from "../errors";
export { SearchHttpError };
import { proxyFetch } from "../proxy";
import type { ProxyResponse } from "../proxy";
// Re-export the shared proxy fetch so each preset module can route its HTTP
// calls through the Rust backend (CORS bypass + VULN-4 scheme allow-list).
export { proxyFetch, type ProxyResponse };

/** Assert a proxy response is ok; otherwise throw a SearchHttpError carrying
 *  the status and (best-effort) body text. Shared by every preset module.
 *
 *  Takes the `ProxyResponse` shape (from the Rust `llm_http` command) rather
 *  than a browser `Response`, since all search calls now route through the
 *  backend proxy (VULN-5). */
export function assertOk(resp: ProxyResponse): void {
  if (resp.ok) return;
  throw new SearchHttpError(resp.status, resp.body || `HTTP ${resp.status}`);
}
