import { getActiveModel, appConfig, personaStore, skillsLiteStore, loadDictionary } from "../stores/config";
import type { ApiFormat, ProviderConfig, ModelInputCapabilities } from "../stores/config";
import { invoke } from "@tauri-apps/api/core";
import { detectInputCapabilitiesAsync } from "./model-capabilities";
import { webSearch, formatSearchContext } from "./websearch";
import type { ClassifiedSearchError, SearchHit } from "./websearch/types";

interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

/** Raw HTTP response from the Rust backend proxy. */
interface LlmProxyResponse {
  status: number;
  body: string;
  ok: boolean;
}

/**
 * Issue an HTTP request through the Rust backend (`llm_http` command) instead
 * of the browser `fetch`. This bypasses WebView2 CORS restrictions so providers
 * that don't return CORS headers (e.g. Volcano Engine Ark) can connect.
 *
 * Transport failures throw an `Error`; HTTP error responses (4xx/5xx) resolve
 * normally with `ok: false` so callers can reuse their status-based handling.
 */
async function llmFetch(
  url: string,
  opts: { method: "GET" | "POST"; headers: Record<string, string>; body?: string },
): Promise<LlmProxyResponse> {
  try {
    return await invoke<LlmProxyResponse>("llm_http", {
      req: {
        method: opts.method,
        url,
        headers: opts.headers,
        body: opts.body,
      },
    });
  } catch (err) {
    // invoke() rejects on transport-level failures (network, timeout, etc.)
    throw new Error(`Connection failed: ${err instanceof Error ? err.message : String(err)}`);
  }
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

/** Thrown by translate/optimizePrompt on HTTP errors; carries status code. */
export class ModelHttpError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = "ModelHttpError";
    this.status = status;
  }
}

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
  request: {},
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
    request: apiFormat.request ?? {},
    response: apiFormat.response ?? {},
    system_key: apiFormat.system_key ?? "",
    force_fields: apiFormat.force_fields ?? [],
  };
}

/** Build request body with field mappings, system_key extraction, and force_fields support. */
function buildRequestBody(
  fmt: Required<ApiFormat>,
  model: string,
  messages: ChatMessage[],
  temperature: number | null,
  maxTokens: number | null,
): Record<string, any> {
  const skipFields: string[] = fmt.request._skip_fields ?? [];

  // Extract system messages if system_key is configured
  let systemContent: string | undefined;
  let nonSystemMessages = messages;
  if (fmt.system_key) {
    const systemMsgs = messages.filter((m) => m.role === "system");
    if (systemMsgs.length > 0) {
      systemContent = systemMsgs.map((m) => m.content).join("\n\n");
    }
    nonSystemMessages = messages.filter((m) => m.role !== "system");
  }

  const body: Record<string, any> = {};
  const fieldMap: Record<string, string> = {
    model: "model",
    messages: "messages",
    temperature: "temperature",
    max_tokens: "max_tokens",
  };

  const defaults: Record<string, any> = {
    max_tokens: 4096,
  };

  const values: Record<string, any> = {
    model,
    messages: nonSystemMessages,
    temperature,
    max_tokens: maxTokens,
  };

  for (const [stdKey, defaultTarget] of Object.entries(fieldMap)) {
    if (skipFields.includes(stdKey)) continue;
    const val = values[stdKey];
    const targetKey = fmt.request[stdKey] ?? defaultTarget;

    // force_fields: include field even if value is null, using a default
    if (val == null) {
      if (fmt.force_fields.includes(stdKey)) {
        body[targetKey] = defaults[stdKey] ?? 0;
      }
      continue;
    }
    body[targetKey] = val;
  }

  // Set system prompt as top-level field if system_key is configured
  if (fmt.system_key && systemContent !== undefined) {
    body[fmt.system_key] = systemContent;
  }

  return body;
}

/** Resolve a dot-path like "choices.0.message.content" against an object. */
export function resolvePath(obj: any, path: string): any {
  if (!path) return obj;
  const parts = path.split(".");
  let cur = obj;
  for (const part of parts) {
    if (cur == null) return undefined;
    if (part === "*") continue; // wildcard — caller handles
    const idx = Number(part);
    cur = Number.isInteger(idx) && Array.isArray(cur) ? cur[idx] : cur[part];
  }
  return cur;
}

export async function translate(text: string, signal?: AbortSignal): Promise<TranslateOutcome> {
  const model = getActiveModel();
  if (!model) {
    throw new Error("No model configured. Please add a model in Settings.");
  }

  const fmt = resolveFormat(model.api_format);

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

  // Build request body using field mappings
  const body = buildRequestBody(
    fmt,
    model.model,
    messages,
    model.temperature,
    model.max_tokens,
  );

  // Build headers
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  if (fmt.auth_header && model.api_key) {
    headers[fmt.auth_header] = `${fmt.auth_prefix}${model.api_key}`;
  }
  for (const [k, v] of Object.entries(fmt.extra_headers)) {
    headers[k] = v;
  }

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

  return { status: "ok", content: String(content).trim(), searched, sources };
}

export async function optimizePrompt(rawPrompt: string, mode: "translate" | "skills_lite" | "summarize" = "translate"): Promise<string> {
  const model = getActiveModel();
  if (!model) {
    throw new Error("No model configured. Please add a model in Settings.");
  }

  const fmt = resolveFormat(model.api_format);

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

  const body = buildRequestBody(
    fmt,
    model.model,
    messages,
    model.temperature,
    model.max_tokens,
  );

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  if (fmt.auth_header && model.api_key) {
    headers[fmt.auth_header] = `${fmt.auth_prefix}${model.api_key}`;
  }
  for (const [k, v] of Object.entries(fmt.extra_headers)) {
    headers[k] = v;
  }

  const baseUrl = model.base_url.replace(/\/$/, "");

  const response = await llmFetch(`${baseUrl}${fmt.chat_endpoint}`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    throw new ModelHttpError(response.status, response.body || `HTTP ${response.status}`);
  }

  const data = JSON.parse(response.body);

  const contentPath = fmt.response["content"] ?? "choices.0.message.content";
  const content = resolvePath(data, contentPath);

  if (content == null) {
    throw new Error("Empty response from LLM API");
  }

  return String(content).trim();
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
    const headers: Record<string, string> = {};
    if (fmt.auth_header && provider.api_key) {
      headers[fmt.auth_header] = `${fmt.auth_prefix}${provider.api_key}`;
    }
    for (const [k, v] of Object.entries(fmt.extra_headers)) {
      headers[k] = v;
    }
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
    const headers: Record<string, string> = {};
    if (fmt.auth_header && provider.api_key) {
      headers[fmt.auth_header] = `${fmt.auth_prefix}${provider.api_key}`;
    }
    for (const [k, v] of Object.entries(fmt.extra_headers)) {
      headers[k] = v;
    }
    const modelsEndpoint = fmt.models_endpoint || "/models";
    const r = await llmFetch(`${url}${modelsEndpoint}`, {
      method: "GET",
      headers,
    });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    const data = JSON.parse(r.body);

    // Extract raw {id, raw} pairs so layer ① can inspect the full object.
    const modelsListPath = fmt.response["models_list"];
    let pairs: Array<{ id: string; raw: any }>;
    if (modelsListPath) {
      const raw = resolvePath(data, modelsListPath.replace(/\.\*$/, ""));
      if (!Array.isArray(raw)) {
        pairs = [];
      } else if (raw.length > 0 && typeof raw[0] === "string") {
        pairs = raw.map((m: any) => ({ id: String(m), raw: m }));
      } else {
        // Array of objects — extract id via the configured path or "id".
        pairs = raw
          .map((m: any) => ({ id: String(m?.id ?? ""), raw: m }))
          .filter((p: any) => p.id);
      }
    } else {
      const arr = Array.isArray(data?.data) ? data.data : [];
      pairs = arr
        .map((m: any) => ({ id: String(m?.id ?? ""), raw: m }))
        .filter((p: { id: string }) => p.id);
    }

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
