import { getActiveModel, appConfig, personaStore, loadDictionary } from "../stores/config";

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

  const body: ChatCompletionRequest = {
    model: model.model,
    messages,
    temperature: model.temperature ?? 0.3,
    max_tokens: model.max_tokens ?? 1024,
  };

  const url = model.base_url.replace(/\/v1\/?$/, "").replace(/\/$/, "");

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (model.api_key) {
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
