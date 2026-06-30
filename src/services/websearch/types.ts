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

export { SearchHttpError } from "../errors";
