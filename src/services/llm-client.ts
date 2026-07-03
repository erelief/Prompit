import { getActiveModel, appConfig, personaStore, skillsLiteStore, loadDictionary } from "../stores/config";
import type { ApiFormat, ProviderConfig, ModelInputCapabilities } from "../stores/config";
import { detectInputCapabilitiesAsync } from "./model-capabilities";
import { webSearch, formatSearchContext } from "./websearch";
import type { ClassifiedSearchError, SearchHit } from "./websearch/types";
import { proxyFetch, type ProxyResponse } from "./proxy";

interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

/**
 * Issue an HTTP request through the Rust backend (`llm_http` command) instead
 * of the browser `fetch`. This bypasses WebView2 CORS restrictions so providers
 * that don't return CORS headers (e.g. Volcano Engine Ark) can connect.
 *
 * Transport failures throw an `Error`; HTTP error responses (4xx/5xx) resolve
 * normally with `ok: false` so callers can reuse their status-based handling.
 *
 * Thin wrapper over the shared `proxyFetch` (see `proxy.ts`) so both the LLM
 * client and the web-search presets share one network chokepoint.
 */
async function llmFetch(
  url: string,
  opts: { method: "GET" | "POST"; headers: Record<string, string>; body?: string },
): Promise<ProxyResponse> {
  return proxyFetch(url, opts);
}

export type TranslateOutcome =
  | { status: "ok"; content: string; searched: boolean; sources?: SearchHit[] }
  | { status: "search-error"; error: ClassifiedSearchError };

/** Wraps a search failure so the caller (FloatingInput) can classify it. */
export class SearchFailureError extends Error {
  cause: unknown;
  constructor(cause: unknown) {
    super("Web search failed");
    this.name = "SearchFailureError";
    this.cause = cause;
  }
}

import { ModelHttpError } from "./errors";
export { ModelHttpError } from "./errors";

export interface FetchModelEntry {
  id: string;
  input_capabilities: ModelInputCapabilities; // {} when nothing detected
}

const OPENAI_FORMAT: Required<ApiFormat> = {
  auth_header: "Authorization",
  auth_prefix: "Bearer ",
  extra_headers: {},
  chat_endpoint: "/chat/completions",
  models_endpoint: "/models",
  response: {},
  system_key: "",
  force_fields: [],
};

export function resolveFormat(apiFormat?: ApiFormat): Required<ApiFormat> {
  if (!apiFormat) return { ...OPENAI_FORMAT };
  return {
    auth_header: apiFormat.auth_header ?? OPENAI_FORMAT.auth_header,
    auth_prefix: apiFormat.auth_prefix ?? OPENAI_FORMAT.auth_prefix,
    extra_headers: apiFormat.extra_headers ?? {},
    chat_endpoint: apiFormat.chat_endpoint ?? OPENAI_FORMAT.chat_endpoint,
    models_endpoint: apiFormat.models_endpoint ?? OPENAI_FORMAT.models_endpoint,
    response: apiFormat.response ?? {},
    system_key: apiFormat.system_key ?? "",
    force_fields: apiFormat.force_fields ?? [],
  };
}

/** Build request headers for a resolved format: optional Content-Type, the
 *  auth header (when both auth_header and a key are present), then any
 *  extra_headers the preset declares. */
function buildHeaders(
  fmt: Required<ApiFormat>,
  apiKey: string,
  jsonBody = false,
): Record<string, string> {
  const headers: Record<string, string> = {};
  if (jsonBody) headers["Content-Type"] = "application/json";
  if (fmt.auth_header && apiKey) {
    headers[fmt.auth_header] = `${fmt.auth_prefix}${apiKey}`;
  }
  Object.assign(headers, fmt.extra_headers);
  return headers;
}

/** Build request body. Extracts system messages into a top-level field when
 *  `system_key` is set (Anthropic), and forces required fields (Anthropic
 *  requires `max_tokens`). Field-name remapping is intentionally not
 *  supported — every preset speaks the OpenAI field vocabulary directly. */
function buildRequestBody(
  fmt: Required<ApiFormat>,
  model: string,
  messages: ChatMessage[],
  temperature: number | null,
  maxTokens: number | null,
): Record<string, any> {
  // Extract system messages if system_key is configured (e.g. Anthropic's
  // top-level `system` field — its /messages API rejects role:"system").
  let systemContent: string | undefined;
  let nonSystemMessages = messages;
  if (fmt.system_key) {
    const systemMsgs = messages.filter((m) => m.role === "system");
    if (systemMsgs.length > 0) {
      systemContent = systemMsgs.map((m) => m.content).join("\n\n");
    }
    nonSystemMessages = messages.filter((m) => m.role !== "system");
  }

  const body: Record<string, any> = {
    model,
    messages: nonSystemMessages,
  };
  if (temperature != null) body.temperature = temperature;
  // max_tokens: Anthropic requires it; fill the conventional default when the
  // user left it unset. OpenAI-style APIs accept it as optional.
  body.max_tokens = maxTokens ?? (fmt.force_fields.includes("max_tokens") ? 4096 : undefined);
  if (body.max_tokens == null) delete body.max_tokens;

  if (fmt.system_key && systemContent !== undefined) {
    body[fmt.system_key] = systemContent;
  }

  return body;
}

/** Resolve a dot-path like "choices.0.message.content" against an object. */
export function resolvePath(obj: any, path: string): any {
  if (!path) return obj;
  let cur = obj;
  for (const part of path.split(".")) {
    if (cur == null) return undefined;
    const idx = Number(part);
    cur = Number.isInteger(idx) && Array.isArray(cur) ? cur[idx] : cur[part];
  }
  return cur;
}

/** Shared "send a chat request, return the trimmed content string" sequence.
 *  Used by translate() and optimizePrompt(), which both post to a chat
 *  endpoint and extract a single content field. Throws ModelHttpError on
 *  non-2xx, Error on an empty response. */
async function chatCompletion(
  model: ReturnType<typeof getActiveModel>,
  messages: ChatMessage[],
  fmt: Required<ApiFormat>,
  signal?: AbortSignal,
): Promise<string> {
  if (!model) {
    throw new Error("No model configured. Please add a model in Settings.");
  }
  const body = buildRequestBody(fmt, model.model, messages, model.temperature, model.max_tokens);
  const headers = buildHeaders(fmt, model.api_key, true);
  const baseUrl = model.base_url.replace(/\/$/, "");

  signal?.throwIfAborted();
  const response = await llmFetch(`${baseUrl}${fmt.chat_endpoint}`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    throw new ModelHttpError(response.status, response.body || `HTTP ${response.status}`);
  }
  signal?.throwIfAborted();

  const data = JSON.parse(response.body);
  const contentPath = fmt.response["content"] ?? "choices.0.message.content";
  const content = resolvePath(data, contentPath);
  if (content == null) {
    throw new Error("Empty response from LLM API");
  }
  return String(content).trim();
}

export async function translate(text: string, signal?: AbortSignal): Promise<TranslateOutcome> {
  const model = getActiveModel();
  const fmt = resolveFormat(model?.api_format);

  const mode = appConfig.active_mode || "translate";
  const systemPrompt = mode === "skills_lite"
    ? buildSkillsLiteSystemPrompt()
    : buildSystemPrompt();

  const messages: ChatMessage[] = [
    { role: "system", content: systemPrompt },
  ];

  if (mode === "translate" && appConfig.user_dict_enabled) {
    const allEntries = await loadDictionary(appConfig.target_lang);
    const activePersona = personaStore.personas.find((p) => p.enabled)?.name || null;
    const matched = allEntries.filter((e) => {
      if (!text.includes(e.source)) return false;
      if (!e.persona) return true;
      return e.persona === activePersona;
    });
    if (matched.length > 0) {
      const dictLines = matched
        .map((e) => `- "${e.source}" → "${e.target}"`)
        .join("\n");
      messages.push({
        role: "system",
        content: `User dictionary — you MUST use these exact translations:\n${dictLines}`,
      });
    }
  }

  // ── Web search (Skills Lite mode only) ──
  let searched = false;
  let sources: SearchHit[] | undefined;
  if (mode === "skills_lite" && appConfig.web_search_enabled_in_skills_lite) {
    try {
      const hits = await webSearch(text, signal);
      if (hits.length > 0) {
        messages.push({ role: "user", content: formatSearchContext(hits) });
        sources = hits;
      }
      searched = true;
    } catch (searchErr) {
      // Blocking: do NOT call the LLM without the requested context.
      // Defer classification to the caller to avoid an i18n import cycle here.
      throw new SearchFailureError(searchErr);
    }
  }

  messages.push({ role: "user", content: text });

  const content = await chatCompletion(model, messages, fmt, signal);
  return { status: "ok", content, searched, sources };
}

export async function optimizePrompt(rawPrompt: string, mode: "translate" | "skills_lite" | "summarize" = "translate"): Promise<string> {
  const model = getActiveModel();
  const fmt = resolveFormat(model?.api_format);

  const messages: ChatMessage[] = [
    {
      role: "system",
      content: mode === "skills_lite"
        ? "You organize and structure user-written prompts. Reorganize the prompt to be clear, well-structured, and unambiguous. Do not change the original intent or add new instructions. Output ONLY the reorganized prompt, nothing else."
        : mode === "summarize"
        ? "Detect the language of the following prompt and reply in THAT same language. Be extremely concise: under 20 characters for Chinese (under 12 words for English). Start directly with the action in the form \"<verb> the input into <result>\" (e.g. 将输入内容润色得更自然 / Rewrite the input more formally / Turn the input into a bulleted summary). Pick the verb that best fits the prompt. No filler, no subject (no \"This tool\"/\"Acts as\"/\"本工具\"/\"该助手\"). Output ONLY that single line, nothing else."
        : "You optimize persona prompts for a translation tool. The user writes a vague style description; you convert it into a concise, structured role instruction that assigns the LLM a professional identity.\n" +
          "Language: Detect the language of the user's input and write the ENTIRE output in that same language, including the template words (e.g. \"You are a / 你是一位\").\n" +
          "Output format (adapt the template words to the input's language):\n" +
          "\"You are a [role] with [years/level] of experience in [domain]. You specialize in [specific skill]. Your audience is [who you are writing for].\"\n" +
          "- Derive each bracket from the user's description. If the input is too vague to fill a bracket, infer the most fitting value yourself based on the role and context.\n" +
          "- Keep the result to one or two sentences.\n" +
          'Examples:\n' +
          '- "像个影视专业人员" → "你是一位拥有10年经验的影视行业从业者，专精于剧本与分镜等专业术语的表达，你的受众是普通观众。"\n' +
          '- "正式一点" → "你是一位资深学术学者，专精于严谨规范的学术写作，你的受众是同行评审专家。"\n' +
          '- "口语化" → "你是一位拥有多年日常对话经验的地道母语者，专精于轻松自然的口语表达，你的受众是朋友和同龄人。"\n' +
          '- "Make it more professional" → "You are a seasoned professional with over 10 years of experience in business communication. You specialize in formal, polished corporate writing. Your audience is clients and executives."\n' +
          "- Output ONLY the optimized prompt, nothing else.",
    },
    { role: "user", content: rawPrompt },
  ];

  return chatCompletion(model, messages, fmt);
}

function buildSystemPrompt(): string {
  const enabledPersonas = personaStore.personas.filter((p) => p.enabled);

  let rules = "";
  for (const persona of enabledPersonas) {
    rules += `\n- Additional style instructions: ${persona.prompt}`;
  }
  rules += "\n- Output ONLY the translated text, nothing else.";
  rules += "\n- Preserve the original punctuation style and line breaks.";
  rules += "\n- Do not add explanations, notes, or any extra content.";
  rules += "\n- If the input is already in the target language, output it as-is.";

  return `You are a translation engine. Translate the user's input text to the target language.\nRules:${rules}\nTarget language: ${appConfig.target_lang}.`;
}

function buildSkillsLiteSystemPrompt(): string {
  const enabled = skillsLiteStore.skillsLites.find((s) => s.enabled);
  if (!enabled) {
    return "You are a helpful assistant. Output ONLY the result, nothing else.";
  }
  return (
    enabled.prompt +
    "\n\nIMPORTANT: Output ONLY the transformed result. Do not include any explanations, notes, meta-commentary, or original text. Output just the result."
  );
}

export async function testProviderConnection(
  provider: Pick<ProviderConfig, "api_key" | "base_url" | "api_format">
): Promise<{ ok: boolean; status?: number; error?: string }> {
  if (!provider.api_key || !provider.base_url) {
    return { ok: false, error: "Missing API key or base URL" };
  }
  try {
    const fmt = resolveFormat(provider.api_format);
    const url = provider.base_url.replace(/\/$/, "");
    const headers = buildHeaders(fmt, provider.api_key);
    const modelsEndpoint = fmt.models_endpoint || "/models";
    const r = await llmFetch(`${url}${modelsEndpoint}`, {
      method: "GET",
      headers,
    });
    if (r.ok) {
      return { ok: true };
    } else {
      const errorText = r.body;
      let detail = `Failed (${r.status})`;
      try {
        const parsed = JSON.parse(errorText);
        if (parsed.error?.message) detail = `${r.status}: ${parsed.error.message}`;
      } catch {
        if (errorText) detail = `${r.status}: ${errorText.slice(0, 120)}`;
      }
      return { ok: false, status: r.status, error: detail };
    }
  } catch (err) {
    return { ok: false, error: `Connection failed: ${err instanceof Error ? err.message : String(err)}` };
  }
}

export async function fetchProviderModels(
  provider: Pick<ProviderConfig, "api_key" | "base_url" | "api_format">
): Promise<{ ok: boolean; models?: FetchModelEntry[]; error?: string }> {
  if (!provider.api_key || !provider.base_url) {
    return { ok: false, error: "Missing API key or base URL" };
  }
  try {
    const fmt = resolveFormat(provider.api_format);
    const url = provider.base_url.replace(/\/$/, "");
    const headers = buildHeaders(fmt, provider.api_key);
    const modelsEndpoint = fmt.models_endpoint || "/models";
    const r = await llmFetch(`${url}${modelsEndpoint}`, {
      method: "GET",
      headers,
    });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    const data = JSON.parse(r.body);

    // Extract raw {id, raw} pairs so layer ① can inspect the full object.
    // All providers serve the OpenAI shape { data: [{id, ...}] }.
    const arr = Array.isArray(data?.data) ? data.data : [];
    const pairs: Array<{ id: string; raw: any }> = arr
      .map((m: any) => ({ id: String(m?.id ?? ""), raw: m }))
      .filter((p: { id: string }) => p.id);

    // Detect capabilities per model (maintained list loaded lazily + cached).
    const entries = await Promise.all(
      pairs.map(async (p) => ({
        id: p.id,
        input_capabilities: await detectInputCapabilitiesAsync(p.raw, p.id),
      })),
    );
    entries.sort((a, b) => a.id.localeCompare(b.id));
    return { ok: true, models: entries };
  } catch (err) {
    return {
      ok: false,
      error: `Fetch failed: ${err instanceof Error ? err.message : String(err)}`,
    };
  }
}
