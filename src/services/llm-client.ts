import { getActiveModel, appConfig } from "../stores/config";

const TRANSLATION_SYSTEM_PROMPT = `You are a translation engine. Translate the user's input text to the target language.
Rules:
- Output ONLY the translated text, nothing else.
- Preserve the original punctuation style and line breaks.
- Do not add explanations, notes, or any extra content.
- If the input is already in the target language, output it as-is.`;

const AIFW_DEFAULT_URL = "http://localhost:8844/api/call";

interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

interface ChatCompletionRequest {
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  max_tokens?: number;
}

interface ChatCompletionResponse {
  choices: Array<{
    message: {
      content: string;
    };
  }>;
}

export async function translate(text: string): Promise<string> {
  const model = getActiveModel();
  if (!model) {
    throw new Error("No model configured. Please add a model in Settings.");
  }

  const systemPrompt = buildSystemPrompt();

  const messages: ChatMessage[] = [
    { role: "system", content: systemPrompt },
    { role: "user", content: text },
  ];

  const body: ChatCompletionRequest = {
    model: model.model,
    messages,
    temperature: model.temperature ?? 0.3,
    max_tokens: model.max_tokens ?? 1024,
  };

  const url = getRequestUrl(model.base_url);

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (!appConfig.privacy_mode && model.api_key) {
    headers["Authorization"] = `Bearer ${model.api_key}`;
  }

  const response = await fetch(`${url}/chat/completions`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`API error ${response.status}: ${errorText}`);
  }

  const data = (await response.json()) as ChatCompletionResponse;

  if (!data.choices || data.choices.length === 0) {
    throw new Error("Empty response from LLM API");
  }

  return data.choices[0].message.content.trim();
}

function buildSystemPrompt(): string {
  let prompt = TRANSLATION_SYSTEM_PROMPT;
  prompt += `\nTarget language: ${appConfig.target_lang}.`;

  if (appConfig.persona) {
    prompt += `\nTranslation style: ${appConfig.persona}.`;
  }

  return prompt;
}

function getRequestUrl(baseUrl: string): string {
  if (appConfig.privacy_mode) {
    return AIFW_DEFAULT_URL;
  }

  const clean = baseUrl.replace(/\/v1\/?$/, "").replace(/\/$/, "");
  return clean;
}

export const _internals = {
  buildSystemPrompt,
  getRequestUrl,
  TRANSLATION_SYSTEM_PROMPT,
  AIFW_DEFAULT_URL,
};
