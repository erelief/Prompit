<script setup lang="ts">
import { ref, reactive, computed, onMounted } from "vue";
import { useRouter } from "vue-router";

import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { save as saveFileDialog } from "@tauri-apps/plugin-dialog";
import { ArrowLeft, History, Trash2, Check, X, Send, MessageSquare, Globe, ExternalLink, ToggleRight, ToggleLeft, Search, Download, Image, FileText } from "@lucide/vue";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { useWindowBg, domainOf } from "../composables/useWindowBg";
import { formatBytes } from "../composables/useAttachments";
import { appConfig, historyStore, loadHistory, saveHistory, MODES, HISTORY_LIMIT_DEFAULT, type HistoryEntry, type HistoryAttachment } from "../stores/config";
import { useI18n } from "vue-i18n";
import type { SearchHit } from "../services/websearch/types";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

const windowBg = useWindowBg();

const showClearConfirm = ref(false);
const pendingRemove = ref<number | null>(null);

async function goBack() {
  router.push("/");
}

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, a, .history-item, .history-search, input")) return;
  await getCurrentWindow().startDragging();
}

function selectEntry(entry: { input: string; output: string }) {
  sessionStorage.setItem("history-restore", JSON.stringify(entry));
  router.push("/");
}

/** Entry whose sources are currently shown in the in-place overlay, or null. */
const sourcesEntry = ref<HistoryEntry | null>(null);
function openSources(entry: HistoryEntry) {
  sourcesEntry.value = entry;
}
function closeSources() {
  sourcesEntry.value = null;
}
const sourcesList = computed<SearchHit[]>(() => sourcesEntry.value?.sources ?? []);

function shortModel(model: string): string {
  // strip date suffix like "-2024-07-18"
  return model.replace(/-\d{4}-\d{2}-\d{2}$/, "");
}

// Compact token count for the per-entry badge (1.2K / 3.4M …); the full
// prompt/completion breakdown goes into the hover tooltip.
const tokenCompactFmt = new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 });
function usageTitle(entry: HistoryEntry): string {
  const u = entry.usage;
  if (!u) return "";
  return t("history.usageBreakdown", {
    prompt: u.prompt != null ? String(u.prompt) : "–",
    completion: u.completion != null ? String(u.completion) : "–",
  });
}

// Mode → icon component for the left indicator (display only)
function modeIcon(mode?: string) {
  return MODES.find(m => m.id === mode)?.icon ?? MODES[0].icon;
}
// Persona (translate) / Skills Lite (skills_lite) name to show as a tag — display only.
// Accept the legacy "sparkle" mode value so older history entries still resolve.
function presetTag(entry: HistoryEntry): string | null {
  if (entry.mode === "translate") return entry.persona || null;
  if (entry.mode === "skills_lite" || entry.mode === "sparkle") return entry.skills_lite || null;
  return null;
}

// All modes share one unified history list
const modeEntries = computed(() => historyStore.entries);

// ── Real-time search (matches input OR output, case-insensitive) ──
const searchQuery = ref("");
const filteredEntries = computed<HistoryEntry[]>(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return historyStore.entries;
  return historyStore.entries.filter(
    (e) =>
      (e.input && e.input.toLowerCase().includes(q)) ||
      (e.output && e.output.toLowerCase().includes(q)),
  );
});
function clearSearch() {
  searchQuery.value = "";
}

// Split text into segments so matched substrings can be highlighted.
// Returns a single non-highlighted segment when there is no active query.
function highlightSegments(text: string): { text: string; hit: boolean }[] {
  const q = searchQuery.value.trim();
  if (!q) return [{ text, hit: false }];
  const lower = text.toLowerCase();
  const ql = q.toLowerCase();
  const parts: { text: string; hit: boolean }[] = [];
  let i = 0;
  while (i < text.length) {
    const idx = lower.indexOf(ql, i);
    if (idx === -1) {
      parts.push({ text: text.slice(i), hit: false });
      break;
    }
    if (idx > i) parts.push({ text: text.slice(i, idx), hit: false });
    parts.push({ text: text.slice(idx, idx + q.length), hit: true });
    i = idx + q.length;
  }
  return parts;
}

async function handleClear() {
  // Clear all history (shared across modes)
  historyStore.entries = [];
  await saveHistory();
  showClearConfirm.value = false;
}

function requestRemove(ts: number) {
  pendingRemove.value = ts;
}
function cancelRemove() {
  pendingRemove.value = null;
}
async function confirmRemove(ts: number) {
  const idx = historyStore.entries.findIndex(e => e.timestamp === ts);
  if (idx >= 0) historyStore.entries.splice(idx, 1);
  pendingRemove.value = null;
  await saveHistory();
}

/** Download a persisted attachment: ask where, then the backend copies the
 *  file out (arbitrary destination paths bypass the fs plugin scope). */
async function downloadAttachment(att: HistoryAttachment) {
  const dest = await saveFileDialog({ defaultPath: att.name });
  if (!dest) return;
  try {
    await invoke("export_history_attachment", { path: att.path, dest });
  } catch (err) {
    console.error("Failed to export attachment:", err);
  }
}

/** Lazily loaded data-URL thumbnails for image attachments (hover preview).
 *  Keyed by attachment path; "" while the read is in flight. */
const attachmentThumbs = reactive<Record<string, string>>({});
/** Popover direction per attachment path: a tag in the window's upper half
 *  pops downward (more room below), lower half pops upward. */
const thumbDirs = reactive<Record<string, "up" | "down">>({});
async function ensureThumb(att: HistoryAttachment, e?: MouseEvent) {
  if (!att.mime.startsWith("image/")) return;
  if (e?.currentTarget) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    thumbDirs[att.path] = rect.top + rect.height / 2 < window.innerHeight / 2 ? "down" : "up";
  }
  if (att.path in attachmentThumbs) return;
  attachmentThumbs[att.path] = "";
  try {
    const f = await invoke<{ data_base64: string }>("read_history_attachment", { path: att.path });
    attachmentThumbs[att.path] = `data:${att.mime};base64,${f.data_base64}`;
  } catch {
    delete attachmentThumbs[att.path];
  }
}

onMounted(async () => {
  await loadHistory();
});
</script>

<template>
  <div
    class="history-root"
    :class="{ 'grow-above': growAbove }"
    @mousedown="handleDrag"
    :style="{ background: windowBg }"
  >
    <!-- Header (normal mode vs sources mode) -->
    <header class="history-header">
      <button v-if="sourcesEntry" @click="closeSources" class="back-btn" :title="t('search.backToHistory')">
        <ArrowLeft :size="18" :stroke-width="1.8" />
      </button>
      <button v-else @click="goBack" class="back-btn" :title="t('common.back')">
        <ArrowLeft :size="18" :stroke-width="1.8" />
      </button>
      <h1 v-if="sourcesEntry" class="header-title">
        <Globe :size="14" :stroke-width="1.8" />
        {{ t('search.sourcesTitle') }}
      </h1>
      <h1 v-else class="header-title">
        <History :size="14" :stroke-width="1.8" />
        {{ t('history.title') }}
      </h1>
      <!-- Real-time search: filters by input OR output -->
      <div v-if="!sourcesEntry" class="history-search" :class="{ 'has-query': searchQuery }">
        <Search :size="13" :stroke-width="1.9" class="search-icon" />
        <input
          v-model="searchQuery"
          type="text"
          class="search-input"
          :placeholder="t('history.searchPlaceholder')"
          @click.stop
        />
        <button
          v-if="searchQuery"
          class="search-clear"
          :title="t('common.cancel')"
          @click.stop="clearSearch"
        >
          <X :size="12" :stroke-width="2.2" />
        </button>
      </div>
      <div v-if="!sourcesEntry" class="header-actions">
        <button
          class="reset-btn"
          :class="{ 'toggle-on': appConfig.history_enabled }"
          @click.stop="appConfig.history_enabled = !appConfig.history_enabled"
        >
          <ToggleRight v-if="appConfig.history_enabled" :size="11" :stroke-width="1.9" />
          <ToggleLeft v-else :size="11" :stroke-width="1.9" />{{ t('history.historyEnabled') }}
        </button>
        <button
          v-if="!showClearConfirm && modeEntries.length > 0"
          class="reset-btn danger"
          @click.stop="showClearConfirm = true"
          :title="t('history.clearAll')"
        >
          <Trash2 :size="11" :stroke-width="1.9" />{{ t('history.clearAll') }}
        </button>
        <template v-if="showClearConfirm">
          <button class="mini-btn danger-active" :title="t('common.confirm')" @click.stop="handleClear">
            <Check :size="11" :stroke-width="2.5" />
          </button>
          <button class="mini-btn" :title="t('common.cancel')" @click.stop="showClearConfirm = false">
            <X :size="11" :stroke-width="2.5" />
          </button>
        </template>
      </div>
    </header>

    <!-- Body: sources view (replaces history list when a 🌐 tag is active) -->
    <main class="history-body">
      <template v-if="sourcesEntry">
        <a v-for="(src, i) in sourcesList" :key="i"
           :href="src.url" target="_blank" rel="noopener noreferrer" class="source-item">
          <div class="source-favicon"><Globe :size="13" :stroke-width="1.8" /></div>
          <div class="source-meta">
            <div class="source-title">{{ src.title || t('search.untitledSource') }}</div>
            <div class="source-domain">{{ domainOf(src.url) }}</div>
          </div>
          <ExternalLink :size="11" :stroke-width="1.8" class="source-external" />
        </a>
        <div v-if="sourcesList.length === 0" class="sources-empty">{{ t('search.noSources') }}</div>
      </template>
      <!-- Normal history list -->
      <template v-else>
      <div v-if="historyStore.entries.length === 0" class="empty-state">
        <History :size="28" :stroke-width="1" class="empty-icon" />
        <span class="empty-text">{{ t('history.empty') }}<br><small>{{ t('history.emptySub') }}</small></span>
      </div>
      <div v-else-if="filteredEntries.length === 0" class="empty-state">
        <Search :size="28" :stroke-width="1" class="empty-icon" />
        <span class="empty-text">{{ t('history.searchEmpty') }}</span>
      </div>
      <div v-else class="history-list">
        <div
          v-for="entry in filteredEntries"
          :key="entry.timestamp"
          class="history-item"
          :class="{ 'remove-pending': pendingRemove === entry.timestamp }"
        >
          <template v-if="pendingRemove === entry.timestamp">
            <span class="remove-warning-text">{{ t('common.cannotBeUndone') }}</span>
            <div class="remove-actions">
              <button class="mini-btn danger-active" :title="t('common.confirm')" @click.stop="confirmRemove(entry.timestamp)">
                <Check :size="11" :stroke-width="2.5" />
              </button>
              <button class="mini-btn" :title="t('common.cancel')" @click.stop="cancelRemove">
                <X :size="11" :stroke-width="2.5" />
              </button>
            </div>
          </template>
          <template v-else>
            <button class="history-item-main" @click="selectEntry(entry)">
              <div class="mode-indicator">
                <component :is="modeIcon(entry.mode)" :size="12" :stroke-width="1.8" />
              </div>
              <div class="history-item-text">
              <div class="history-item-input">
                <Send :size="9" :stroke-width="2" class="input-icon" />
                <span :title="entry.input"><template v-for="(seg, si) in highlightSegments(entry.input)" :key="si"><mark v-if="seg.hit" class="search-hl">{{ seg.text }}</mark><template v-else>{{ seg.text }}</template></template></span>
              </div>
                <div class="history-item-output">
                  <MessageSquare :size="9" :stroke-width="2" class="output-icon" />
                  <span :title="entry.output"><template v-for="(seg, si) in highlightSegments(entry.output)" :key="si"><mark v-if="seg.hit" class="search-hl">{{ seg.text }}</mark><template v-else>{{ seg.text }}</template></template></span>
                  <span v-if="entry.model" class="model-badge">{{ shortModel(entry.model) }}</span>
                  <span v-if="presetTag(entry)" class="preset-badge">{{ presetTag(entry) }}</span>
                  <span v-if="entry.edited" class="edited-tag">{{ t('history.edited') }}</span>
                  <button v-if="entry.searched" class="searched-tag" :title="t('search.sourcesTitle')" @click.stop="openSources(entry)">
                    <Globe :size="9" :stroke-width="2" />
                  </button>
                  <span v-if="entry.usage?.total" class="model-badge usage-badge" :title="usageTitle(entry)">{{ tokenCompactFmt.format(entry.usage.total) }} tokens</span>
                </div>
                <div v-if="entry.attachments?.length" class="history-item-attachments">
                  <span
                    v-for="(att, ai) in entry.attachments"
                    :key="ai"
                    class="attachment-tag"
                    :class="{ 'pop-below': thumbDirs[att.path] === 'down' }"
                    :title="att.name"
                    @mouseenter="ensureThumb(att, $event)"
                  >
                    <Image v-if="att.mime.startsWith('image/')" :size="9" :stroke-width="2" class="attachment-icon" />
                    <FileText v-else :size="9" :stroke-width="2" class="attachment-icon" />
                    <span class="attachment-name">{{ att.name }}</span>
                    <span class="attachment-size">{{ formatBytes(att.size) }}</span>
                    <button class="attachment-download" :title="t('history.downloadAttachment')" @click.stop="downloadAttachment(att)">
                      <Download :size="9" :stroke-width="2" />
                    </button>
                    <span v-if="attachmentThumbs[att.path]" class="attachment-thumb-pop">
                      <img :src="attachmentThumbs[att.path]" :alt="att.name" />
                    </span>
                  </span>
                </div>
              </div>
            </button>
            <div class="history-item-actions" @click.stop>
              <button class="mini-btn warn" :title="t('common.remove')" @click="requestRemove(entry.timestamp)">
                <Trash2 :size="11" :stroke-width="1.9" />
              </button>
            </div>
          </template>
        </div>
        <div class="history-footer">
          <template v-if="searchQuery">
            {{ t('history.searchCount', { current: filteredEntries.length, total: historyStore.entries.length }) }}
          </template>
          <template v-else>
            {{ t('history.entryCount', { current: modeEntries.length, limit: appConfig.history_limit || HISTORY_LIMIT_DEFAULT }) }}
          </template>
        </div>
      </div>
      </template>
    </main>
  </div>
</template>

<style scoped>
.history-root {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: calc(100dvh / var(--font-scale, 1));
  overflow: hidden;
}
.history-root.grow-above {
  flex-direction: column-reverse;
}
.history-root.grow-above .history-header {
  border-bottom: none;
  border-top: 1px solid var(--color-surface);
}

/* Header */
.history-header {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-6) var(--space-3);
  flex-shrink: 0;
  border-bottom: 1px solid var(--color-surface);
}
.header-title {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex: 0 0 auto;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
.reset-btn {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: 3px var(--space-2);
  border-radius: var(--radius-sm);
  font-size: 10.5px;
  font-weight: var(--weight-medium);
  color: var(--color-text-muted);
  cursor: pointer;
  border: none;
  background: none;
  transition: color 0.15s, background 0.15s, transform 0.15s;
}
.reset-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}
.reset-btn:active { transform: translateY(0.5px); }
.reset-btn:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
.reset-btn.danger:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}

/* ── Search box (realtime filter by input OR output) ── */
.history-search {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex: 1;
  min-width: 0;
  margin: 0 var(--space-1);
  padding: 5px 9px;
  border-radius: var(--radius-md);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  color: var(--color-text-muted);
  transition: border-color 0.15s, background 0.15s, box-shadow 0.15s;
}
.history-search:focus-within {
  border-color: var(--color-accent-border);
  background: var(--color-surface-hover);
  box-shadow: 0 0 0 2px var(--color-accent-bg);
}
.history-search.has-query {
  color: var(--color-text-secondary);
}
.search-icon {
  flex-shrink: 0;
  opacity: 0.7;
}
.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  outline: none;
  padding: 0;
  font-family: inherit;
  font-size: var(--text-base);
  color: var(--color-text);
}
.search-input::placeholder {
  color: var(--color-text-placeholder);
}
.search-clear {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: var(--radius-xs);
  border: none;
  background: none;
  cursor: pointer;
  color: var(--color-text-muted);
  flex-shrink: 0;
  transition: background 0.12s, color 0.12s;
}
.search-clear:hover {
  color: var(--color-text);
  background: var(--color-border);
}
.search-clear:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }

/* Highlighted matched keyword in search results */
mark.search-hl {
  background: var(--color-accent-bg);
  color: var(--color-accent);
  border-radius: 2px;
  padding: 0 1px;
  font-weight: var(--weight-semibold);
}
/* Hide the title text on very narrow windows so search stays usable */
@media (max-width: 360px) {
  .header-title { display: none; }
}

/* Body */
.history-body {
  flex: 1;
  overflow-y: auto;
  padding: 10px var(--space-6) var(--space-4);
}
.history-body::-webkit-scrollbar { width: 3px; }
.history-body::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: var(--radius-xs); }

/* Empty */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  padding: 40px var(--space-5);
  color: var(--color-text-secondary);
  text-align: center;
  font-size: var(--text-base);
  line-height: 1.5;
}
.empty-icon { opacity: 0.4; }

/* List items */
.history-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.history-item {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-2) 10px;
  border-radius: var(--radius-md);
  border: 1px solid transparent;
  background: transparent;
  transition: background 0.12s, border-color 0.12s;
}
.history-item:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-border);
}
.history-item.remove-pending {
  background: var(--color-danger-bg);
  border-color: var(--color-border);
  justify-content: space-between;
}
.history-item-main {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 9px;
  cursor: pointer;
  text-align: left;
  width: 100%;
  background: none;
  border: none;
  padding: 0;
  min-width: 0;
}
.history-item-main:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 2px; border-radius: var(--radius-sm); }
.history-item-text {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
  flex: 1;
}
.mode-indicator {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  color: var(--color-text-secondary);
}
.history-item-input {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: var(--text-base);
  font-weight: var(--weight-medium);
  color: var(--color-text);
  line-height: 1.3;
}
.history-item-input span {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.history-item-input .input-icon {
  flex-shrink: 0;
  color: var(--color-accent);
  opacity: 0.7;
}
.history-item-output {
  display: flex;
  align-items: flex-start;
  gap: 5px;
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
  line-height: 1.3;
  min-width: 0;
}
.history-item-output span {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.model-badge {
  flex-shrink: 0;
  font-size: 9px;
  font-weight: var(--weight-semibold);
  letter-spacing: 0.02em;
  color: var(--color-text-muted);
  background: var(--color-surface-hover);
  padding: 0 5px;
  border-radius: var(--radius-xs);
  line-height: 16px;
  max-width: 112px;
  white-space: nowrap;
}
.usage-badge {
  color: var(--color-accent-text);
  background: var(--color-accent-bg);
  font-variant-numeric: tabular-nums;
}
.preset-badge {
  flex-shrink: 0;
  font-size: 9px;
  font-weight: var(--weight-semibold);
  letter-spacing: 0.02em;
  color: var(--color-text-muted);
  background: var(--color-surface-hover);
  padding: 0 5px;
  border-radius: var(--radius-xs);
  line-height: 16px;
  max-width: 96px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.history-item-output .output-icon {
  flex-shrink: 0;
  color: var(--color-text-muted);
  opacity: 0.5;
  margin-top: 1px;
}
.history-item-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.12s;
  flex-shrink: 0;
}
.history-item:hover .history-item-actions,
.history-item:focus-within .history-item-actions {
  opacity: 1;
}
.remove-warning-text {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  letter-spacing: 0.01em;
  color: var(--color-danger);
}
.remove-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}
.history-footer {
  margin-top: var(--space-2);
  padding-top: 10px;
  border-top: 1px solid var(--color-surface);
  text-align: center;
  font-size: 10.5px;
  color: var(--color-text-muted);
  letter-spacing: 0.01em;
}

/* ── Searched tag (Globe icon) + sources overlay ── */
.searched-tag {
  display: inline-flex; align-items: center; justify-content: center;
  flex-shrink: 0;
  width: 16px; height: 16px; border-radius: var(--radius-xs);
  background: var(--color-surface-hover); border: none;
  color: var(--color-accent); cursor: pointer;
  transition: background 0.12s, color 0.12s;
}
.searched-tag:hover { background: var(--color-accent-bg); color: var(--color-accent); }
.searched-tag:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }

/* ── Attachment tags (skills_lite multimodal entries) ── */
.history-item-attachments {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 2px;
}
.attachment-tag {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  padding: 0 5px;
  line-height: 16px;
  border-radius: var(--radius-xs);
  background: var(--color-surface-hover);
  color: var(--color-text-muted);
  font-size: 9px;
}
.attachment-icon { flex-shrink: 0; }
.attachment-name {
  min-width: 0;
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.attachment-size { flex-shrink: 0; opacity: 0.7; }
.attachment-download {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: color 0.12s;
}
.attachment-download:hover { color: var(--color-text); }
.attachment-download:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }

/* Hover thumbnail popover for image attachments. Default pops above the tag;
   .pop-below (tag in the window's upper half) flips it below. */
.attachment-thumb-pop {
  display: none;
  position: absolute;
  bottom: calc(100% + 6px);
  left: 0;
  z-index: 20;
  padding: 3px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
}
.attachment-tag.pop-below .attachment-thumb-pop {
  bottom: auto;
  top: calc(100% + 6px);
}
.attachment-tag:hover .attachment-thumb-pop { display: block; }
.attachment-thumb-pop img {
  display: block;
  max-width: 128px;
  max-height: 128px;
  object-fit: cover;
  border-radius: var(--radius-xs);
}

/* ── Edited tag ── */
.edited-tag {
  flex-shrink: 0;
  font-size: 9px;
  font-weight: var(--weight-semibold);
  letter-spacing: 0.02em;
  color: var(--color-text-muted);
  background: var(--color-surface-hover);
  padding: 0 5px;
  border-radius: var(--radius-xs);
  line-height: 16px;
  white-space: nowrap;
}

/* Source-item list (used inside history-body when viewing sources) */
.source-item {
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-2) 10px; border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  text-decoration: none; cursor: pointer;
  transition: background 0.12s, border-color 0.12s;
}
.source-item:hover { background: var(--color-surface-hover); border-color: var(--color-border-hover); }
.source-item:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
.source-favicon { flex-shrink: 0; display: inline-flex; align-items: center; color: var(--color-text-muted); }
.source-meta { display: flex; flex-direction: column; gap: 1px; min-width: 0; flex: 1; }
.source-title {
  font-size: var(--text-base); font-weight: var(--weight-semibold); color: var(--color-text);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.source-domain {
  font-size: var(--text-xs); color: var(--color-text-muted);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.source-external { flex-shrink: 0; color: var(--color-text-muted); }
.sources-empty { font-size: var(--text-sm); color: var(--color-text-muted); padding: var(--space-4) 0; text-align: center; }

/* Toggle button */
.reset-btn.toggle-on {
  color: var(--color-accent);
}
.reset-btn.toggle-on:hover {
  color: var(--color-accent);
}
</style>
