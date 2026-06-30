import type { ModelInputCapabilities, ModelCapabilityItem } from "../stores/config";
import { loadModelCapabilities } from "../stores/config";

/** Raw entry from a /models response (shape varies by provider). */
type RawModel = any;

// ── Layer cache (loaded lazily, once per session) ──
let _maintainedList: ModelCapabilityItem[] | null = null;

async function ensureMaintainedList(): Promise<ModelCapabilityItem[]> {
  if (_maintainedList === null) {
    try {
      _maintainedList = await loadModelCapabilities();
    } catch {
      _maintainedList = [];
    }
  }
  return _maintainedList;
}

// ── Modality keyword registry ──
// Each modality has three sets of heuristic rules. Layer ① is provider-agnostic
// (reads raw objects) so it's shared and only needs the keyword to look for in
// arrays/strings; layers ②③ are modality-specific.

interface ModalityKeywords {
  /** Lowercased exact match in an array (layer ①) */
  arrayValue: string;
  /** Substring in the input part of a modality string (layer ①) */
  stringSubstr: string;
  /** Heuristic substrings that imply this modality in a model id (layer ③) */
  substrings: string[];
  /** Heuristic prefixes (layer ③) */
  prefixes: string[];
  /** Substrings that override positives (layer ③) */
  excludeSubstrings: string[];
}

const MODALITY_KEYWORDS: Record<string, ModalityKeywords> = {
  image: {
    arrayValue: "image",
    stringSubstr: "image",
    substrings: ["vision", "llava", "-vl", "vl-", "multimodal", "glm-4v", "qwen2-vl", "qwen-vl"],
    prefixes: ["gpt-4o", "gpt-4-turbo", "claude-3", "gemini-1.5", "gemini-2"],
    excludeSubstrings: ["-text"],
  },
  audio: {
    arrayValue: "audio",
    stringSubstr: "audio",
    substrings: ["whisper", "audio", "speech", "tts", "asr"],
    prefixes: [],
    excludeSubstrings: [],
  },
  video: {
    arrayValue: "video",
    stringSubstr: "video",
    substrings: ["video", "llava-video"],
    prefixes: [],
    excludeSubstrings: [],
  },
};

// ── Layer ①: API capability field ──
// Parse provider-authoritative capability info from the raw /models object.
// Only returns true when a field explicitly lists the modality. Never returns
// false from absence (so lower layers can still rule).
function detectFromApiField(raw: RawModel, keywords: ModalityKeywords): boolean | undefined {
  if (raw == null || typeof raw !== "object") return undefined;

  // Structured array form (e.g. OpenAI newer responses):
  //   architecture.input_modalities: ["text", "image", "audio", ...]
  const modalities =
    raw?.architecture?.input_modalities ??
    raw?.input_modalities ??
    raw?.capabilities?.input_modalities;
  if (Array.isArray(modalities)) {
    const has = modalities.some(
      (m) => typeof m === "string" && m.toLowerCase() === keywords.arrayValue,
    );
    return has ? true : undefined;
  }

  // String form (OpenRouter's primary field):
  //   architecture.modality: "text+image+audio->text"  (inputs -> outputs)
  // Only the LEFT side of "->" describes INPUT capability.
  const modalityStr = raw?.architecture?.modality ?? raw?.modality;
  if (typeof modalityStr === "string" && modalityStr.length > 0) {
    const inputPart = modalityStr.includes("->")
      ? modalityStr.split("->")[0]
      : modalityStr;
    return inputPart.toLowerCase().includes(keywords.stringSubstr) ? true : undefined;
  }

  return undefined;
}

// ── Layer ②: dev-maintained glob list ──
function globMatch(pattern: string, id: string): boolean {
  const escaped = pattern
    .replace(/[.+^${}()|[\]\\]/g, "\\$&")
    .replace(/\*/g, ".*");
  return new RegExp(`^${escaped}$`, "i").test(id);
}

function detectFromMaintainedList(
  id: string,
  list: ModelCapabilityItem[],
  field: keyof ModelInputCapabilities,
): boolean | undefined {
  for (const entry of list) {
    if (globMatch(entry.id, id)) {
      return entry.input_capabilities[field] ?? undefined;
    }
  }
  return undefined;
}

// ── Layer ③: name heuristic ──
function detectFromHeuristic(id: string, keywords: ModalityKeywords): boolean {
  const lower = id.toLowerCase();
  if (keywords.excludeSubstrings.some((s) => lower.includes(s))) return false;
  if (keywords.prefixes.some((p) => lower.startsWith(p))) return true;
  if (keywords.substrings.some((s) => lower.includes(s))) return true;
  return false;
}

// ── Public API ──

/** Detect a single modality across all three layers (in priority order). */
function detectModality(
  raw: RawModel,
  id: string,
  maintainedList: ModelCapabilityItem[],
  field: keyof ModelInputCapabilities,
  keywords: ModalityKeywords,
): boolean | undefined {
  return (
    detectFromApiField(raw, keywords) ??
    detectFromMaintainedList(id, maintainedList, field) ??
    detectFromHeuristic(id, keywords) ??
    undefined
  );
}

/** Detect the full input-capabilities object for a model.
 *  Returns an object with only the fields where detection had a result
 *  (unknown capability => key omitted => clean config.json on persist). */
export function detectInputCapabilities(
  raw: RawModel,
  id: string,
  maintainedList: ModelCapabilityItem[],
): ModelInputCapabilities {
  const caps: ModelInputCapabilities = {};
  for (const [field, keywords] of Object.entries(MODALITY_KEYWORDS)) {
    const val = detectModality(raw, id, maintainedList, field as keyof ModelInputCapabilities, keywords);
    if (val !== undefined) (caps as any)[field] = val;
  }
  return caps;
}

/** Convenience: load the maintained list (cached) then detect. */
export async function detectInputCapabilitiesAsync(
  raw: RawModel,
  id: string,
): Promise<ModelInputCapabilities> {
  const list = await ensureMaintainedList();
  return detectInputCapabilities(raw, id, list);
}
