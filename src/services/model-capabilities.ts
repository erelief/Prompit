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

/** Force a re-load of the maintained list (e.g. for testing). */
export function resetCapabilityCache(): void {
  _maintainedList = null;
}

// ── Layer ①: API capability field ──
// Parse provider-authoritative capability info from the raw /models object.
// Only returns true when a field explicitly lists "image" on the INPUT side.
// Never returns false from absence (so lower layers can still rule).
function detectFromApiField(raw: RawModel): boolean | undefined {
  if (raw == null || typeof raw !== "object") return undefined;

  // Structured array form (e.g. OpenAI newer responses):
  //   architecture.input_modalities: ["text", "image", ...]
  const modalities =
    raw?.architecture?.input_modalities ??
    raw?.input_modalities ??
    raw?.capabilities?.input_modalities;
  if (Array.isArray(modalities)) {
    const has = modalities.some(
      (m) => typeof m === "string" && m.toLowerCase() === "image",
    );
    return has ? true : undefined;
  }

  // String form (OpenRouter's primary field):
  //   architecture.modality: "text+image->text"  (inputs -> outputs)
  // Only the LEFT side of "->" describes INPUT capability, so a text-to-image
  // model like "text->image" is correctly treated as NOT supporting image input.
  const modalityStr = raw?.architecture?.modality ?? raw?.modality;
  if (typeof modalityStr === "string" && modalityStr.length > 0) {
    const inputPart = modalityStr.includes("->")
      ? modalityStr.split("->")[0]
      : modalityStr;
    return inputPart.toLowerCase().includes("image") ? true : undefined;
  }

  return undefined;
}

// ── Layer ②: dev-maintained glob list ──
function globMatch(pattern: string, id: string): boolean {
  // Convert simple glob (* matches anything) to regex, case-insensitive.
  const escaped = pattern
    .replace(/[.+^${}()|[\]\\]/g, "\\$&")
    .replace(/\*/g, ".*");
  return new RegExp(`^${escaped}$`, "i").test(id);
}

function detectFromMaintainedList(
  id: string,
  list: ModelCapabilityItem[],
): boolean | undefined {
  for (const entry of list) {
    if (globMatch(entry.id, id)) {
      return entry.input_capabilities.image;
    }
  }
  return undefined;
}

// ── Layer ③: name heuristic ──
// Lowercased substring/series patterns. Conservative — reduce false positives.
const HEURISTIC_SUBSTRINGS = [
  "vision", "llava", "-vl", "vl-", "multimodal", "glm-4v", "qwen2-vl", "qwen-vl",
];
const HEURISTIC_PREFIXES = ["gpt-4o", "gpt-4-turbo", "claude-3", "gemini-1.5", "gemini-2"];
const HEURISTIC_TEXT_ONLY_SUBSTRINGS = ["-text"]; // overrides positives

function detectFromHeuristic(id: string): boolean {
  const lower = id.toLowerCase();
  if (HEURISTIC_TEXT_ONLY_SUBSTRINGS.some((s) => lower.includes(s))) return false;
  if (HEURISTIC_PREFIXES.some((p) => lower.startsWith(p))) return true;
  if (HEURISTIC_SUBSTRINGS.some((s) => lower.includes(s))) return true;
  return false;
}

/** Detect image-input capability across all three layers (in priority order).
 *  Returns the first definitive result, or undefined if no layer can rule. */
export function detectImageInput(
  raw: RawModel,
  id: string,
  maintainedList: ModelCapabilityItem[],
): boolean | undefined {
  return (
    detectFromApiField(raw) ??
    detectFromMaintainedList(id, maintainedList) ??
    detectFromHeuristic(id) ??
    undefined
  );
}

/** Detect the full input-capabilities object for a model. Today only `image`;
 *  future modalities add their own detect* functions and assemble here. */
export function detectInputCapabilities(
  raw: RawModel,
  id: string,
  maintainedList: ModelCapabilityItem[],
): ModelInputCapabilities {
  const image = detectImageInput(raw, id, maintainedList);
  const caps: ModelInputCapabilities = {};
  if (image !== undefined) caps.image = image;
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
