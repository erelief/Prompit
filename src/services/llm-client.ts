import { getActiveModel, appConfig, personaStore, loadDictionary } from "../stores/config";
import type { ApiFormat, ProviderConfig } from "../stores/config";

interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

const OPENAI_FORMAT: Required<ApiFormat> = {
  auth_header: "Authorization",
  auth_prefix: "Bearer ",
  extra_headers: {},
  chat_endpoint: "/chat/completions",
  models_endpoint: "/models",
  request: {},
  response: {},
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
  };
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

export async function translate(text: string): Promise<string> {
  const model = getActiveModel();
  if (!model) {
    throw new Error("No model configured. Please add a model in Settings.");
  }

  const fmt = resolveFormat(model.api_format);
  const skipFields: string[] = fmt.request._skip_fields ?? [];

  const systemPrompt = buildSystemPrompt();

  const messages: ChatMessage[] = [
    { role: "system", content: systemPrompt },
  ];

  if (appConfig.user_dict_enabled) {
    const allEntries = await loadDictionary(appConfig.target_lang);
    const matched = allEntries.filter((e) => text.includes(e.source));
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

  messages.push({ role: "user", content: text });

  // Build request body using field mappings
  const body: Record<string, any> = {};
  const fieldMap: Record<string, string> = {
    model: "model",
    messages: "messages",
    temperature: "temperature",
    max_tokens: "max_tokens",
  };

  const values: Record<string, any> = {
    model: model.model,
    messages,
    temperature: model.temperature,
    max_tokens: model.max_tokens,
  };

  for (const [stdKey, defaultTarget] of Object.entries(fieldMap)) {
    if (skipFields.includes(stdKey)) continue;
    if (values[stdKey] == null) continue;
    const targetKey = fmt.request[stdKey] ?? defaultTarget;
    body[targetKey] = values[stdKey];
  }

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

  const response = await fetch(`${baseUrl}${fmt.chat_endpoint}`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`API error ${response.status}: ${errorText}`);
  }

  const data = await response.json();

  const contentPath = fmt.response["content"] ?? "choices.0.message.content";
  const content = resolvePath(data, contentPath);

  if (content == null) {
    throw new Error("Empty response from LLM API");
  }

  return String(content).trim();
}

export async function optimizePrompt(rawPrompt: string): Promise<string> {
  const model = getActiveModel();
  if (!model) {
    throw new Error("No model configured. Please add a model in Settings.");
  }

  const fmt = resolveFormat(model.api_format);
  const skipFields: string[] = fmt.request._skip_fields ?? [];

  const messages: ChatMessage[] = [
    {
      role: "system",
      content:
        "You optimize persona prompts for a translation tool. The user writes a vague style description in any language; you convert it into a concise English instruction that tells the LLM how to translate.\n" +
        "Output format: Start with an imperative verb (Use/Apply/Translate with/Simplify to), then specify tone, vocabulary domain, or formality level. Under 20 words.\n" +
        'Examples:\n' +
        '- "像个影视专业人员" → "Use professional film and television audiovisual language vocabulary."\n' +
        '- "正式一点" → "Use formal academic tone and precise terminology."\n' +
        '- "口语化" → "Use casual conversational tone with everyday vocabulary."\n' +
        "- Output ONLY the optimized prompt, nothing else.",
    },
    { role: "user", content: rawPrompt },
  ];

  const body: Record<string, any> = {};
  const fieldMap: Record<string, string> = {
    model: "model",
    messages: "messages",
    temperature: "temperature",
    max_tokens: "max_tokens",
  };

  const values: Record<string, any> = {
    model: model.model,
    messages,
    temperature: model.temperature,
    max_tokens: model.max_tokens,
  };

  for (const [stdKey, defaultTarget] of Object.entries(fieldMap)) {
    if (skipFields.includes(stdKey)) continue;
    if (values[stdKey] == null) continue;
    const targetKey = fmt.request[stdKey] ?? defaultTarget;
    body[targetKey] = values[stdKey];
  }

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

  const response = await fetch(`${baseUrl}${fmt.chat_endpoint}`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`API error ${response.status}: ${errorText}`);
  }

  const data = await response.json();

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
    const r = await fetch(`${url}${modelsEndpoint}`, {
      method: "GET",
      headers,
    });
    if (r.ok) {
      return { ok: true };
    } else {
      await r.text();
      return { ok: false, status: r.status, error: `Failed (${r.status})` };
    }
  } catch {
    return { ok: false, error: "Connection failed" };
  }
}

export async function fetchProviderModels(
  provider: Pick<ProviderConfig, "api_key" | "base_url" | "api_format">
): Promise<{ ok: boolean; models?: string[]; error?: string }> {
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
    const r = await fetch(`${url}${modelsEndpoint}`, {
      method: "GET",
      headers,
    });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    const data = await r.json();

    const modelsListPath = fmt.response["models_list"];
    let modelIds: string[];
    if (modelsListPath) {
      const raw = resolvePath(data, modelsListPath.replace(/\.\*$/, ""));
      modelIds = Array.isArray(raw) ? raw.filter((m: any) => typeof m === "string").sort() : [];
    } else {
      modelIds = data.data?.map((m: any) => m.id).sort() || [];
    }
    return { ok: true, models: modelIds };
  } catch (err) {
    return {
      ok: false,
      error: `Fetch failed: ${err instanceof Error ? err.message : String(err)}`,
    };
  }
}
