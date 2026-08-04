import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { OutgoingAttachment } from "../services/llm-client";
import type { HistoryAttachment } from "../stores/config";

/** A file staged in the composer (skills_lite mode). Images carry a blob
 *  preview URL for the thumbnail strip; text files carry decoded content for
 *  inlining into the prompt; videos carry bytes only (icon chip). `data`
 *  (base64) is kept for all kinds so the payload can be persisted to history
 *  on send. */
export interface ComposeAttachment {
  id: string;
  name: string;
  mime: string;
  size: number;
  kind: "image" | "video" | "text";
  data?: string;
  text?: string;
  previewUrl?: string;
}

/** Structured intake rejection; the view maps `kind` to an i18n message. */
export type IntakeError =
  | { kind: "unsupported" | "readFailed"; name: string }
  | { kind: "tooLarge"; name: string; max: string }
  | { kind: "tooHighRes"; name: string; max: string }
  | { kind: "tooMany"; max: number };

// ── Intake rules (single source of truth for type classification) ──
// Images: the intersection OpenAI- and Anthropic-compatible APIs accept.
// Videos: containers the webview can probe for dimensions.
// Text: inlined into the prompt, so anything UTF-8 readable goes — svg
// included (it is XML; sent inlined as text rather than as an image).
const IMAGE_EXTS = ["png", "jpg", "jpeg", "gif", "webp"];
const VIDEO_EXTS = ["mp4", "m4v", "mov", "webm", "ogv"];
const TEXT_EXTS = [
  "txt", "md", "markdown", "json", "jsonc", "csv", "tsv", "log", "yaml", "yml",
  "toml", "xml", "svg", "html", "htm", "css", "js", "mjs", "cjs", "ts", "tsx",
  "jsx", "py", "rs", "go", "java", "c", "h", "cpp", "hpp", "cs", "rb", "php",
  "sh", "sql", "vue", "svelte", "ini", "cfg", "conf",
];

const MAX_COUNT = 9;
/** Media (image/video) must stay strictly below 10 MB — the app does no
 *  recompression or transcoding of its own; oversize files are rejected. */
const MAX_MEDIA_BYTES = 10 * 1024 * 1024;
const MAX_TEXT_BYTES = 256 * 1024;
/** Pixel-count caps (width × height): 4K for images, 1080p for videos. */
const MAX_IMAGE_PIXELS = 4096 * 2160;
const MAX_VIDEO_PIXELS = 1920 * 1080;

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

/** Classify by extension first (reliable), then mime. Anything else — bmp
 *  images, pdf, zip, … — is rejected at intake. */
function classify(name: string, mime: string): "image" | "video" | "text" | null {
  const ext = extOf(name);
  if (IMAGE_EXTS.includes(ext)) return "image";
  if (VIDEO_EXTS.includes(ext) || mime.startsWith("video/")) return "video";
  if (TEXT_EXTS.includes(ext) || mime.startsWith("text/")) return "text";
  return null;
}

const VIDEO_MIMES: Record<string, string> = {
  mp4: "video/mp4",
  m4v: "video/x-m4v",
  mov: "video/quicktime",
  webm: "video/webm",
  ogv: "video/ogg",
};

/** Best-effort mime for files read from disk paths (no browser File object).
 *  Media mimes must be exact — they end up in the API data URL. */
function mimeFor(name: string): string {
  const ext = extOf(name);
  if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
  if (["png", "gif", "webp"].includes(ext)) return `image/${ext}`;
  return VIDEO_MIMES[ext] ?? "text/plain";
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

/** Pixel count (width × height) of an image payload; null when undecodable. */
async function probeImagePixels(data: string, mime: string): Promise<number | null> {
  try {
    const bmp = await createImageBitmap(new Blob([base64ToBytes(data)], { type: mime }));
    const pixels = bmp.width * bmp.height;
    bmp.close();
    return pixels;
  } catch {
    return null;
  }
}

/** Pixel count of a video payload, read off a throwaway <video> element;
 *  null when the webview can't parse the container/codec (or it hangs). */
async function probeVideoPixels(data: string, mime: string): Promise<number | null> {
  const url = URL.createObjectURL(new Blob([base64ToBytes(data)], { type: mime }));
  try {
    return await new Promise<number | null>((resolve) => {
      const v = document.createElement("video");
      const timer = setTimeout(() => resolve(null), 10_000);
      v.preload = "metadata";
      v.onloadedmetadata = () => {
        clearTimeout(timer);
        resolve(v.videoWidth * v.videoHeight || null);
      };
      v.onerror = () => {
        clearTimeout(timer);
        resolve(null);
      };
      v.src = url;
    });
  } finally {
    URL.revokeObjectURL(url);
  }
}

export function useAttachments() {
  const attachments = ref<ComposeAttachment[]>([]);
  /** Last intake rejection, for the view to display. Cleared on next intake. */
  const intakeError = ref<IntakeError | null>(null);

  /** Build and stage one attachment object (no validation — the caller's job). */
  function stageOne(name: string, mime: string, size: number, data: string, kind: "image" | "video" | "text"): void {
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
    } else if (kind === "text") {
      att.text = base64ToText(data);
    }
    attachments.value.push(att);
  }

  /** Stage one decoded file. Returns false (and sets intakeError) on reject. */
  async function pushOne(name: string, mime: string, size: number, data: string): Promise<boolean> {
    const kind = classify(name, mime);
    if (!kind) {
      intakeError.value = { kind: "unsupported", name };
      return false;
    }
    if (attachments.value.length >= MAX_COUNT) {
      intakeError.value = { kind: "tooMany", max: MAX_COUNT };
      return false;
    }
    if (kind === "text") {
      if (size > MAX_TEXT_BYTES) {
        intakeError.value = { kind: "tooLarge", name, max: formatBytes(MAX_TEXT_BYTES) };
        return false;
      }
    } else {
      if (size >= MAX_MEDIA_BYTES) {
        intakeError.value = { kind: "tooLarge", name, max: formatBytes(MAX_MEDIA_BYTES) };
        return false;
      }
      const pixels = kind === "image"
        ? await probeImagePixels(data, mime)
        : await probeVideoPixels(data, mime);
      if (pixels === null) {
        intakeError.value = { kind: "readFailed", name };
        return false;
      }
      const maxPixels = kind === "image" ? MAX_IMAGE_PIXELS : MAX_VIDEO_PIXELS;
      if (pixels > maxPixels) {
        intakeError.value = {
          kind: "tooHighRes",
          name,
          max: kind === "image" ? "4096×2160" : "1920×1080",
        };
        return false;
      }
    }
    stageOne(name, mime, size, data, kind);
    return true;
  }

  /** Intake from disk paths: the + button's open dialog and native drag-drop. */
  async function addFromPaths(paths: string[]): Promise<void> {
    intakeError.value = null;
    for (const path of paths) {
      const name = path.split(/[\\/]/).pop() ?? path;
      try {
        const f = await invoke<{ name: string; size: number; data_base64: string }>(
          "read_attachment_file",
          { path },
        );
        await pushOne(f.name, mimeFor(f.name), Number(f.size), f.data_base64);
      } catch (e) {
        // The backend's 20 MB read backstop fires before the frontend limits
        // get a chance — surface it as the size rejection it is.
        if (String(e).includes("too large")) {
          const kind = classify(name, mimeFor(name));
          if (!kind) {
            intakeError.value = { kind: "unsupported", name };
          } else {
            const maxBytes = kind === "text" ? MAX_TEXT_BYTES : MAX_MEDIA_BYTES;
            intakeError.value = { kind: "tooLarge", name, max: formatBytes(maxBytes) };
          }
        } else {
          intakeError.value = { kind: "readFailed", name };
        }
      }
    }
  }

  /** Intake from browser File objects: clipboard paste. */
  async function addFromFiles(files: File[]): Promise<void> {
    intakeError.value = null;
    for (const file of files) {
      // Pasted media usually arrives nameless ("image.png" or "").
      const fallbackExt = file.type.startsWith("video/") ? "mp4" : "png";
      const name = file.name || `pasted-${Date.now()}.${fallbackExt}`;
      const mime = file.type || mimeFor(name);
      const data = bufferToBase64(await file.arrayBuffer());
      await pushOne(name, mime, file.size, data);
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
        // Re-derive the kind with today's rules (e.g. svg is text now, not an
        // image) instead of trusting the stored mime prefix.
        const kind = classify(att.name, att.mime)
          ?? (att.mime.startsWith("image/") ? "image" : "text");
        stageOne(att.name, att.mime, Number(f.size), f.data_base64, kind);
      } catch { /* file missing — skip */ }
    }
  }

  /** Shape for translate(): media payloads + inlined text content. */
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
