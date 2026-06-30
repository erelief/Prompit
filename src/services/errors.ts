/** HTTP error carrying a status code. Shared by the LLM client (ModelHttpError)
 *  and the web-search modules (SearchHttpError), which used to ship byte-identical
 *  copies. Both names below are aliases of this one class so existing
 *  `instanceof` checks keep working. */
export class HttpError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = "HttpError";
    this.status = status;
  }
}

/** Alias retained for call-site readability in the LLM client. */
export const ModelHttpError = HttpError;
/** Alias retained for call-site readability in the web-search modules. */
export const SearchHttpError = HttpError;
