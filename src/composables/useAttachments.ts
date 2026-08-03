import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { OutgoingAttachment } from "../services/llm-client";
import type { HistoryAttachment } from "../stores/config";

/** A file staged in the composer (skills_lite mode). Images carry a blob
 *  preview URL for the thumbnail strip; text files carry decoded content for
 *  inlining into the prompt. `data` (base64) is kept for both kinds so the
 *  payload can be persisted to history on send. */
export interface ComposeAttachment {
  id: string;
  name: string;
  mime: string;
  size: number;
  kind: "image" | "text";
  data?: string;
  text?: string;
  previewUrl?: string;
}

/** Structured intake rejection; the view maps `kind` to an i18n message. */
export type IntakeError =
  | { kind: "unsupported" | "readFailed"; name: string }
  | { kind: "tooLarge"; name: string; max: string }
  | { kind: "tooMany"; max: number };

// ── Intake rules (single source of truth for type classification) ──
// Images: the intersection OpenAI- and Anthropic-compatible APIs accept.
// Text: inlined into the prompt, so anything UTF-8 readable goes.
const IMAGE_EXTS = ["png", "jpg", "jpeg", "gif", "webp"];
const TEXT_EXTS = [
  "txt", "md", "markdown", "json", "jsonc", "csv", "tsv", "log", "yaml", "yml",
  "toml", "xml", "html", "htm", "css", "js", "mjs", "cjs", "ts", "tsx", "jsx",
  "py", "rs", "go", "java", "c", "h", "cpp", "hpp", "cs", "rb", "php", "sh",
  "sql", "vue", "svelte", "ini", "cfg", "conf",
];

const MAX_COUNT = 9;
const MAX_IMAGE_BYTES = 10 * 1024 * 1024;
const MAX_TEXT_BYTES = 256 * 1024;

/** Compact byte size for chip labels and error messages ("256 KB", "10 MB"). */
export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`;
  return `${(n / 1024 / 1024).toFixed(n < 10 * 1024 * 1024 ? 1 : 0)} MB`;
}

function extOf(name: string): string {
  const i = name.lastIndexOf(".");
  return i >= 0 ? name.slice(i + 1).toLowerCase() : "";
}

/** Classify by extension first (reliable), then mime. Anything else — svg /
 *  bmp images, pdf, zip, … — is rejected at intake. */
function classify(name: string, mime: string): "image" | "text" | null {
  const ext = extOf(name);
  if (IMAGE_EXTS.includes(ext)) return "image";
  if (TEXT_EXTS.includes(ext) || mime.startsWith("text/")) return "text";
  return null;
}

/** Best-effort mime for files read from disk paths (no browser File object).
 *  Image mimes must be exact — they end up in the API data URL. */
function mimeFor(name: string): string {
  const ext = extOf(name);
  if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
  if (["png", "gif", "webp"].includes(ext)) return `image/${ext}`;
  return "text/plain";
}

function bufferToBase64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf);
  let bin = "";
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) {
    bin += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(bin);
}

function base64ToBytes(b64: string): Uint8Array {
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}

function base64ToText(b64: string): string {
  return new TextDecoder().decode(base64ToBytes(b64));
}

export function useAttachments() {
  const attachments = ref<ComposeAttachment[]>([]);
  /** Last intake rejection, for the view to display. Cleared on next intake. */
  const intakeError = ref<IntakeError | null>(null);

  /** Build and stage one attachment object (no validation — the caller's job). */
  function stageOne(name: string, mime: string, size: number, data: string, kind: "image" | "text"): void {
    const att: ComposeAttachment = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name,
      mime,
      size,
      kind,
      data,
    };
    if (kind === "image") {
      att.previewUrl = URL.createObjectURL(new Blob([base64ToBytes(data)], { type: mime }));
    } else {
      att.text = base64ToText(data);
    }
    attachments.value.push(att);
  }

  /** Stage one decoded file. Returns false (and sets intakeError) on reject. */
  function pushOne(name: string, mime: string, size: number, data: string): boolean {
    const kind = classify(name, mime);
    if (!kind) {
      intakeError.value = { kind: "unsupported", name };
      return false;
    }
    if (attachments.value.length >= MAX_COUNT) {
      intakeError.value = { kind: "tooMany", max: MAX_COUNT };
      return false;
    }
    const maxBytes = kind === "image" ? MAX_IMAGE_BYTES : MAX_TEXT_BYTES;
    if (size > maxBytes) {
      intakeError.value = { kind: "tooLarge", name, max: formatBytes(maxBytes) };
      return false;
    }
    stageOne(name, mime, size, data, kind);
    return true;
  }

  /** Intake from disk paths: the + button's open dialog and native drag-drop. */
  async function addFromPaths(paths: string[]): Promise<void> {
    intakeError.value = null;
    for (const path of paths) {
      try {
        const f = await invoke<{ name: string; size: number; data_base64: string }>(
          "read_attachment_file",
          { path },
        );
        pushOne(f.name, mimeFor(f.name), Number(f.size), f.data_base64);
      } catch {
        intakeError.value = { kind: "readFailed", name: path.split(/[\\/]/).pop() ?? path };
      }
    }
  }

  /** Intake from browser File objects: clipboard paste. */
  async function addFromFiles(files: File[]): Promise<void> {
    intakeError.value = null;
    for (const file of files) {
      // Pasted images usually arrive nameless ("image.png" or "").
      const name = file.name || `pasted-${Date.now()}.png`;
      const mime = file.type || mimeFor(name);
      const data = bufferToBase64(await file.arrayBuffer());
      pushOne(name, mime, file.size, data);
    }
  }

  function removeAttachment(id: string): void {
    const idx = attachments.value.findIndex((a) => a.id === id);
    if (idx < 0) return;
    const [att] = attachments.value.splice(idx, 1);
    if (att.previewUrl) URL.revokeObjectURL(att.previewUrl);
  }

  function clearAttachments(): void {
    for (const att of attachments.value) {
      if (att.previewUrl) URL.revokeObjectURL(att.previewUrl);
    }
    attachments.value = [];
  }

  /** Restore attachments from a history entry (history navigation / panel
   *  restore): read the persisted files back and stage them anew, bypassing
   *  intake limits (they were accepted once already). Files that went missing
   *  out of band are skipped silently. */
  async function restoreFromHistory(list: HistoryAttachment[]): Promise<void> {
    clearAttachments();
    intakeError.value = null;
    for (const att of list) {
      try {
        const f = await invoke<{ name: string; size: number; data_base64: string }>(
          "read_history_attachment",
          { path: att.path },
        );
        const kind: "image" | "text" = att.mime.startsWith("image/") ? "image" : "text";
        stageOne(att.name, att.mime, Number(f.size), f.data_base64, kind);
      } catch { /* file missing — skip */ }
    }
  }

  /** Shape for translate(): images + inlined text content. */
  function toOutgoing(): OutgoingAttachment[] {
    return attachments.value.map((a) => ({
      name: a.name,
      mime: a.mime,
      kind: a.kind,
      data: a.data,
      text: a.text,
    }));
  }

  return {
    attachments,
    intakeError,
    addFromPaths,
    addFromFiles,
    removeAttachment,
    clearAttachments,
    restoreFromHistory,
    toOutgoing,
  };
}
