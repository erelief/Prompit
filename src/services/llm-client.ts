import { getActiveModel, appConfig, personaStore, loadDictionary } from "../stores/config";
import type { ApiFormat } from "../stores/config";

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
    const entries = await loadDictionary(appConfig.target_lang);
    if (entries.length > 0) {
      const dictLines = entries
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
    temperature: model.temperature ?? 0.3,
    max_tokens: model.max_tokens ?? 1024,
  };

  for (const [stdKey, defaultTarget] of Object.entries(fieldMap)) {
    if (skipFields.includes(stdKey)) continue;
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

  const baseUrl = model.base_url.replace(/\/v1\/?$/, "").replace(/\/$/, "");

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
