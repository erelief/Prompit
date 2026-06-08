<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getLangName } from "../constants/languages";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  appConfig,
  loadDictionary,
  saveDictionary,
  importDictionaryCsv,
  exportDictionaryCsv,
  clearAllDictionaries,
} from "../stores/config";
import type { DictEntry } from "../stores/config";
import { ArrowLeft, Download, Upload, Trash2, Plus, Save } from "@lucide/vue";

const { t } = useI18n();
const entries = ref<DictEntry[]>([]);
const loading = ref(true);
const router = useRouter();
const saveError = ref("");
const dirty = ref(false);

/* ── Window drag ── */
async function handleDrag(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (t.closest("textarea, button, input, select, a")) return;
  await getCurrentWindow().startDragging();
}

/* ── Add entry ── */
function addEntry() {
  entries.value.push({ source: "", target: "" });
  dirty.value = true;
}

/* ── Validate & Save ── */
async function handleSave() {
  saveError.value = "";
  for (let i = 0; i < entries.value.length; i++) {
    const e = entries.value[i];
    const hasSource = e.source.trim() !== "";
    const hasTarget = e.target.trim() !== "";
    if (!hasSource && !hasTarget) {
      saveError.value = t('dictionary.rowIsEmpty', { n: i + 1 });
      return;
    }
    if (!hasSource) {
      saveError.value = t('dictionary.sourceRequired', { n: i + 1 });
      return;
    }
    if (!hasTarget) {
      saveError.value = t('dictionary.translationRequired', { n: i + 1 });
      return;
    }
  }
  const valid = entries.value
    .filter((e) => e.source.trim() !== "" && e.target.trim() !== "")
    .map((e) => ({ source: e.source.trim(), target: e.target.trim() }));
  try {
    await saveDictionary(appConfig.target_lang, valid);
    dirty.value = false;
  } catch (err) {
    saveError.value = "Failed to save dictionary.";
    console.error("Failed to save dictionary:", err);
  }
}

/* ── Remove entry ── */
function removeEntry(index: number) {
  entries.value.splice(index, 1);
  dirty.value = true;
  saveError.value = "";
}

/* ── Import flow ── */
const showImportMode = ref(false);
const pendingImportPath = ref("");
const showOverwriteWarn = ref(false);
const importMessage = ref("");

function cancelImportMode() {
  showImportMode.value = false;
  showOverwriteWarn.value = false;
  pendingImportPath.value = "";
}

async function requestImport() {
  const filePath = await open({
    multiple: false,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (!filePath) return;
  pendingImportPath.value = filePath as string;
  const currentEntries = entries.value;
  const isEmpty = currentEntries.length === 0;
  if (isEmpty) {
    await executeImport("add");
  } else {
    showImportMode.value = true;
  }
}

async function chooseImportMode(mode: "add" | "overwrite") {
  if (mode === "overwrite") {
    showOverwriteWarn.value = true;
    return;
  }
  showImportMode.value = false;
  await executeImport("add");
}

async function confirmOverwrite() {
  showOverwriteWarn.value = false;
  showImportMode.value = false;
  await executeImport("overwrite");
}

async function executeImport(mode: "add" | "overwrite") {
  try {
    const result = await importDictionaryCsv(pendingImportPath.value, mode);
    entries.value = await loadDictionary(appConfig.target_lang);
    dirty.value = false;
    saveError.value = "";
    const langs = result.languages_affected.join(", ");
    importMessage.value = t('dictionary.imported', { n: result.imported, langs });
    setTimeout(() => { importMessage.value = ""; }, 4000);
  } catch (err) {
    console.error("Failed to import dictionary:", err);
  } finally {
    pendingImportPath.value = "";
  }
}

async function handleExport() {
  const now = new Date();
  const ts = [
    String(now.getFullYear()).slice(2),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
    String(now.getHours()).padStart(2, "0"),
    String(now.getMinutes()).padStart(2, "0"),
  ].join("");
  const filePath = await save({
    defaultPath: `Prompit_Translation_UD-${ts}.csv`,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (!filePath) return;
  try {
    await exportDictionaryCsv(filePath);
  } catch (err) {
    console.error("Failed to export dictionary:", err);
  }
}

/* ── Clear flow ── */
const pendingClear = ref<"current" | "all" | null>(null);

function requestClearCurrent() {
  pendingClear.value = "current";
}

function requestClearAll() {
  pendingClear.value = "all";
}

function cancelClear() {
  pendingClear.value = null;
}

async function confirmClear() {
  if (pendingClear.value === "current") {
    entries.value = [];
    await handleSave();
  } else if (pendingClear.value === "all") {
    await clearAllDictionaries();
    entries.value = [];
    dirty.value = false;
  }
  pendingClear.value = null;
}

/* ── Lifecycle ── */
onMounted(async () => {
  try {
    entries.value = await loadDictionary(appConfig.target_lang);
  } catch {
    entries.value = [];
  }
  loading.value = false;
});
</script>

<template>
  <div class="dict-root" @mousedown="handleDrag">
    <!-- Header -->
    <div class="dict-header">
      <button class="back-btn" @click="router.push('/settings?tab=translation')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('dictionary.userDictionary') }}</span>
      <button class="pill-btn micro" @click="requestImport">
        <Download :size="12" />
        <span>{{ t('dictionary.import') }}</span>
      </button>
      <button class="pill-btn micro" @click="handleExport">
        <Upload :size="12" />
        <span>{{ t('dictionary.export') }}</span>
      </button>
    </div>

    <!-- Language label + Add Entry -->
    <div class="dict-lang-row">
      <span class="dict-lang">{{ t('dictionary.target') }}: {{ getLangName(appConfig.target_lang) }}</span>
      <button class="pill-btn add-pill" @click="addEntry">
        <Plus :size="12" :stroke-width="2" />
        <span>{{ t('dictionary.addEntry') }}</span>
      </button>
    </div>

    <!-- Table -->
    <div class="dict-table-wrap">
      <div class="dict-table settings-scrollbar">
        <!-- Sticky header row -->
        <div class="dict-row dict-header-row">
          <div class="dict-col col-source">{{ t('dictionary.source') }}</div>
          <div class="dict-col col-trans">{{ t('dictionary.translation') }}</div>
          <div class="dict-col col-action"></div>
        </div>

        <!-- Data rows -->
        <div v-for="(entry, i) in entries" :key="i" class="dict-row">
          <div class="dict-col col-source">
            <input
              class="dict-input"
              v-model="entry.source"
              placeholder="..."
              @input="dirty = true"
            />
          </div>
          <div class="dict-col col-trans">
            <input
              class="dict-input"
              v-model="entry.target"
              placeholder="..."
              @input="dirty = true"
            />
          </div>
          <div class="dict-col col-action">
            <button
              class="mini-btn warn"
              @click="removeEntry(i)"
            >
              <Trash2 :size="13" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="dict-footer">
      <span class="footer-count">{{ t('dictionary.entries') }}: {{ entries.length }}</span>
      <span v-if="importMessage" class="footer-import-msg">{{ importMessage }}</span>
      <span v-if="saveError" class="footer-error">{{ saveError }}</span>

      <!-- Clear buttons -->
      <button class="pill-btn micro clear-btn" @click="requestClearCurrent" :disabled="entries.length === 0">
        <Trash2 :size="11" />
        <span>{{ t('dictionary.clearCurrent') }}</span>
      </button>
      <button class="pill-btn micro clear-btn" @click="requestClearAll">
        <Trash2 :size="11" />
        <span>{{ t('dictionary.clearAll') }}</span>
      </button>

      <button class="pill-btn save-btn" :disabled="!dirty" @click="handleSave">
        <Save :size="12" />
        <span>{{ t('common.save') }}</span>
      </button>
    </div>

    <!-- Import mode dialog -->
    <Teleport to="body">
      <Transition name="drop">
        <div v-if="showImportMode" class="modal-overlay" @click.self="cancelImportMode">
          <div class="modal-card">
            <div class="modal-title">{{ t('dictionary.importModeTitle') }}</div>
            <div class="modal-hint">{{ t('dictionary.importModeHint') }}</div>
            <template v-if="!showOverwriteWarn">
              <div class="modal-actions modal-actions--stacked">
                <button class="modal-choice" @click="chooseImportMode('add')">
                  <span class="choice-label">{{ t('dictionary.addToExisting') }}</span>
                  <span class="choice-desc">{{ t('common.cancel') }}</span>
                </button>
                <button class="modal-choice modal-choice--danger" @click="chooseImportMode('overwrite')">
                  <span class="choice-label">{{ t('dictionary.overwritePerLang') }}</span>
                  <span class="choice-desc">{{ t('dictionary.overwriteWarning') }}</span>
                </button>
              </div>
            </template>
            <template v-else>
              <div class="modal-warn-row">
                <span class="remove-warning-text">{{ t('dictionary.overwriteWarning') }}</span>
              </div>
              <div class="modal-actions">
                <button class="pill-btn modal-btn" @click="cancelImportMode">
                  {{ t('common.cancel') }}
                </button>
                <button class="pill-btn modal-btn danger-active" @click="confirmOverwrite">
                  {{ t('common.confirm') }}
                </button>
              </div>
            </template>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Clear confirm dialog -->
    <Teleport to="body">
      <Transition name="drop">
        <div v-if="pendingClear" class="modal-overlay" @click.self="cancelClear">
          <div class="modal-card">
            <div class="modal-title">
              {{ pendingClear === 'current'
                ? t('dictionary.clearCurrentConfirm', { lang: getLangName(appConfig.target_lang) })
                : t('dictionary.clearAllConfirm') }}
            </div>
            <div class="modal-warn-row">
              <span class="remove-warning-text">{{ t('dictionary.overwriteWarning') }}</span>
            </div>
            <div class="modal-actions">
              <button class="pill-btn modal-btn" @click="cancelClear">
                {{ t('common.cancel') }}
              </button>
              <button class="pill-btn modal-btn danger-active" @click="confirmClear">
                {{ t('common.confirm') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   Design tokens & base
   ══════════════════════════════════════ */
.dict-root {
  height: 100dvh;
  display: flex;
  flex-direction: column;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: 11px;
}

/* ── Header ── */
.dict-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 24px 12px;
  border-bottom: 1px solid var(--color-surface);
  flex-shrink: 0;
}
.header-title {
  flex: 1;
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--color-text);
  line-height: 1.2;
}
.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 9px;
  color: var(--color-text-muted);
  transition: 0.15s;
}
.back-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}

/* ── Pill buttons ── */
.pill-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 11px;
  border-radius: 7px;
  font-size: 10.5px;
  font-weight: 550;
  cursor: pointer;
  border: none;
  background: none;
  transition: 0.15s;
}
.add-pill {
  color: var(--color-accent-text);
}
.add-pill:hover {
  color: var(--color-accent);
  background: var(--color-accent-bg);
}
.micro {
  color: var(--color-text-muted);
  padding: 3px 8px;
}
.micro:hover:not(:disabled) {
  color: var(--color-text-secondary);
  background: var(--color-surface-hover);
}

/* ── Language row ── */
.dict-lang-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 24px 6px;
  flex-shrink: 0;
}
.dict-lang {
  flex: 1;
  font-size: 11.5px;
  font-weight: 550;
  color: var(--color-text-muted);
}

/* ── Table ── */
.dict-table-wrap {
  flex: 1;
  overflow: hidden;
  padding: 0 24px;
  min-height: 0;
}
.dict-table {
  height: 100%;
  overflow-y: auto;
  border: 1px solid var(--color-surface);
  border-radius: 9px;
}

/* scrollbar */
.settings-scrollbar::-webkit-scrollbar {
  width: 3px;
}
.settings-scrollbar::-webkit-scrollbar-track {
  margin: 10px 0;
}
.settings-scrollbar::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar);
  border-radius: 3px;
}

/* ── Rows ── */
.dict-row {
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid var(--color-surface);
}
.dict-row:last-child {
  border-bottom: none;
}

/* ── Header row ── */
.dict-header-row {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--color-surface);
  backdrop-filter: blur(6px);
}
.dict-header-row .dict-col {
  font-size: 10.5px;
  font-weight: 650;
  color: var(--color-text-muted);
  letter-spacing: 0.02em;
  padding: 8px 12px;
}

/* ── Columns ── */
.dict-col {
  display: flex;
  align-items: center;
  padding: 0 12px;
  min-height: 36px;
}
.col-source {
  flex: 1;
  border-right: 1px solid var(--color-surface);
}
.col-trans {
  flex: 1;
}
.col-action {
  width: 40px;
  justify-content: center;
  flex-shrink: 0;
}

/* ── Inputs ── */
.dict-input {
  width: 100%;
  background: none;
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--color-text);
  font-size: 12px;
  padding: 6px 6px;
  outline: none;
  transition: border-color 0.18s;
  font-family: inherit;
}
.dict-input:focus {
  border-color: var(--color-accent-border);
}
.dict-input::placeholder {
  color: var(--color-scrollbar);
}

/* ── Mini button (delete) ── */
.mini-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 27px;
  height: 27px;
  border-radius: 7px;
  color: var(--color-text-muted);
  cursor: pointer;
  border: none;
  background: none;
  transition: 0.12s;
}
.mini-btn.warn:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}

/* ── Footer ── */
.dict-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 24px 14px;
  font-size: 10.5px;
  font-weight: 550;
  color: var(--color-text-muted);
  flex-shrink: 0;
}
.footer-count {
  flex: 1;
}
.footer-error {
  color: var(--color-danger);
  font-weight: 500;
  flex: 1;
  text-align: right;
}
.save-btn {
  color: var(--color-text-secondary);
  background: var(--color-accent-bg);
  padding: 4px 12px;
}
.save-btn:hover:not(:disabled) {
  color: var(--color-text);
  background: var(--color-accent-bg);
}
.save-btn:disabled {
  opacity: 0.35;
  cursor: default;
}

/* ── Footer extras ── */
.footer-import-msg {
  flex: 1;
  color: var(--color-accent);
  font-weight: 500;
}
.clear-btn {
  color: var(--color-text-muted);
  margin-left: auto;
}
.clear-btn:hover:not(:disabled) {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.clear-btn:disabled {
  opacity: 0.35;
  cursor: default;
}

/* ── Modal overlay ── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 999;
  backdrop-filter: blur(2px);
}
.modal-card {
  background: var(--color-bg);
  border: 1px solid var(--color-surface);
  border-radius: 11px;
  padding: 20px 24px;
  min-width: 280px;
  max-width: 360px;
  box-shadow:
    0 4px 16px rgba(0,0,0,.12),
    0 8px 32px rgba(0,0,0,.14),
    0 2px 6px rgba(0,0,0,.08);
}
.modal-title {
  font-size: 13px;
  font-weight: 700;
  letter-spacing: -0.01em;
  margin-bottom: 4px;
  color: var(--color-text);
  line-height: 1.3;
}
.modal-hint {
  font-size: 11.5px;
  color: var(--color-text-muted);
  margin-bottom: 18px;
  line-height: 1.45;
}
.modal-warn-row {
  margin-bottom: 14px;
  padding: 10px 14px;
  border-radius: 7px;
  background: var(--color-danger-bg);
}
.modal-warn-row .remove-warning-text {
  display: block;
  font-size: 10px;
  font-weight: 550;
  letter-spacing: .01em;
  color: var(--color-danger);
}
.modal-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
.modal-actions--stacked {
  flex-direction: column;
  gap: 6px;
}
.modal-choice {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 10px 14px;
  border-radius: 8px;
  border: 1px solid var(--color-surface);
  background: var(--color-surface);
  cursor: pointer;
  transition: border-color .15s, background .15s;
  text-align: left;
}
.modal-choice:hover {
  border-color: var(--color-accent-border);
  background: var(--color-bg);
}
.modal-choice--danger {
  border-color: rgba(220,38,38,.25);
  background: rgba(220,38,38,.04);
}
.modal-choice--danger:hover {
  border-color: rgba(220,38,38,.45);
  background: rgba(220,38,38,.08);
}
.choice-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text);
}
.choice-desc {
  font-size: 10px;
  color: var(--color-text-muted);
  margin-left: auto;
  padding-left: 10px;
}
.modal-btn {
  padding: 6px 16px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: .01em;
  color: var(--color-text-secondary);
  background: var(--color-surface);
  border-radius: 7px;
  transition: color .15s, background .15s;
}
.modal-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}
.warn-btn {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.warn-btn:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
  filter: brightness(.92);
}
.danger-active {
  color: var(--color-danger);
  background: var(--color-danger-bg);
  animation: danger-pulse .8s ease-in-out infinite alternate;
}
@keyframes danger-pulse {
  to { background: var(--color-danger-bg); filter: brightness(.88); }
}

/* ── Modal transition ── */
.drop-enter-active {
  transition: opacity .18s ease, transform .2s cubic-bezier(.16,1,.3,1);
}
.drop-leave-active {
  transition: opacity .12s ease;
}
.drop-enter-from {
  opacity: 0;
  transform: scale(.96) translateY(4px);
}
.drop-leave-to {
  opacity: 0;
}
</style>
