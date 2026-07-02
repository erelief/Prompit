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

/** Assert a fetch Response is ok; otherwise throw a SearchHttpError carrying
 *  the status and (best-effort) body text. Shared by every preset module. */
export async function assertOk(response: Response): Promise<void> {
  if (response.ok) return;
  const errorText = await response.text().catch(() => "");
  throw new SearchHttpError(response.status, errorText || `HTTP ${response.status}`);
}
