// Shared HTTP proxy bridge: routes web requests through the Rust backend
// (`llm_http` command) instead of the browser `fetch`. This bypasses WebView2
// CORS restrictions so providers/engines without CORS headers can connect, and
// it keeps all network egress in the Rust process (where the VULN-4 scheme
// allow-list and redirect cap are enforced).
//
// Both the LLM client and the web-search presets route through here so the
// app has a single network chokepoint.

import { invoke } from "@tauri-apps/api/core";

/** Raw HTTP response from the Rust backend proxy. */
export interface ProxyResponse {
  /** HTTP status code (0 when the request never reached the server). */
  status: number;
  /** Raw response body text. */
  body: string;
  /** True when status is in the 200–299 success range. */
  ok: boolean;
}

export interface ProxyFetchOptions {
  method: "GET" | "POST";
  headers: Record<string, string>;
  body?: string;
  /** Optional abort signal. When aborted, the in-flight request is cancelled
   *  on the Rust side via `llm_http_abort`. */
  signal?: AbortSignal;
}

/**
 * Issue an HTTP request through the Rust backend (`llm_http` command) instead
 * of the browser `fetch`.
 *
 * Transport failures throw an `Error`; HTTP error responses (4xx/5xx) resolve
 * normally with `ok: false` so callers can reuse their status-based handling.
 * An aborted request rejects with an `Error` whose name is `AbortError`.
 */
export async function proxyFetch(url: string, opts: ProxyFetchOptions): Promise<ProxyResponse> {
  // Generate a request id so the Rust side can track and abort this request.
  // crypto.randomUUID is available in all Tauri WebView2/WebKit versions.
  const requestId = (typeof crypto !== "undefined" && crypto.randomUUID)
    ? crypto.randomUUID()
    : `req-${Date.now()}-${Math.random().toString(36).slice(2)}`;

  // Wire the AbortSignal (if any) to the backend abort command. On abort we
  // also reject the promise locally so the caller sees the error immediately.
  let abortPromise: Promise<never> | undefined;
  let onAbort: (() => void) | undefined;
  if (opts.signal) {
    if (opts.signal.aborted) {
      throw new DOMException("The operation was aborted.", "AbortError");
    }
    abortPromise = new Promise<never>((_, reject) => {
      onAbort = () => {
        invoke("llm_http_abort", { requestId }).catch(() => {
          // Best-effort: the server may have already completed the request.
        });
        reject(new DOMException("The operation was aborted.", "AbortError"));
      };
      opts.signal!.addEventListener("abort", onAbort, { once: true });
    });
  }

  const invokePromise = invoke<ProxyResponse>("llm_http", {
    req: {
      method: opts.method,
      url,
      headers: opts.headers,
      body: opts.body,
      requestId,
    },
  }).catch((err) => {
    // invoke() rejects on transport-level failures (network, timeout, abort).
    // An aborted request surfaces here too (Rust returns Err("request aborted")).
    if (opts.signal?.aborted) {
      throw new DOMException("The operation was aborted.", "AbortError");
    }
    throw new Error(`Connection failed: ${err instanceof Error ? err.message : String(err)}`);
  }).finally(() => {
    // Remove the abort listener once the request settles, so a reused
    // AbortController doesn't accumulate stale listeners across requests.
    if (onAbort && opts.signal) {
      opts.signal.removeEventListener("abort", onAbort);
    }
  });

  // If we have an abort signal, race the invoke against local abort rejection.
  // Otherwise just await invoke directly.
  return abortPromise ? Promise.race([invokePromise, abortPromise]) : invokePromise;
}
