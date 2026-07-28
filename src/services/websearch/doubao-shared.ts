// Shared helper for the Doubao (Volcengine) search presets. Both the Global
// and Custom endpoints return the same error envelope: Result: null with the
// cause in ResponseMetadata.Error.{ CodeN, Code, Message } (HTTP is often 200,
// so assertOk alone cannot surface these).

import { SearchHttpError } from "./types";

/** Throw a SearchHttpError if `data` carries a Doubao error envelope. */
export function throwOnDoubaoError(data: any): void {
  const err = data?.ResponseMetadata?.Error;
  if (!err || (err.CodeN == null && !err.Code && !err.Message)) return;
  const status = typeof err.CodeN === "number" && err.CodeN >= 400 && err.CodeN < 600 ? err.CodeN : 502;
  throw new SearchHttpError(status, `Doubao ${err.Code ?? "error"}: ${err.Message ?? ""}`);
}
